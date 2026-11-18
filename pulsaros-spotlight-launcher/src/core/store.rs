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
#[path = "../tests/store_tests.rs"]
mod tests;
