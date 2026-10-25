//! Versioned JSON store with atomic writes and tolerant reads.
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

pub const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct Envelope<T> {
    version: u32,
    data: T,
}

#[derive(Clone, Debug)]
pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        Store { dir }
    }

    pub fn default_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pulsaros-spotlight")
            .join("db")
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.json"))
    }

    /// Missing, corrupt, wrong-shape or newer-version files all yield `T::default()`.
    pub fn load<T: DeserializeOwned + Default>(&self, name: &str) -> T {
        let Ok(text) = fs::read_to_string(self.path(name)) else {
            return T::default();
        };
        match serde_json::from_str::<Envelope<T>>(&text) {
            Ok(e) if e.version <= VERSION => e.data,
            _ => T::default(),
        }
    }

    /// Writes `<name>.json.tmp` then renames over the target.
    pub fn save<T: Serialize>(&self, name: &str, data: &T) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let path = self.path(name);
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(&Envelope { version: VERSION, data })
            .map_err(io::Error::other)?;
        fs::write(&tmp, text)?;
        fs::rename(&tmp, &path)
    }
}

#[cfg(test)]
pub fn temp_store(tag: &str) -> Store {
    let dir = std::env::temp_dir().join(format!("pulsaros-core-{}-{}", tag, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    Store::new(dir)
}

#[cfg(test)]
mod tests {
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
}
