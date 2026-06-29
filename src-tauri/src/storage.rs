use crate::error::{AppError, Result};
use std::io::Write;
use std::path::Path;

pub fn exists(path: &Path) -> bool {
    path.exists()
}

pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|e| AppError::Other(format!("read failed: {e}")))
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = std::fs::File::create(&tmp)
            .map_err(|e| AppError::Other(format!("create tmp failed: {e}")))?;
        f.write_all(bytes).map_err(|e| AppError::Other(format!("write failed: {e}")))?;
        f.sync_all().map_err(|e| AppError::Other(format!("fsync failed: {e}")))?;
    }
    std::fs::rename(&tmp, path).map_err(|e| AppError::Other(format!("rename failed: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_then_read_round_trips() {
        let dir = std::env::temp_dir().join(format!("atotp_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("vault.bin");
        assert!(!exists(&path));
        write_atomic(&path, b"hello").unwrap();
        assert!(exists(&path));
        assert_eq!(read_file(&path).unwrap(), b"hello");
        // overwrite
        write_atomic(&path, b"world!!").unwrap();
        assert_eq!(read_file(&path).unwrap(), b"world!!");
        // no leftover tmp
        assert!(!dir.join("vault.tmp").exists());
        std::fs::remove_dir_all(&dir).ok();
    }
}
