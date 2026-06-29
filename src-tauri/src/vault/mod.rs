pub mod kdf;
pub mod crypto;

use crate::error::{AppError, Result};
use crate::model::Account;
use crypto::{decrypt_with_key, encrypt, VaultHeader};
use kdf::{derive_key, KdfParams};
use zeroize::Zeroize;

struct Unlocked {
    key: [u8; 32],
    accounts: Vec<Account>,
    kdf: KdfParams,
    salt: [u8; 16],
}

impl Drop for Unlocked {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

pub struct Vault {
    state: Option<Unlocked>,
}

impl Vault {
    pub fn new() -> Self {
        Vault { state: None }
    }

    pub fn is_unlocked(&self) -> bool {
        self.state.is_some()
    }

    /// Pure: derive key from injected salt, start with empty accounts. No I/O.
    pub fn create_unlocked(password: &[u8], salt: [u8; 16], kdf: KdfParams) -> Result<Vault> {
        let key: [u8; 32] = *derive_key(password, &salt, &kdf)?;
        Ok(Vault { state: Some(Unlocked { key, accounts: Vec::new(), kdf, salt }) })
    }

    /// Pure: decrypt + parse from injected bytes. No I/O.
    pub fn unlock_from_bytes(file_bytes: &[u8], password: &[u8]) -> Result<Vault> {
        let (plaintext, key) = decrypt_with_key(password, file_bytes)?;
        let accounts: Vec<Account> =
            serde_json::from_slice(&plaintext).map_err(|_| AppError::Crypto)?;
        let (header, _) = crypto::parse_header(file_bytes)?;
        Ok(Vault { state: Some(Unlocked { key: *key, accounts, kdf: header.kdf, salt: header.salt }) })
    }

    pub fn lock(&mut self) {
        self.state = None;
    }

    pub fn accounts(&self) -> Result<&[Account]> {
        self.state.as_ref().map(|u| u.accounts.as_slice()).ok_or(AppError::Crypto)
    }

    pub fn snapshot(&self) -> Result<Vec<Account>> {
        self.state.as_ref().map(|u| u.accounts.clone()).ok_or(AppError::Crypto)
    }

    pub fn add(&mut self, account: Account) -> Result<()> {
        let u = self.state.as_mut().ok_or(AppError::Crypto)?;
        u.accounts.push(account);
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> Result<()> {
        let u = self.state.as_mut().ok_or(AppError::Crypto)?;
        u.accounts.retain(|a| a.id != id);
        Ok(())
    }

    /// Pure: encrypt current accounts with the stored key + injected nonce. No I/O.
    pub fn serialize(&self, nonce: [u8; 12]) -> Result<Vec<u8>> {
        let u = self.state.as_ref().ok_or(AppError::Crypto)?;
        let header = VaultHeader { version: 1, kdf: u.kdf, salt: u.salt, nonce };
        let plaintext = serde_json::to_vec(&u.accounts).map_err(|_| AppError::Crypto)?;
        encrypt(&u.key, &header, &plaintext)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Account;
    use crate::vault::kdf::KdfParams;

    fn fast_kdf() -> KdfParams { KdfParams { mem_kib: 8192, iterations: 1, parallelism: 1 } }
    fn acc(id: &str) -> Account {
        let mut a = Account::new("GitHub".into(), "alice".into(), "JBSWY3DPEHPK3PXP".into());
        a.id = id.to_string();
        a
    }

    #[test]
    fn create_serialize_unlock_cycle() {
        let v0 = Vault::create_unlocked(b"pw", [9u8; 16], fast_kdf()).unwrap();
        assert!(v0.is_unlocked());
        let mut v = v0;
        v.add(acc("id1")).unwrap();
        let bytes = v.serialize([3u8; 12]).unwrap();
        let v2 = Vault::unlock_from_bytes(&bytes, b"pw").unwrap();
        assert_eq!(v2.accounts().unwrap().len(), 1);
        assert_eq!(v2.accounts().unwrap()[0].issuer, "GitHub");
        assert_eq!(v2.accounts().unwrap()[0].id, "id1");
    }

    #[test]
    fn wrong_password_fails() {
        let v = Vault::create_unlocked(b"pw", [9u8; 16], fast_kdf()).unwrap();
        let bytes = v.serialize([3u8; 12]).unwrap();
        assert!(matches!(Vault::unlock_from_bytes(&bytes, b"nope"), Err(AppError::Crypto)));
    }

    #[test]
    fn tamper_fails() {
        let v = Vault::create_unlocked(b"pw", [9u8; 16], fast_kdf()).unwrap();
        let mut bytes = v.serialize([3u8; 12]).unwrap();
        let n = bytes.len() - 1;
        bytes[n] ^= 0x01;
        assert!(matches!(Vault::unlock_from_bytes(&bytes, b"pw"), Err(AppError::Crypto)));
    }

    #[test]
    fn remove_in_memory() {
        let mut v = Vault::create_unlocked(b"pw", [1u8; 16], fast_kdf()).unwrap();
        v.add(acc("x")).unwrap();
        v.remove("x").unwrap();
        assert_eq!(v.accounts().unwrap().len(), 0);
    }

    #[test]
    fn locked_ops_error() {
        let mut v = Vault::new();
        assert!(!v.is_unlocked());
        assert!(v.accounts().is_err());
        assert!(v.snapshot().is_err());
        assert!(v.add(acc("x")).is_err());
        assert!(v.remove("x").is_err());
        assert!(v.serialize([0u8; 12]).is_err());
    }
}
