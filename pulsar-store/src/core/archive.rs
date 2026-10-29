//! Safe file helpers for installing downloaded content.

use crate::core::error::{Result, StoreError};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// Accepts an id only if it is a single, plain path component. Ids from the
/// catalog become directory names, so `../x` and `a/b` must never pass.
pub fn safe_component(id: &str) -> Result<&str> {
    let bad = id.is_empty()
        || id == "."
        || id == ".."
        || id.contains(['/', '\\', '\0'])
        || id.starts_with('.');
    if bad {
        return Err(StoreError::Unsupported(format!(
            "unsafe identifier: {id:?}"
        )));
    }
    Ok(id)
}

/// Extracts a zip into `dest`, refusing entries that would escape it.
pub fn extract_zip(zip_path: &Path, dest: &Path) -> Result<()> {
    let mut archive = zip::ZipArchive::new(File::open(zip_path)?)
        .map_err(|e| StoreError::Parse(format!("bad zip: {e}")))?;
    fs::create_dir_all(dest)?;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| StoreError::Parse(format!("bad zip entry: {e}")))?;
        let Some(rel) = entry.enclosed_name() else {
            return Err(StoreError::Unsupported(format!(
                "zip entry escapes the target folder: {}",
                entry.name()
            )));
        };
        let out = dest.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&out)?;
        } else {
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent)?;
            }
            io::copy(&mut entry, &mut File::create(&out)?)?;
        }
    }
    Ok(())
}

pub fn is_zip(path: &Path) -> bool {
    let mut magic = [0u8; 4];
    File::open(path)
        .and_then(|mut f| f.read_exact(&mut magic))
        .is_ok()
        && magic == *b"PK\x03\x04"
}

/// Recursively copies a directory into `dest`, creating it.
pub fn copy_dir(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Finds the directory under `root` (including `root`) that contains `file_name`.
pub fn find_dir_with(root: &Path, file_name: &str) -> Option<PathBuf> {
    if root.join(file_name).is_file() {
        return Some(root.to_path_buf());
    }
    for entry in fs::read_dir(root).ok()?.flatten() {
        if entry.file_type().ok()?.is_dir() {
            if let Some(found) = find_dir_with(&entry.path(), file_name) {
                return Some(found);
            }
        }
    }
    None
}

pub fn sha256_hex(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Checks a downloaded file against the checksum the catalog declared.
/// Does nothing when the catalog declares none.
pub fn verify_sha256(path: &Path, expected: Option<&str>) -> Result<()> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let actual = sha256_hex(path)?;
    if actual.eq_ignore_ascii_case(expected.trim()) {
        Ok(())
    } else {
        Err(StoreError::Parse(format!(
            "checksum mismatch: expected {expected}, got {actual}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_zip(path: &Path, files: &[(&str, &str)]) {
        let mut zw = zip::ZipWriter::new(File::create(path).unwrap());
        for (name, body) in files {
            zw.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zw.write_all(body.as_bytes()).unwrap();
        }
        zw.finish().unwrap();
    }

    #[test]
    fn safe_component_rules() {
        assert!(safe_component("org.gimp.GIMP").is_ok());
        assert!(safe_component("ext@example.org").is_ok());
        for bad in ["", ".", "..", "../x", "a/b", "a\\b", ".hidden"] {
            assert!(safe_component(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn extracts_and_finds_nested_file() {
        let dir = tempfile::tempdir().unwrap();
        let z = dir.path().join("a.zip");
        make_zip(&z, &[("pkg/manifest.json", "{}"), ("pkg/run.sh", "x")]);
        assert!(is_zip(&z));
        let out = dir.path().join("out");
        extract_zip(&z, &out).unwrap();
        assert_eq!(find_dir_with(&out, "manifest.json"), Some(out.join("pkg")));
        assert_eq!(find_dir_with(&out, "nope"), None);
    }

    #[test]
    fn rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let z = dir.path().join("evil.zip");
        make_zip(&z, &[("../escape.txt", "x")]);
        let out = dir.path().join("out");
        assert!(extract_zip(&z, &out).is_err());
        assert!(!dir.path().join("escape.txt").exists());
    }

    #[test]
    fn checksum_verification() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("f");
        fs::write(&f, "hello").unwrap();
        let good = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(verify_sha256(&f, Some(good)).is_ok());
        assert!(verify_sha256(&f, Some(&good.to_uppercase())).is_ok());
        assert!(verify_sha256(&f, Some("00")).is_err());
        assert!(verify_sha256(&f, None).is_ok());
    }

    #[test]
    fn copies_directories() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("s");
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("sub/f"), "x").unwrap();
        copy_dir(&src, &dir.path().join("d")).unwrap();
        assert!(dir.path().join("d/sub/f").is_file());
    }
}
