use thiserror::Error;

/// Unified error type for bg3-save operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Error from the bg3_lib package reader (uses String errors upstream).
    #[error("{0}")]
    Package(String),

    /// A file was not found inside an LSV package.
    #[error("File '{0}' not found in package")]
    FileNotFound(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Toml(#[from] toml::de::Error),

    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("{0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = Error::Package("bad package".into());
        assert_eq!(err.to_string(), "bad package");

        let err = Error::FileNotFound("globals.lsf".into());
        assert_eq!(err.to_string(), "File 'globals.lsf' not found in package");

        let err = Error::Other("something went wrong".into());
        assert_eq!(err.to_string(), "something went wrong");
    }

    #[test]
    fn error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
    }

    #[test]
    fn error_from_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let err: Error = json_err.into();
        assert!(matches!(err, Error::Json(_)));
    }
}
