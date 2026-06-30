use crate::error::{AppError, Result};
use crate::model::Account;
use crate::vault::crypto::{decrypt, encrypt, Mode, VaultHeader};
use crate::vault::kdf::{derive_key, KdfParams};

/// Pure: encrypt accounts into a standalone backup blob. salt + nonce injected.
pub fn export_encrypted(
    accounts: &[Account],
    password: &[u8],
    salt: [u8; 16],
    nonce: [u8; 12],
) -> Result<Vec<u8>> {
    let kdf = KdfParams::default();
    let key: [u8; 32] = *derive_key(password, &salt, &kdf)?;
    // Backup blobs are always password-protected by design (export takes a `password`,
    // never a key file), so `Mode::Password` here is a permanent invariant, not a placeholder.
    let header = VaultHeader { version: 1, mode: Mode::Password, kdf, salt, nonce };
    let plaintext = serde_json::to_vec(accounts).map_err(|_| AppError::Crypto)?;
    encrypt(&key, &header, &plaintext)
}

/// Pure: decrypt a backup blob to accounts. bytes injected.
pub fn import_encrypted(file_bytes: &[u8], password: &[u8]) -> Result<Vec<Account>> {
    let plaintext = decrypt(password, file_bytes)?;
    serde_json::from_slice(&plaintext).map_err(|_| AppError::Crypto)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Account;

    #[test]
    fn export_import_round_trips() {
        let mut a = Account::new("GitHub".into(), "alice".into(), "JBSWY3DPEHPK3PXP".into());
        a.id = "id1".into();
        let accounts = vec![a];
        let bytes = export_encrypted(&accounts, b"backup-pw", [7u8; 16], [2u8; 12]).unwrap();
        let restored = import_encrypted(&bytes, b"backup-pw").unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].issuer, "GitHub");
        assert_eq!(restored[0].label, "alice");
        assert_eq!(restored[0].id, "id1");
        assert!(matches!(import_encrypted(&bytes, b"wrong"), Err(AppError::Crypto)));
    }
}
