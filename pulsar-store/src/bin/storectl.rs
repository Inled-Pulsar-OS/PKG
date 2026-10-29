//! Command line front end for the store core. Builds without GTK.
//!
//! ```text
//! pulsar-storectl [--json] refresh [--force]
//! pulsar-storectl [--json] search [QUERY] [--category KEY]
//! pulsar-storectl [--json] info ID
//! pulsar-storectl [--json] installed
//! pulsar-storectl [--json] updates
//! pulsar-storectl [--json] install ID [--edition arch|debian|flatpak]
//! pulsar-storectl [--json] uninstall ID
//! pulsar-storectl [--json] update ID
//! ```
//!
//! Exit codes: 0 success, 1 failure, 2 usage error. With `--json`, long
//! operations print one JSON object per line, ending with `done` or `error`.

use pulsar_store::core::editions::editions;
use pulsar_store::core::error::StoreError;
use pulsar_store::core::i18n;
use pulsar_store::core::install::{Action, Event};
use pulsar_store::core::model::{EditionKey, Item, Phase};
use pulsar_store::core::ports::CatalogProvider;
use pulsar_store::core::search;
use pulsar_store::core::store::Store;
use serde_json::{json, Value};
use std::process::ExitCode;

struct Args {
    json: bool,
    positional: Vec<String>,
    force: bool,
    category: Option<String>,
    edition: Option<String>,
}

fn parse(args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut out = Args {
        json: false,
        positional: Vec::new(),
        force: false,
        category: None,
        edition: None,
    };
    let mut it = args;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--json" => out.json = true,
            "--force" => out.force = true,
            "--category" => out.category = Some(it.next().ok_or("--category needs a value")?),
            "--edition" => out.edition = Some(it.next().ok_or("--edition needs a value")?),
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            _ => out.positional.push(a),
        }
    }
    Ok(out)
}

fn edition_key(s: &str) -> Option<EditionKey> {
    match s {
        "arch" => Some(EditionKey::Arch),
        "debian" => Some(EditionKey::Debian),
        "flatpak" => Some(EditionKey::Flatpak),
        _ => None,
    }
}

fn phase_name(p: Phase) -> &'static str {
    match p {
        Phase::Preparing => "preparing",
        Phase::Downloading => "downloading",
        Phase::Installing => "installing",
        Phase::Finalizing => "finalizing",
    }
}

fn summarize(item: &Item, locales: &[&str]) -> Value {
    json!({
        "id": item.id,
        "type": item.kind,
        "name": item.name.resolve(locales),
        "summary": item.summary.resolve(locales),
        "category": item.category,
        "version": item.version,
    })
}

fn error_json(e: &StoreError) -> Value {
    json!({ "event": "error", "key": e.message_key(), "message": e.to_string() })
}

fn print_list(json: bool, rows: Vec<Value>) {
    if json {
        println!("{}", Value::Array(rows));
        return;
    }
    for r in rows {
        let s = |k: &str| r.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        println!("{}\t{}\t{}", s("id"), s("name"), s("version"));
    }
}

fn find<'a>(items: &'a [Item], id: &str) -> Result<&'a Item, StoreError> {
    items
        .iter()
        .find(|i| i.id == id)
        .ok_or_else(|| StoreError::NotFound(id.to_string()))
}

fn run_action(store: &Store, action: Action, item: Item, json: bool) -> Result<(), StoreError> {
    let handle = store.installer.start(action, item);
    loop {
        match handle.events.recv() {
            Ok(Event::Progress(p)) if json => {
                println!(
                    "{}",
                    json!({ "event": "progress", "phase": phase_name(p.phase), "fraction": p.fraction })
                );
            }
            Ok(Event::Progress(_)) => {}
            Ok(Event::Finished(result)) => return result,
            Err(_) => {
                return Err(StoreError::CommandFailed(
                    "worker stopped unexpectedly".into(),
                ))
            }
        }
    }
}

