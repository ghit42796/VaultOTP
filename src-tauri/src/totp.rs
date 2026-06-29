use crate::error::{AppError, Result};
use crate::model::{Account, Algorithm};
use crate::secret::decode_secret;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::{Sha256, Sha512};

fn hmac_digest(algo: Algorithm, key: &[u8], counter: u64) -> Result<Vec<u8>> {
    let msg = counter.to_be_bytes();
    macro_rules! run {
        ($T:ty) => {{
            let mut m = <Hmac<$T>>::new_from_slice(key)
                .map_err(|_| AppError::InvalidSecret)?;
            m.update(&msg);
            Ok(m.finalize().into_bytes().to_vec())
        }};
    }
    match algo {
        Algorithm::Sha1 => run!(Sha1),
        Algorithm::Sha256 => run!(Sha256),
        Algorithm::Sha512 => run!(Sha512),
    }
}

/// RFC 6238 TOTP. `unix_time` is seconds since epoch.
pub fn generate(account: &Account, unix_time: u64) -> Result<String> {
    if account.period == 0 {
        return Err(AppError::Other("period must not be zero".into()));
    }
    if !(6..=8).contains(&account.digits) {
        return Err(AppError::Other(format!(
            "digits must be 6-8, got {}",
            account.digits
        )));
    }
    let key = decode_secret(&account.secret)?;
    let counter = unix_time / account.period;
    let digest = hmac_digest(account.algorithm, &key, counter)?;
    let offset = (digest[digest.len() - 1] & 0x0f) as usize;
    let bin = ((digest[offset] as u32 & 0x7f) << 24)
        | ((digest[offset + 1] as u32) << 16)
        | ((digest[offset + 2] as u32) << 8)
        | (digest[offset + 3] as u32);
    let modulo = 10u32.pow(account.digits);
    let code = bin % modulo;
    Ok(format!("{:0width$}", code, width = account.digits as usize))
}

/// Returns the number of seconds until the current TOTP window expires.
pub fn remaining_seconds(account: &Account, unix_time: u64) -> u64 {
    if account.period == 0 {
        return 0;
    }
    account.period - (unix_time % account.period)
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 6238 Appendix B uses ASCII seed "12345678901234567890" => Base32 below.
    fn vec_account(algo: Algorithm, b32: &str) -> Account {
        let mut a = Account::new("t".into(), "t".into(), b32.into());
        a.algorithm = algo;
        a.digits = 8;
        a.period = 30;
        a
    }

    #[test]
    fn rfc6238_sha1_vectors() {
        let a = vec_account(Algorithm::Sha1, "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
        assert_eq!(generate(&a, 59).unwrap(), "94287082");
        assert_eq!(generate(&a, 1111111109).unwrap(), "07081804");
        assert_eq!(generate(&a, 2000000000).unwrap(), "69279037");
    }

    #[test]
    fn rfc6238_sha256_vector() {
        // 32-byte seed "12345678901234567890123456789012"
        let a = vec_account(
            Algorithm::Sha256,
            "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZA",
        );
        assert_eq!(generate(&a, 59).unwrap(), "46119246");
    }

    #[test]
    fn rfc6238_sha512_vector() {
        // 64-byte seed "1234567890" repeated 6x + "1234"
        // = "1234567890123456789012345678901234567890123456789012345678901234"
        // Base32 computed from ASCII bytes (no padding):
        // GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNA
        // (Brief's literal was wrong; corrected by encoding the 64-byte seed)
        let a = vec_account(
            Algorithm::Sha512,
            "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNA",
        );
        assert_eq!(generate(&a, 59).unwrap(), "90693936");
    }

    #[test]
    fn default_six_digits() {
        let mut a = Account::new(
            "t".into(),
            "t".into(),
            "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".into(),
        );
        a.digits = 6;
        let code = generate(&a, 59).unwrap();
        assert_eq!(code, "287082");
    }

    #[test]
    fn remaining_counts_down() {
        let a = Account::new("t".into(), "t".into(), "GEZDGNBVGY3TQOJQ".into());
        assert_eq!(remaining_seconds(&a, 0), 30);
        assert_eq!(remaining_seconds(&a, 29), 1);
        assert_eq!(remaining_seconds(&a, 30), 30);
    }

    #[test]
    fn generate_rejects_zero_period() {
        let mut a = Account::new("t".into(), "t".into(), "GEZDGNBVGY3TQOJQ".into());
        a.period = 0;
        assert!(generate(&a, 0).is_err());
    }

    #[test]
    fn generate_rejects_invalid_digits() {
        let mut a = Account::new("t".into(), "t".into(), "GEZDGNBVGY3TQOJQ".into());
        a.digits = 9;
        assert!(generate(&a, 0).is_err());
        a.digits = 5;
        assert!(generate(&a, 0).is_err());
    }
}
