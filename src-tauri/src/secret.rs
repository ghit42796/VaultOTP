use crate::error::{AppError, Result};

/// Decode an RFC 4648 Base32 secret, tolerating spaces, lowercase, and padding.
pub fn decode_secret(s: &str) -> Result<Vec<u8>> {
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect::<String>()
        .trim_end_matches('=')
        .to_uppercase();
    if cleaned.is_empty() {
        return Err(AppError::InvalidSecret);
    }
    base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &cleaned)
        .ok_or(AppError::InvalidSecret)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_known_secret() {
        // "Hello!\xDE\xAD\xBE\xEF" classic test vector
        let bytes = decode_secret("JBSWY3DPEHPK3PXP").unwrap();
        assert_eq!(bytes, vec![0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x21, 0xde, 0xad, 0xbe, 0xef]);
    }
    #[test]
    fn tolerates_spaces_and_lowercase() {
        let a = decode_secret("jbsw y3dp ehpk 3pxp").unwrap();
        let b = decode_secret("JBSWY3DPEHPK3PXP").unwrap();
        assert_eq!(a, b);
    }
    #[test]
    fn rejects_invalid() {
        assert!(matches!(decode_secret("not base32!!"), Err(AppError::InvalidSecret)));
        assert!(matches!(decode_secret("   "), Err(AppError::InvalidSecret)));
    }
}