fn real_main() -> Result<(), (StoreError, bool)> {
    let args = parse(std::env::args().skip(1)).map_err(|m| (StoreError::Unsupported(m), false))?;
    let json = args.json;
    let fail = |e: StoreError| (e, json);
    let Some(cmd) = args.positional.first().map(String::as_str) else {
        return Err((StoreError::Unsupported("missing command".into()), false));
    };

    let store = Store::new();
    let tags = i18n::locales();
    let locales: Vec<&str> = tags.iter().map(String::as_str).collect();

    if cmd == "refresh" {
        let updated = store.catalog.refresh(args.force).map_err(fail)?;
        println!(
            "{}",
            if json {
                json!({ "updated": updated }).to_string()
            } else {
                updated.to_string()
            }
        );
        return Ok(());
    }

    let catalog = store.catalog.load().map_err(fail)?;
    let items = &catalog.packages;
    let id_arg = || {
        args.positional
            .get(1)
            .cloned()
            .ok_or_else(|| (StoreError::Unsupported("missing ID".into()), false))
    };

    match cmd {
        "search" => {
            let query = args.positional.get(1).map_or("", String::as_str);
            let pool = match &args.category {
                Some(c) => search::by_category(items, c, &|i| {
                    store
                        .installer
                        .installed_version(i)
                        .ok()
                        .flatten()
                        .is_some()
                }),
                None => items.iter().collect(),
            };
            let hits: Vec<Item> = pool.into_iter().cloned().collect();
            let found = search::search(&hits, query);
            print_list(json, found.iter().map(|i| summarize(i, &locales)).collect());
        }
        "info" => {
            let item = find(items, &id_arg()?).map_err(fail)?;
            let state = store.installer.state(item).map_err(fail)?;
            let v = json!({ "item": item, "state": state, "editions": editions(item) });
            println!(
                "{}",
                if json {
                    v.to_string()
                } else {
                    serde_json::to_string_pretty(&v).unwrap_or_default()
                }
            );
        }
        "installed" => {
            let rows = items
                .iter()
                .filter(|i| {
                    store
                        .installer
                        .installed_version(i)
                        .ok()
                        .flatten()
                        .is_some()
                })
                .map(|i| summarize(i, &locales))
                .collect();
            print_list(json, rows);
        }
        "updates" => {
            let rows = store
                .installer
                .updates(items)
                .into_iter()
                .map(|u| json!({ "id": u.item.id, "name": u.item.name.resolve(&locales), "installed": u.installed, "available": u.available }))
                .collect::<Vec<_>>();
            if json {
                println!("{}", Value::Array(rows));
            } else {
                for r in rows {
                    println!(
                        "{}\t{}\t{}",
                        r["id"].as_str().unwrap_or(""),
                        r["installed"].as_str().unwrap_or(""),
                        r["available"].as_str().unwrap_or("")
                    );
                }
            }
        }
        "install" | "uninstall" | "update" => {
            let item = find(items, &id_arg()?).map_err(fail)?.clone();
            let action = match cmd {
                "install" => {
                    let edition = match args.edition.as_deref() {
                        Some(s) => Some(edition_key(s).ok_or_else(|| {
                            (
                                StoreError::Unsupported(format!("unknown edition {s}")),
                                false,
                            )
                        })?),
                        None => None,
                    };
                    Action::Install(edition)
                }
                "uninstall" => Action::Uninstall,
                _ => Action::Update,
            };
            run_action(&store, action, item, json).map_err(fail)?;
            if json {
                println!("{}", json!({ "event": "done", "ok": true }));
            }
        }
        other => {
            return Err((
                StoreError::Unsupported(format!("unknown command {other}")),
                false,
            ))
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    // Die quietly when the reader closes the pipe (`... | head`).
    // SAFETY: restoring the default SIGPIPE action at startup, before any threads exist.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err((e, json)) => {
            if json {
                println!("{}", error_json(&e));
            } else {
                eprintln!("pulsar-storectl: {e}");
            }
            if matches!(e, StoreError::Unsupported(_)) && !json {
                ExitCode::from(2)
            } else {
                ExitCode::from(1)
            }
        }
    }
}
