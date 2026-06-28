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

/// Composite key (KeePass-style): SHA-256(password) then Argon2id.
///
/// # Salt
/// Callers MUST supply a cryptographically random 16-byte salt, unique per
/// vault file, and never reused across vaults.
///
/// # Returns
/// A `Zeroizing<[u8; 32]>` that clears itself when dropped.
pub fn derive_key(
    password: &[u8],
    salt: &[u8; 16],
    params: &KdfParams,
) -> Result<Zeroizing<[u8; 32]>> {
    use sha2::{Digest, Sha256};

    // Step 1: Pre-hash password to a fixed-size input (KeePass composite key pattern).
    let mut composite = Sha256::digest(password);

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
}
