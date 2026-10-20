use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

pub fn read<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    match fs::read(path) {
        Ok(bytes) => {
            Ok(Some(serde_json::from_slice(&bytes).with_context(|| {
                format!("Invalid state file {}", path.display())
            })?))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("Reading {}", path.display())),
    }
}
pub fn write<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    write_bytes(path, &serde_json::to_vec_pretty(value)?)
}
pub fn write_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let tmp = path.with_extension("tmp");
    let mut file = File::create(&tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&tmp, path)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}
pub fn html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn state_roundtrip_and_corruption_is_not_reset() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("nested/state.json");
        assert_eq!(read::<Vec<u64>>(&path).unwrap(), None);
        write(&path, &vec![42_u64]).unwrap();
        assert_eq!(read::<Vec<u64>>(&path).unwrap(), Some(vec![42]));
        fs::write(&path, b"{").unwrap();
        assert!(read::<Vec<u64>>(&path).is_err());
    }
}
