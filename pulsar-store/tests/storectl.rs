//! Runs the real `pulsar-storectl` binary against the embedded catalog, offline.

use std::process::Command;

fn run(args: &[&str]) -> (i32, String, String) {
    let cache = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulsar-storectl"))
        .args(args)
        .env("XDG_CACHE_HOME", cache.path())
        .env("LANG", "en_US.UTF-8")
        .env_remove("LANGUAGE")
        .env_remove("LC_ALL")
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn search_returns_json_array() {
    let (code, out, _) = run(&["--json", "search", "telegram"]);
    assert_eq!(code, 0);
    let rows: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert!(rows.iter().any(|r| r["id"] == "sayri-gateway-telegram"));
}

#[test]
fn info_includes_item_state_and_editions() {
    let (code, out, _) = run(&["--json", "info", "sayri-gateway-telegram"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["item"]["id"], "sayri-gateway-telegram");
    assert!(v["editions"].is_array());
    assert!(v.get("state").is_some());
}

#[test]
fn unknown_id_is_a_json_error_with_exit_1() {
    let (code, out, _) = run(&["--json", "install", "no.such.item"]);
    assert_eq!(code, 1);
    let v: serde_json::Value = serde_json::from_str(out.lines().last().unwrap()).unwrap();
    assert_eq!(v["event"], "error");
    assert_eq!(v["key"], "error.not_found");
}

#[test]
fn usage_errors_exit_2() {
    assert_eq!(run(&["bogus"]).0, 2);
    assert_eq!(run(&[]).0, 2);
    assert_eq!(run(&["--nope", "search"]).0, 2);
}

#[test]
fn category_filter_applies() {
    let (_, out, _) = run(&["--json", "search", "--category", "sayri"]);
    let rows: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert!(!rows.is_empty());
    assert!(rows
        .iter()
        .all(|r| r["type"].as_str().unwrap().starts_with("sayri")));
}
