use crate::error::{AppError, Result};
use crate::vault::kdf::{derive_key, KdfParams};
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// Which credential(s) unlock a vault. Stored in the header and authenticated as AAD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Password,
    Keyfile,
    Composite,
}

pub const MAGIC: &[u8] = b"ATOTP1\0";

// Compile-time guard: file-layout comment says MAGIC is 7 bytes.
const _: () = assert!(MAGIC.len() == 7);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultHeader {
    pub version: u16,
    /// Credential mode. `#[serde(default)]` makes pre-`mode` (v1) files parse as `Password`.
    #[serde(default)]
    pub mode: Mode,
    pub kdf: KdfParams,
    pub salt: [u8; 16],
    /// Caller MUST supply a cryptographically random nonce, unique per (key, vault write).
    /// Reusing a nonce with the same key breaks AES-GCM confidentiality entirely.
    pub nonce: [u8; 12],
}

/// File layout: MAGIC (7) | header_len: u32 LE (4) | header_json | ciphertext+tag
/// The bytes `MAGIC | header_len | header_json` are the AEAD associated data (AAD).
fn aad(header_json: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(MAGIC.len() + 4 + header_json.len());
    v.extend_from_slice(MAGIC);
    v.extend_from_slice(&(header_json.len() as u32).to_le_bytes());
    v.extend_from_slice(header_json);
    v
}

/// Serialize a `VaultHeader` to JSON bytes. Returns `Err(AppError::Crypto)` on failure.
pub fn serialize_header(h: &VaultHeader) -> Result<Vec<u8>> {
    serde_json::to_vec(h).map_err(|_| AppError::Crypto)
}

/// Parse a vault file's header region.
///
/// Returns `(header, ciphertext_offset)` where `file_bytes[ciphertext_offset..]`
/// is the ciphertext+tag block.
pub fn parse_header(bytes: &[u8]) -> Result<(VaultHeader, usize)> {
    if bytes.len() < MAGIC.len() + 4 || &bytes[..MAGIC.len()] != MAGIC {
        return Err(AppError::Crypto);
    }
    let len_start = MAGIC.len();
    let header_len = u32::from_le_bytes(
        bytes[len_start..len_start + 4].try_into().map_err(|_| AppError::Crypto)?,
    ) as usize;
    let header_start = len_start + 4;
    let header_end = header_start + header_len;
    if bytes.len() < header_end {
        return Err(AppError::Crypto);
    }
    let header: VaultHeader =
        serde_json::from_slice(&bytes[header_start..header_end]).map_err(|_| AppError::Crypto)?;
    Ok((header, header_end))
}

