//! dmenu mode logic: argument parsing, item reading, ranking, exit-code decision.
use std::io::BufRead;

use super::ranker::{self, Item};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Args {
    pub prompt: Option<String>,
    pub placeholder: Option<String>,
}

impl Args {
    /// True when argv (including argv[0]) requests dmenu mode.
    pub fn wants_dmenu(argv: &[String]) -> bool {
        argv.iter().skip(1).any(|a| a == "--dmenu")
    }

    /// Parse `--dmenu [--prompt T] [--placeholder T]` (also `--prompt=T`). argv[0] is skipped.
    pub fn parse(argv: &[String]) -> Result<Args, String> {
        let mut out = Args::default();
        let mut seen = false;
        let mut it = argv.iter().skip(1);
        while let Some(a) = it.next() {
            let (key, inline) = match a.split_once('=') {
                Some((k, v)) if k.starts_with("--") => (k, Some(v.to_string())),
                _ => (a.as_str(), None),
            };
            match key {
                "--dmenu" => seen = true,
                "--prompt" | "--placeholder" => {
                    let v = inline.or_else(|| it.next().cloned()).ok_or(format!("{key} needs a value"))?;
                    if key == "--prompt" {
                        out.prompt = Some(v);
                    } else {
                        out.placeholder = Some(v);
                    }
                }
                other => return Err(format!("unknown argument: {other}")),
            }
        }
        if seen { Ok(out) } else { Err("--dmenu not given".into()) }
    }
}

/// Newline-separated items; strips `\r`, drops empty lines, keeps order and duplicates.
pub fn read_items<R: BufRead>(reader: R) -> Vec<String> {
    reader
        .lines()
        .map_while(Result::ok)
        .map(|l| l.trim_end_matches('\r').to_string())
        .filter(|l| !l.trim().is_empty())
        .collect()
}

/// Fuzzy-filter items. Empty query returns all items in input order.
pub fn filter(query: &str, items: &[String]) -> Vec<String> {
    if query.trim().is_empty() {
        return items.to_vec();
    }
    let ranked: Vec<Item> = items
        .iter()
        .enumerate()
        .map(|(i, t)| Item { id: format!("{i:08}"), title: t.clone(), ..Default::default() })
        .collect();
    ranker::rank(query, &ranked).into_iter().map(|(i, _)| i.title.clone()).collect()
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Selected(String),
    Cancelled,
}

impl Outcome {
    pub fn exit_code(&self) -> i32 {
        match self {
            Outcome::Selected(_) => 0,
            Outcome::Cancelled => 1,
        }
    }

    /// Text to print on stdout (with trailing newline); `None` when cancelled.
    pub fn stdout(&self) -> Option<String> {
        match self {
            Outcome::Selected(s) => Some(format!("{s}\n")),
            Outcome::Cancelled => None,
        }
    }
}

/// Non-interactive check used before the GTK window exists: no items means nothing to pick.
pub fn run_headless_check(items: &[String]) -> Option<Outcome> {
    items.is_empty().then_some(Outcome::Cancelled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_flags() {
        let a = Args::parse(&v(&["x", "--dmenu", "--prompt", "Pick", "--placeholder=Type"])).unwrap();
        assert_eq!(a, Args { prompt: Some("Pick".into()), placeholder: Some("Type".into()) });
        assert_eq!(Args::parse(&v(&["x", "--dmenu"])).unwrap(), Args::default());
    }

    #[test]
    fn parse_errors() {
        assert!(Args::parse(&v(&["x", "--dmenu", "--prompt"])).is_err());
        assert!(Args::parse(&v(&["x", "--dmenu", "--bogus"])).is_err());
        assert!(Args::parse(&v(&["x", "--prompt", "a"])).is_err());
    }

    #[test]
    fn wants_dmenu_detect() {
        assert!(Args::wants_dmenu(&v(&["x", "--dmenu"])));
        assert!(!Args::wants_dmenu(&v(&["x", "--hidden"])));
        assert!(!Args::wants_dmenu(&v(&["--dmenu"])));
    }

    #[test]
    fn reads_items() {
        let items = read_items("a\r\n\nb\n  \nc".as_bytes());
        assert_eq!(items, v(&["a", "b", "c"]));
    }

    #[test]
    fn filter_empty_keeps_order_and_fuzzy_ranks() {
        let items = v(&["zeta", "Firefox", "Fast Fonts"]);
        assert_eq!(filter("", &items), items);
        assert_eq!(filter("fire", &items), v(&["Firefox"]));
        assert_eq!(filter("zz", &items), Vec::<String>::new());
        assert!(filter("qqq", &items).is_empty());
    }

    #[test]
    fn outcomes() {
        let s = Outcome::Selected("b".into());
        assert_eq!((s.exit_code(), s.stdout()), (0, Some("b\n".into())));
        assert_eq!((Outcome::Cancelled.exit_code(), Outcome::Cancelled.stdout()), (1, None));
        assert_eq!(run_headless_check(&[]), Some(Outcome::Cancelled));
        assert_eq!(run_headless_check(&v(&["a"])), None);
    }
}
