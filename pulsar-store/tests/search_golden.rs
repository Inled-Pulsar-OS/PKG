//! The Rust search must match the Python store's results, in the same order.

use pulsar_store::core::catalog::parse_catalog;
use pulsar_store::core::search::search;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[test]
fn search_matches_python_golden_results() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let catalog =
        parse_catalog(&fs::read(root.join("assets/catalog/catalog.json")).unwrap()).unwrap();
    let golden: BTreeMap<String, Vec<String>> = serde_json::from_str(
        &fs::read_to_string(root.join("tests/fixtures/search_golden.json")).unwrap(),
    )
    .unwrap();
    assert!(golden.len() > 10);
    for (query, expected) in golden {
        let got: Vec<&str> = search(&catalog.packages, &query)
            .iter()
            .map(|i| i.id.as_str())
            .collect();
        assert_eq!(got, expected, "query {query:?}");
    }
}
