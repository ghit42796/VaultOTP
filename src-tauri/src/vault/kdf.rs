use crate::error::{AppError, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct KdfParams {
    /// Memory cost in KiB (default 64 MiB).
    pub mem_kib: u32,
    /// Number of iterations (time cost).
    pub iterations: u32,
    /// Degree of parallelism.
    pub parallelism: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        // 64 MiB / 3 iterations / 4 threads — above OWASP minimum; deliberate
        // for a desktop app where memory is not constrained.
        KdfParams { mem_kib: 65536, iterations: 3, parallelism: 4 }
    }
}

/// Composite key (KeePass-style): SHA-256(material) then Argon2id.
///
/// # Salt
/// Callers MUST supply a cryptographically random 16-byte salt, unique per
/// vault file, and never reused across vaults.
///
/// # Returns
/// A `Zeroizing<[u8; 32]>` that clears itself when dropped.
pub fn derive_key(
    material: &[u8],
    salt: &[u8; 16],
    params: &KdfParams,
) -> Result<Zeroizing<[u8; 32]>> {
    use sha2::{Digest, Sha256};

    // Step 1: Pre-hash material to a fixed-size input (KeePass composite key pattern).
    let mut composite = Sha256::digest(material);

    // Step 2: Stretch with Argon2id.
    let p = Params::new(params.mem_kib, params.iterations, params.parallelism, Some(32))
        .map_err(|_| AppError::Crypto)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, p);
    let mut out = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(&composite, salt, out.as_mut())
        .map_err(|_| AppError::Crypto)?;

    // Clear the SHA-256 intermediate from the stack.
    composite.zeroize();

    Ok(out)
}

/// Build the KDF input for a composite (password + key file) credential:
/// `SHA-256(password) ‖ SHA-256(keyfile)` (64 bytes). Feeding this into
/// `derive_key` applies the existing SHA-256 + Argon2id stretch on top.
///
/// The returned buffer is secret derived key material; it is wrapped in
/// `Zeroizing` so it clears itself on drop. `Zeroizing<Vec<u8>>` derefs to
/// `Vec<u8>`/`[u8]`, so it passes to `derive_key(material: &[u8], ...)` directly.
pub fn composite_material(password: &[u8], keyfile: &[u8]) -> Zeroizing<Vec<u8>> {
    use sha2::{Digest, Sha256};
    let mut v = Zeroizing::new(Vec::with_capacity(64));

    // Hash each input separately, then zeroize the stack intermediates,
    // mirroring the `composite.zeroize()` treatment in `derive_key`.
    let mut pw_hash = Sha256::digest(password);
    let mut kf_hash = Sha256::digest(keyfile);
    v.extend_from_slice(&pw_hash);
    v.extend_from_slice(&kf_hash);
    pw_hash.zeroize();
    kf_hash.zeroize();

    v
}

#[cfg(test)]
mod tests {
    use super::*;

    // Use small params in tests so they run fast.
    fn fast() -> KdfParams { KdfParams { mem_kib: 8192, iterations: 1, parallelism: 1 } }

    #[test]
    fn same_input_same_key() {
        let salt = [7u8; 16];
        let a = derive_key(b"hunter2", &salt, &fast()).unwrap();
        let b = derive_key(b"hunter2", &salt, &fast()).unwrap();
        assert_eq!(*a, *b);
    }

    #[test]
    fn different_password_different_key() {
        let salt = [7u8; 16];
        let a = derive_key(b"hunter2", &salt, &fast()).unwrap();
        let b = derive_key(b"hunter3", &salt, &fast()).unwrap();
        assert_ne!(*a, *b);
    }

    #[test]
    fn different_salt_different_key() {
        let a = derive_key(b"pw", &[1u8; 16], &fast()).unwrap();
        let b = derive_key(b"pw", &[2u8; 16], &fast()).unwrap();
        assert_ne!(*a, *b);
    }

    #[test]
    fn composite_material_is_two_sha256_digests() {
        let m = composite_material(b"pw", b"keyfile-bytes");
        assert_eq!(m.len(), 64);
        // Order matters and is stable: password digest first, keyfile digest second.
        let other = composite_material(b"keyfile-bytes", b"pw");
        assert_ne!(m, other);
    }

    #[test]
    fn composite_changes_when_either_input_changes() {
        let base = composite_material(b"pw", b"kf");
        assert_ne!(*base, *composite_material(b"pw2", b"kf"));
        assert_ne!(*base, *composite_material(b"pw", b"kf2"));
    }

    #[test]
    fn composite_material_feeds_derive_key() {
        // The end-to-end contract for Task 2: composite_material output is a
        // valid `derive_key` input, and changing an input changes the key.
        let salt = [3u8; 16];
        let m = composite_material(b"pw", b"kf");
        let k = derive_key(&m, &salt, &fast()).unwrap();
        assert_eq!(k.len(), 32);
        let m2 = composite_material(b"pw", b"kf2");
        let k2 = derive_key(&m2, &salt, &fast()).unwrap();
        assert_ne!(*k, *k2);
    }
}
