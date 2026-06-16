use crate::errors::{Result, ZapretError};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).map_err(|source| crate::errors::io_error(path, source))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(hex::encode(hasher.finalize()))
}

pub fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
    let actual = sha256_file(path)?;
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(ZapretError::HashMismatch {
            path: path.to_path_buf(),
            expected: expected.to_string(),
            actual,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn verifies_hash() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("sample.txt");
        fs::write(&file, b"zapret-manager").expect("write");
        let hash = sha256_file(&file).expect("hash");
        assert!(verify_sha256(&file, &hash).is_ok());
        assert!(verify_sha256(&file, "00").is_err());
    }
}
