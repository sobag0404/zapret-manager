use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ZapretError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("json error at {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("hash mismatch for {path}: expected {expected}, actual {actual}")]
    HashMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },
    #[error("untrusted source: {0}")]
    UntrustedSource(String),
    #[error("unsafe path outside allowed directory: {0}")]
    UnsafePath(PathBuf),
    #[error("operation failed: {0}")]
    Operation(String),
}

pub type Result<T> = std::result::Result<T, ZapretError>;

pub fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> ZapretError {
    ZapretError::Io {
        path: path.into(),
        source,
    }
}

pub fn json_error(path: impl Into<PathBuf>, source: serde_json::Error) -> ZapretError {
    ZapretError::Json {
        path: path.into(),
        source,
    }
}
