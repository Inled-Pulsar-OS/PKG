//! Runs the catalog fixtures against the v1 JSON Schema and the Rust model.

use pulsar_store::core::model::{Catalog, SourceKind};
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/schema")
}

fn json_files(dir: &str) -> Vec<(String, serde_json::Value)> {
    let mut out: Vec<_> = fs::read_dir(root().join("fixtures").join(dir))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (
                name,
                serde_json::from_str(&fs::read_to_string(&p).unwrap()).unwrap(),
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(!out.is_empty(), "no fixtures in {dir}");
    out
}

fn validator() -> jsonschema::Validator {
    let schema = fs::read_to_string(root().join("catalog.v1.schema.json")).unwrap();
    jsonschema::validator_for(&serde_json::from_str(&schema).unwrap()).unwrap()
}

#[test]
fn valid_fixtures_validate_and_parse() {
    let v = validator();
    for (name, doc) in json_files("valid") {
        assert!(v.is_valid(&doc), "{name} should validate");
        let catalog: Catalog =
            serde_json::from_value(doc).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(!catalog.packages.is_empty(), "{name} parsed no packages");
    }
}

#[test]
fn invalid_fixtures_are_rejected() {
    let v = validator();
    for (name, doc) in json_files("invalid") {
        assert!(!v.is_valid(&doc), "{name} should be rejected");
    }
}

#[test]
fn localized_fixture_covers_each_fallback_case() {
    let doc = json_files("valid")
        .into_iter()
        .find(|(n, _)| n == "localized.json")
        .unwrap()
        .1;
    let item = &serde_json::from_value::<Catalog>(doc).unwrap().packages[0];
    assert_eq!(item.kind, SourceKind::Flatpak);
    assert_eq!(item.name.resolve(&["fr"]), "GIMP"); // plain string
    assert_eq!(item.summary.resolve(&["es-DO"]), "Editor de imágenes (RD)"); // exact tag
    assert_eq!(item.summary.resolve(&["es-MX"]), "Editor de imágenes"); // language
    assert_eq!(item.summary.resolve(&["fr"]), "Image editor"); // en
    assert_eq!(item.description.resolve(&["fr"]), "Bildbearbeitung"); // first entry
}

/// Checks a real catalog: `CATALOG_FILE=path cargo test -- --ignored mirror_catalog`.
#[test]
#[ignore]
fn mirror_catalog_validates_and_parses() {
    let path = std::env::var("CATALOG_FILE").expect("set CATALOG_FILE");
    let doc: serde_json::Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let errors: Vec<String> = validator()
        .iter_errors(&doc)
        .take(10)
        .map(|e| e.to_string())
        .collect();
    assert!(errors.is_empty(), "schema errors: {errors:#?}");
    let catalog: Catalog = serde_json::from_value(doc).unwrap();
    println!("{} packages parsed", catalog.packages.len());
}
