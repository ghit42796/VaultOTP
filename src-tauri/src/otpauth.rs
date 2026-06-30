use crate::error::{AppError, Result};
use crate::model::{Account, Algorithm};

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                if let (Some(h), Some(l)) = (hi, lo) {
                    out.push((h * 16 + l) as u8);
                    i += 3;
                    continue;
                }
                out.push(bytes[i]);
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn parse_otpauth(uri: &str) -> Result<Account> {
    let rest = uri
        .strip_prefix("otpauth://totp/")
        .ok_or(AppError::InvalidSecret)?;
    let (path, query) = match rest.split_once('?') {
        Some((p, q)) => (p, q),
        None => (rest, ""),
    };

    let label_part = url_decode(path);
    let (issuer_from_label, label) = match label_part.split_once(':') {
        Some((i, l)) => (i.trim().to_string(), l.trim().to_string()),
        None => (String::new(), label_part.trim().to_string()),
    };

    let mut secret = String::new();
    let mut issuer = issuer_from_label;
    let mut algorithm = Algorithm::Sha1;
    let mut digits = 6u32;
    let mut period = 30u64;

    for pair in query.split('&').filter(|s| !s.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let v = url_decode(v);
        match k {
            "secret" => secret = v,
            "issuer" => issuer = v,
            "algorithm" => {
                algorithm = match v.to_uppercase().as_str() {
                    "SHA1" => Algorithm::Sha1,
                    "SHA256" => Algorithm::Sha256,
                    "SHA512" => Algorithm::Sha512,
                    _ => return Err(AppError::InvalidSecret),
                }
            }
            "digits" => digits = v.parse().map_err(|_| AppError::InvalidSecret)?,
            "period" => period = v.parse().map_err(|_| AppError::InvalidSecret)?,
            _ => {}
        }
    }

    if secret.is_empty() {
        return Err(AppError::InvalidSecret);
    }
    crate::secret::decode_secret(&secret)?; // validate Base32

    Ok(Account {
        id: String::new(),
        issuer,
        label,
        secret,
        algorithm,
        digits,
        period,
        kind: "totp".to_string(),
    })
}

/// Percent-encode all bytes except RFC 3986 unreserved (ALPHA / DIGIT / -._~).
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        let unreserved = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~');
        if unreserved {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

/// Build an `otpauth://totp/...` URI for an account. Inverse of `parse_otpauth`.
pub fn build_otpauth(a: &Account) -> String {
    let label_path = if a.issuer.is_empty() {
        percent_encode(&a.label)
    } else {
        format!("{}:{}", percent_encode(&a.issuer), percent_encode(&a.label))
    };
    let algo = match a.algorithm {
        Algorithm::Sha1 => "SHA1",
        Algorithm::Sha256 => "SHA256",
        Algorithm::Sha512 => "SHA512",
    };
    let mut uri = format!("otpauth://totp/{}?secret={}", label_path, a.secret);
    if !a.issuer.is_empty() {
        uri.push_str(&format!("&issuer={}", percent_encode(&a.issuer)));
    }
    uri.push_str(&format!("&algorithm={}&digits={}&period={}", algo, a.digits, a.period));
    uri
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_then_parse_round_trips() {
        let mut a = Account::new("GitHub".into(), "alice@example.com".into(), "JBSWY3DPEHPK3PXP".into());
        a.algorithm = Algorithm::Sha256;
        a.digits = 8;
        a.period = 60;
        let uri = build_otpauth(&a);
        assert!(uri.starts_with("otpauth://totp/"));
        let back = parse_otpauth(&uri).unwrap();
        assert_eq!(back.issuer, "GitHub");
        assert_eq!(back.label, "alice@example.com");
        assert_eq!(back.secret, "JBSWY3DPEHPK3PXP");
        assert_eq!(back.algorithm, Algorithm::Sha256);
        assert_eq!(back.digits, 8);
        assert_eq!(back.period, 60);
    }

    #[test]
    fn build_without_issuer_omits_issuer_param() {
        let a = Account::new(String::new(), "solo".into(), "JBSWY3DPEHPK3PXP".into());
        let uri = build_otpauth(&a);
        assert!(!uri.contains("issuer="));
        let back = parse_otpauth(&uri).unwrap();
        assert_eq!(back.label, "solo");
        assert_eq!(back.issuer, "");
    }

    #[test]
    fn parses_full_uri() {
        let uri = "otpauth://totp/GitHub:alice%40example.com?secret=JBSWY3DPEHPK3PXP&issuer=GitHub&algorithm=SHA256&digits=8&period=60";
        let a = parse_otpauth(uri).unwrap();
        assert_eq!(a.id, "");
        assert_eq!(a.issuer, "GitHub");
        assert_eq!(a.label, "alice@example.com");
        assert_eq!(a.secret, "JBSWY3DPEHPK3PXP");
        assert_eq!(a.algorithm, Algorithm::Sha256);
        assert_eq!(a.digits, 8);
        assert_eq!(a.period, 60);
    }

    #[test]
    fn parses_minimal_uri_with_defaults() {
        let a = parse_otpauth("otpauth://totp/alice?secret=JBSWY3DPEHPK3PXP").unwrap();
        assert_eq!(a.label, "alice");
        assert_eq!(a.digits, 6);
        assert_eq!(a.period, 30);
        assert_eq!(a.algorithm, Algorithm::Sha1);
    }

    #[test]
    fn rejects_missing_secret() {
        assert!(parse_otpauth("otpauth://totp/alice").is_err());
    }

    #[test]
    fn rejects_non_totp() {
        assert!(matches!(
            parse_otpauth("otpauth://hotp/alice?secret=JBSWY3DPEHPK3PXP&counter=0"),
            Err(AppError::InvalidSecret)
        ));
    }

    #[test]
    fn rejects_unknown_algorithm() {
        assert!(parse_otpauth("otpauth://totp/alice?secret=JBSWY3DPEHPK3PXP&algorithm=SHA3").is_err());
    }

    #[test]
    fn rejects_invalid_digits() {
        assert!(parse_otpauth("otpauth://totp/alice?secret=JBSWY3DPEHPK3PXP&digits=abc").is_err());
    }

    #[test]
    fn url_decode_trailing_percent_escape() {
        // Verify that a %XX at the very end of the string decodes correctly.
        // "alice%40" — %40 is '@', trailing position.
        assert_eq!(url_decode("alice%40"), "alice@");
        // Lone % at end should pass through literally (no panic / no drop).
        assert_eq!(url_decode("foo%"), "foo%");
        // %4 (incomplete) at end should pass through literally.
        assert_eq!(url_decode("foo%4"), "foo%4");
    }
}