/// Encrypt `plaintext` and return full vault file bytes.
///
/// The caller is responsible for generating a random nonce and placing it in
/// `header.nonce` before calling this function. See `VaultHeader::nonce` docs.
pub fn encrypt(key: &[u8; 32], header: &VaultHeader, plaintext: &[u8]) -> Result<Vec<u8>> {
    let header_json = serialize_header(header)?;
    let associated = aad(&header_json);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(&header.nonce);
    let ct = cipher
        .encrypt(nonce, Payload { msg: plaintext, aad: &associated })
        .map_err(|_| AppError::Crypto)?;
    let mut out = associated;
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Decrypt a vault file given an already-derived 32-byte content key.
/// Verifies the AEAD tag against the header AAD. `Err(AppError::Crypto)` on any failure.
pub fn decrypt_with_derived_key(key: &[u8; 32], file_bytes: &[u8]) -> Result<Vec<u8>> {
    let (header, header_end) = parse_header(file_bytes)?;
    let header_json = &file_bytes[MAGIC.len() + 4..header_end];
    let associated = aad(header_json);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(&header.nonce);
    cipher
        .decrypt(nonce, Payload { msg: &file_bytes[header_end..], aad: &associated })
        .map_err(|_| AppError::Crypto)
}

/// Decrypt a vault file given the user password. Derives the key from the
/// embedded KDF parameters and salt, then verifies the AEAD tag.
///
/// Returns `Err(AppError::Crypto)` for any failure (wrong password, corruption,
/// truncation, or header tampering) — no oracle about which check failed.
///
/// This is a thin wrapper over [`decrypt_with_key`] that discards the derived
/// key, keeping existing callers (e.g. `backup::import_encrypted`) unchanged.
pub fn decrypt(password: &[u8], file_bytes: &[u8]) -> Result<Vec<u8>> {
    let (plaintext, _key) = decrypt_with_key(password, file_bytes)?;
    Ok(plaintext)
}

/// Decrypt a vault file and return BOTH the plaintext and the derived key.
///
/// Identical behavior to [`decrypt`] (same opaque `AppError::Crypto` on every
/// failure path), but additionally hands back the Argon2id-derived key so a
/// caller that needs to keep the vault unlocked does not have to re-derive it.
/// The key is wrapped in `Zeroizing` and clears itself when dropped.
pub fn decrypt_with_key(
    password: &[u8],
    file_bytes: &[u8],
) -> Result<(Vec<u8>, Zeroizing<[u8; 32]>)> {
    // parse_header validates MAGIC, bounds, and JSON parsing.
    let (header, header_end) = parse_header(file_bytes)?;

    // Raw header JSON slice — must match what encrypt() serialised for AAD to verify.
    let header_json = &file_bytes[MAGIC.len() + 4..header_end];

    // derive_key returns Zeroizing<[u8;32]>; &*key deref-coerces to &[u8;32].
    let key = derive_key(password, &header.salt, &header.kdf)?;
    let associated = aad(header_json);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&*key));
    let nonce = Nonce::from_slice(&header.nonce);
    let plaintext = cipher
        .decrypt(nonce, Payload { msg: &file_bytes[header_end..], aad: &associated })
        .map_err(|_| AppError::Crypto)?;
    Ok((plaintext, key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeroize::Zeroizing;

    /// Returns a header and derived key using fast (test-only) KDF parameters.
    fn fast_header() -> (VaultHeader, Zeroizing<[u8; 32]>) {
        let kdf = KdfParams { mem_kib: 8192, iterations: 1, parallelism: 1 };
        let salt = [9u8; 16];
        let nonce = [3u8; 12];
        let key = derive_key(b"masterpw", &salt, &kdf).unwrap();
        (VaultHeader { version: 1, mode: Mode::Password, kdf, salt, nonce }, key)
    }

    #[test]
    fn round_trip() {
        let (h, key) = fast_header();
        let file = encrypt(&*key, &h, b"secret data").unwrap();
        let pt = decrypt(b"masterpw", &file).unwrap();
        assert_eq!(pt, b"secret data");
    }

    #[test]
    fn wrong_password_fails() {
        let (h, key) = fast_header();
        let file = encrypt(&*key, &h, b"secret data").unwrap();
        assert!(matches!(decrypt(b"wrongpw", &file), Err(AppError::Crypto)));
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let (h, key) = fast_header();
        let mut file = encrypt(&*key, &h, b"secret data").unwrap();
        let last = file.len() - 1;
        file[last] ^= 0x01;
        assert!(matches!(decrypt(b"masterpw", &file), Err(AppError::Crypto)));
    }

    #[test]
    fn header_without_mode_field_defaults_to_password() {
        // A v1 header JSON has no "mode" key; it must parse as Mode::Password.
        let v1_json = br#"{"version":1,"kdf":{"mem_kib":8192,"iterations":1,"parallelism":1},"salt":[9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9],"nonce":[3,3,3,3,3,3,3,3,3,3,3,3]}"#;
        let h: VaultHeader = serde_json::from_slice(v1_json).unwrap();
        assert_eq!(h.mode, Mode::Password);
    }

    #[test]
    fn mode_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Mode::Composite).unwrap(), "\"composite\"");
    }

    #[test]
    fn tampered_header_fails() {
        let (h, key) = fast_header();
        let mut file = encrypt(&*key, &h, b"secret data").unwrap();
        // Locate and flip a byte that is a digit inside a KDF numeric field so
        // JSON still parses (proving AEAD tag mismatch, not parse error).
        // Header JSON looks like: {"version":1,"kdf":{"mem_kib":8192,...
        // We scan the header region for a digit character to flip safely.
        let header_start = MAGIC.len() + 4;
        let header_json_orig = serde_json::to_vec(&h).unwrap();
        let header_end = header_start + header_json_orig.len();
        // Find index of first ASCII digit in header JSON region.
        let digit_offset = file[header_start..header_end]
            .iter()
            .position(|&b| b.is_ascii_digit())
            .expect("header JSON must contain at least one digit");
        // Flip the digit to an adjacent digit (XOR with 1 keeps it a digit if even,
        // or adjusts by 1; fallback: just flip bit 0 which moves between adjacent digits).
        file[header_start + digit_offset] ^= 0x01;
        // This changes the header bytes, so the AAD passed to decrypt() will differ
        // from the AAD used at encrypt time → GCM tag check fails.
        assert!(matches!(decrypt(b"masterpw", &file), Err(AppError::Crypto)));
    }
}
