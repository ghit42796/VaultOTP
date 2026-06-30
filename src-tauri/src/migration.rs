use crate::error::{AppError, Result};
use crate::model::{Account, Algorithm};
use prost::Message;

#[derive(Clone, PartialEq, Message)]
pub struct OtpParameters {
    #[prost(bytes = "vec", tag = "1")]
    pub secret: Vec<u8>,
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(string, tag = "3")]
    pub issuer: String,
    #[prost(int32, tag = "4")]
    pub algorithm: i32, // 1 SHA1, 2 SHA256, 3 SHA512
    #[prost(int32, tag = "5")]
    pub digits: i32, // 1 SIX, 2 EIGHT
    #[prost(int32, tag = "6")]
    pub r#type: i32, // 1 HOTP, 2 TOTP
    #[prost(int64, tag = "7")]
    pub counter: i64,
}

#[derive(Clone, PartialEq, Message)]
pub struct MigrationPayload {
    #[prost(message, repeated, tag = "1")]
    pub otp_parameters: Vec<OtpParameters>,
    #[prost(int32, tag = "2")]
    pub version: i32,
    #[prost(int32, tag = "3")]
    pub batch_size: i32,
    #[prost(int32, tag = "4")]
    pub batch_index: i32,
    #[prost(int32, tag = "5")]
    pub batch_id: i32,
}

fn url_query_value(uri: &str, key: &str) -> Option<String> {
    let q = uri.split_once('?')?.1;
    for pair in q.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return Some(crate::otpauth_decode(v));
            }
        }
    }
    None
}

pub fn parse_migration(uri: &str) -> Result<Vec<Account>> {
    if !uri.starts_with("otpauth-migration://") {
        return Err(AppError::Migration);
    }
    let data_b64 = url_query_value(uri, "data").ok_or(AppError::Migration)?;
    let raw = base64_decode(&data_b64).ok_or(AppError::Migration)?;
    let payload = MigrationPayload::decode(&raw[..]).map_err(|_| AppError::Migration)?;

    let mut accounts = Vec::new();
    for p in payload.otp_parameters {
        if p.r#type != 2 {
            continue; // TOTP only
        }
        let secret_b32 =
            base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &p.secret);
        let algorithm = match p.algorithm {
            2 => Algorithm::Sha256,
            3 => Algorithm::Sha512,
            _ => Algorithm::Sha1,
        };
        let digits = if p.digits == 2 { 8 } else { 6 };
        accounts.push(Account {
            id: String::new(),
            issuer: p.issuer,
            label: p.name,
            secret: secret_b32,
            algorithm,
            digits,
            period: 30,
            kind: "totp".to_string(),
        });
    }
    if accounts.is_empty() {
        return Err(AppError::Migration);
    }
    Ok(accounts)
}

/// Standard-alphabet Base64 encoder (module scope; shared by build_migration and tests).
fn base64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        out.push(T[(b[0] >> 2) as usize] as char);
        out.push(T[(((b[0] & 0x03) << 4) | (b[1] >> 4)) as usize] as char);
        if chunk.len() > 1 { out.push(T[(((b[1] & 0x0f) << 2) | (b[2] >> 6)) as usize] as char); } else { out.push('='); }
        if chunk.len() > 2 { out.push(T[(b[2] & 0x3f) as usize] as char); } else { out.push('='); }
    }
    out
}

