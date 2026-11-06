//! Filesystem boundary used by the storage reader.
//!
//! The Linux implementation calls `std::fs`. A future `qdb-platform-soso` adapter
//! should implement [`FileSystem`] with the same read-only contract: full-file
//! reads, directory listing, and no memory mapping. Callers check bounds on the
//! returned bytes. This crate does not use `unsafe`.

use qdb_types::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub trait FileSystem {
    fn read(&self, path: &Path) -> Result<Vec<u8>, Error>;
    fn is_dir(&self, path: &Path) -> bool;
    fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>, Error>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct StdFs;

impl FileSystem for StdFs {
    fn read(&self, path: &Path) -> Result<Vec<u8>, Error> {
        fs::read(path).map_err(|err| Error::new(format!("cannot read {}: {err}", path.display())))
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>, Error> {
        let entries = fs::read_dir(path)
            .map_err(|err| Error::new(format!("cannot list {}: {err}", path.display())))?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry
                .map_err(|err| Error::new(format!("cannot list {}: {err}", path.display())))?;
            names.push(entry.path());
        }
        names.sort();
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn reads_a_file_it_created() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("qdb-platform-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("sample.txt");
        fs::write(&file, b"abc").unwrap();
        let fs = StdFs;
        assert_eq!(fs.read(&file).unwrap(), b"abc");
        assert!(fs.is_dir(&dir));
        assert!(fs.list_dir(&dir).unwrap().iter().any(|p| p == &file));
        fs::remove_dir_all(&dir).unwrap();
    }
}
