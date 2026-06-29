use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Incorrect password or corrupted vault")]
    Crypto,
    #[error("Invalid Base32 secret")]
    InvalidSecret,
    #[error("Could not decode QR code")]
    QrDecode,
    #[error("Invalid migration data")]
    Migration,
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, AppError>;

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crypto_error_is_opaque() {
        assert_eq!(AppError::Crypto.to_string(), "Incorrect password or corrupted vault");
    }
    #[test]
    fn crypto_error_serializes_to_string() {
        let json = serde_json::to_string(&AppError::Crypto).unwrap();
        assert_eq!(json, "\"Incorrect password or corrupted vault\"");
    }
}