/// Percent-encode the base64 `data` value so it survives a URL round-trip
/// (encodes +, /, = and any non-unreserved byte). `crate::otpauth_decode`
/// reverses this on parse.
fn percent_encode_b64(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

/// Build a single-batch Google Authenticator migration URI for `accounts` (TOTP).
pub fn build_migration(accounts: &[Account]) -> Result<String> {
    if accounts.is_empty() {
        return Err(AppError::Migration);
    }
    let mut params = Vec::with_capacity(accounts.len());
    for a in accounts {
        let secret = crate::secret::decode_secret(&a.secret)?;
        let algorithm = match a.algorithm {
            Algorithm::Sha1 => 1,
            Algorithm::Sha256 => 2,
            Algorithm::Sha512 => 3,
        };
        let digits = if a.digits == 8 { 2 } else { 1 };
        params.push(OtpParameters {
            secret,
            name: a.label.clone(),
            issuer: a.issuer.clone(),
            algorithm,
            digits,
            r#type: 2, // TOTP
            counter: 0,
        });
    }
    let payload = MigrationPayload {
        otp_parameters: params,
        version: 1,
        batch_size: 1,
        batch_index: 0,
        batch_id: 0,
    };
    let mut raw = Vec::new();
    payload.encode(&mut raw).map_err(|_| AppError::Migration)?;
    let b64 = base64_encode(&raw);
    Ok(format!("otpauth-migration://offline?data={}", percent_encode_b64(&b64)))
}

/// Minimal standard Base64 decoder (GA uses standard alphabet, may be URL-encoded first).
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut lut = [255u8; 256];
    for (i, &c) in T.iter().enumerate() {
        lut[c as usize] = i as u8;
    }
    let clean: Vec<u8> = s.bytes().filter(|&b| b != b'=' && !b.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in clean.chunks(4) {
        let mut buf = [0u8; 4];
        let mut n = 0;
        for (i, &c) in chunk.iter().enumerate() {
            let v = lut[c as usize];
            if v == 255 { return None; }
            buf[i] = v;
            n += 1;
        }
        if n >= 2 { out.push((buf[0] << 2) | (buf[1] >> 4)); }
        if n >= 3 { out.push((buf[1] << 4) | (buf[2] >> 2)); }
        if n >= 4 { out.push((buf[2] << 6) | buf[3]); }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_totp_entry() {
        // Build a payload, encode it, wrap in a migration URI, then parse back.
        let secret = vec![0x48u8, 0x65, 0x6c, 0x6c, 0x6f]; // arbitrary bytes
        let payload = MigrationPayload {
            otp_parameters: vec![OtpParameters {
                secret: secret.clone(),
                name: "alice".into(),
                issuer: "Example".into(),
                algorithm: 1,
                digits: 1,
                r#type: 2,
                counter: 0,
            }],
            version: 1, batch_size: 1, batch_index: 0, batch_id: 0,
        };
        let mut raw = Vec::new();
        payload.encode(&mut raw).unwrap();
        let b64 = base64_encode(&raw);
        let uri = format!("otpauth-migration://offline?data={}", b64);

        let accounts = parse_migration(&uri).unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].id, "");
        assert_eq!(accounts[0].issuer, "Example");
        assert_eq!(accounts[0].label, "alice");
        let expected_b32 = base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &secret);
        assert_eq!(accounts[0].secret, expected_b32);
    }

    #[test]
    fn skips_hotp_and_errors_when_empty() {
        let payload = MigrationPayload {
            otp_parameters: vec![OtpParameters {
                secret: vec![1, 2, 3], name: "x".into(), issuer: "y".into(),
                algorithm: 1, digits: 1, r#type: 1, counter: 5, // HOTP
            }],
            version: 1, batch_size: 1, batch_index: 0, batch_id: 0,
        };
        let mut raw = Vec::new();
        payload.encode(&mut raw).unwrap();
        let uri = format!("otpauth-migration://offline?data={}", base64_encode(&raw));
        assert!(matches!(parse_migration(&uri), Err(AppError::Migration)));
    }

    #[test]
    fn empty_otp_parameters_errors() {
        // A valid payload with zero entries should return Err(Migration)
        let payload = MigrationPayload {
            otp_parameters: vec![],
            version: 1, batch_size: 0, batch_index: 0, batch_id: 0,
        };
        let mut raw = Vec::new();
        payload.encode(&mut raw).unwrap();
        let uri = format!("otpauth-migration://offline?data={}", base64_encode(&raw));
        assert!(matches!(parse_migration(&uri), Err(AppError::Migration)));
    }

    #[test]
    fn maps_sha256_and_eight_digits() {
        // Verify algorithm=2 (SHA256) and digits=2 (EIGHT) are mapped correctly
        let secret = vec![0xDEu8, 0xAD, 0xBE, 0xEF];
        let payload = MigrationPayload {
            otp_parameters: vec![OtpParameters {
                secret: secret.clone(),
                name: "bob".into(),
                issuer: "Corp".into(),
                algorithm: 2, // SHA256
                digits: 2,    // EIGHT
                r#type: 2,    // TOTP
                counter: 0,
            }],
            version: 1, batch_size: 1, batch_index: 0, batch_id: 0,
        };
        let mut raw = Vec::new();
        payload.encode(&mut raw).unwrap();
        let uri = format!("otpauth-migration://offline?data={}", base64_encode(&raw));

        let accounts = parse_migration(&uri).unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].algorithm, crate::model::Algorithm::Sha256);
        assert_eq!(accounts[0].digits, 8);
        assert_eq!(accounts[0].period, 30);
    }

    #[test]
    fn build_then_parse_migration_round_trips() {
        let mut a = Account::new("Example".into(), "alice".into(),
            base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &[0x48,0x65,0x6c,0x6c,0x6f]));
        a.algorithm = Algorithm::Sha256;
        a.digits = 8;
        let uri = build_migration(&[a]).unwrap();
        assert!(uri.starts_with("otpauth-migration://offline?data="));
        let back = parse_migration(&uri).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].issuer, "Example");
        assert_eq!(back[0].label, "alice");
        assert_eq!(back[0].algorithm, Algorithm::Sha256);
        assert_eq!(back[0].digits, 8);
    }
}
