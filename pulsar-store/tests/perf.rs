//! Checks the performance budget in docs/performance-budget.md.
//! Timings only mean something in release mode:
//!
//!     cargo test --release --test perf -- --ignored --nocapture

use pulsar_store::core::catalog::parse_catalog;
use pulsar_store::core::search::SearchIndex;
use std::time::Instant;

const ITEMS: usize = 5000;
const RSS_BUDGET_MB: f64 = 60.0;
const QUERIES: [&str; 10] = [
    "gimp",
    "edit",
    "image",
    "browser",
    "code",
    "game",
    "music",
    "zzzz-no-match",
    "a",
    "pulsar",
];

/// The real catalog, repeated with unique ids until it has `n` items.
fn big_catalog_json(n: usize) -> Vec<u8> {
    let base: serde_json::Value =
        serde_json::from_str(include_str!("../assets/catalog/catalog.json")).unwrap();
    let packages = base["packages"].as_array().unwrap();
    let items: Vec<serde_json::Value> = (0..n)
        .map(|i| {
            let mut p = packages[i % packages.len()].clone();
            p["id"] = format!("{}.{i}", p["id"].as_str().unwrap()).into();
            p
        })
        .collect();
    serde_json::to_vec(&serde_json::json!({ "packages": items })).unwrap()
}

/// Peak memory in MB of `pulsar-storectl search` loading `json` as its catalog.
/// A separate process gives a number free of this test's own allocations.
fn storectl_peak_rss_mb(json: &[u8]) -> f64 {
    let cache = tempfile::tempdir().unwrap();
    let dir = cache.path().join("pulsar-store");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("catalog.json"), json).unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_pulsar-storectl"))
        .args(["--json", "search", "zzzz-no-match"])
        .env("XDG_CACHE_HOME", cache.path())
        .output()
        .unwrap()
        .status;
    assert!(status.success());
    // SAFETY: getrusage only writes into the struct we pass.
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::getrusage(libc::RUSAGE_CHILDREN, &mut usage) },
        0
    );
    usage.ru_maxrss as f64 / 1024.0 // kilobytes on Linux
}

#[test]
#[ignore = "timing test; run with --release"]
fn five_thousand_items_meet_the_budget() {
    if cfg!(debug_assertions) {
        panic!("run this test with --release");
    }
    let json = big_catalog_json(ITEMS);

    let mut loads = Vec::new();
    let mut catalog = None;
    for _ in 0..5 {
        let t = Instant::now();
        catalog = Some(parse_catalog(&json).unwrap());
        loads.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    loads.sort_by(f64::total_cmp);
    let load_median = loads[loads.len() / 2];
    let catalog = catalog.unwrap();
    assert_eq!(catalog.packages.len(), ITEMS);

    let index = SearchIndex::build(&catalog.packages);
    let mut latencies = Vec::new();
    for _ in 0..5 {
        for q in QUERIES {
            let t = Instant::now();
            let found = index.search(&catalog.packages, q);
            std::hint::black_box(found.len());
            latencies.push(t.elapsed().as_secs_f64() * 1000.0);
        }
    }
    latencies.sort_by(f64::total_cmp);
    let p95 = latencies[(latencies.len() as f64 * 0.95) as usize - 1];
    let rss = storectl_peak_rss_mb(&json);

    println!("catalog load median: {load_median:.2} ms (budget 25)");
    println!("search p95: {p95:.3} ms (budget 5)");
    println!("storectl peak RSS: {rss:.1} MB (budget {RSS_BUDGET_MB})");

    assert!(
        load_median <= 25.0,
        "catalog load {load_median:.2} ms over budget"
    );
    assert!(p95 <= 5.0, "search p95 {p95:.3} ms over budget");
    assert!(rss <= RSS_BUDGET_MB, "peak RSS {rss:.1} MB over budget");
}
