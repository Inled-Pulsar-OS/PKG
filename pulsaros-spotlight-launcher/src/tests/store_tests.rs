use super::*;
use std::collections::BTreeMap;

#[test]
fn roundtrip() {
    let s = temp_store("rt");
    let mut m = BTreeMap::new();
    m.insert("a".to_string(), 1u32);
    s.save("x", &m).unwrap();
    assert_eq!(s.load::<BTreeMap<String, u32>>("x"), m);
    assert!(!s.path("x").with_extension("json.tmp").exists());
}

#[test]
fn missing_is_default() {
    let s = temp_store("missing");
    assert!(s.load::<Vec<u32>>("nope").is_empty());
}

#[test]
fn corrupt_is_default() {
    let s = temp_store("corrupt");
    s.save("x", &vec![1u32]).unwrap();
    fs::write(s.path("x"), "{ not json").unwrap();
    assert!(s.load::<Vec<u32>>("x").is_empty());
}

#[test]
fn wrong_shape_is_default() {
    let s = temp_store("shape");
    fs::create_dir_all(&s.dir).unwrap();
    fs::write(s.path("x"), r#"{"version":1,"data":"str"}"#).unwrap();
    assert!(s.load::<Vec<u32>>("x").is_empty());
}

#[test]
fn newer_version_is_default() {
    let s = temp_store("newer");
    fs::create_dir_all(&s.dir).unwrap();
    fs::write(s.path("x"), r#"{"version":99,"data":[1]}"#).unwrap();
    assert!(s.load::<Vec<u32>>("x").is_empty());
}

#[test]
fn file_has_version_field() {
    let s = temp_store("ver");
    s.save("x", &vec![1u32]).unwrap();
    let v: serde_json::Value = serde_json::from_str(&fs::read_to_string(s.path("x")).unwrap()).unwrap();
    assert_eq!(v["version"], 1);
}
