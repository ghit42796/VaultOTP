pub mod kdf;
pub mod crypto;

use crate::error::{AppError, Result};
use crate::model::Account;
use crypto::{decrypt_with_derived_key, encrypt, Mode, VaultHeader};
use kdf::{composite_material, derive_key, KdfParams};
use zeroize::{Zeroize, Zeroizing};

/// A credential combination presented to unlock or re-key a vault.
pub enum Credential<'a> {
    Password(&'a [u8]),
    KeyFile(&'a [u8]),
    Both { password: &'a [u8], keyfile: &'a [u8] },
}

impl Credential<'_> {
    pub fn mode(&self) -> Mode {
        match self {
            Credential::Password(_) => Mode::Password,
            Credential::KeyFile(_) => Mode::Keyfile,
            Credential::Both { .. } => Mode::Composite,
        }
    }

    /// KDF input material for this credential. Zeroized on drop.
    fn material(&self) -> Zeroizing<Vec<u8>> {
        match self {
            Credential::Password(p) => Zeroizing::new(p.to_vec()),
            Credential::KeyFile(k) => Zeroizing::new(k.to_vec()),
            Credential::Both { password, keyfile } => composite_material(password, keyfile),
        }
    }
}

/// Read a vault file's `mode` without unlocking it (header is plaintext).
pub fn peek_mode(file_bytes: &[u8]) -> Result<Mode> {
    let (header, _) = crypto::parse_header(file_bytes)?;
    Ok(header.mode)
}

struct Unlocked {
    key: [u8; 32],
    accounts: Vec<Account>,
    kdf: KdfParams,
    salt: [u8; 16],
    mode: Mode,
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

    /// Pure: derive key from injected salt + credential, start with empty accounts. No I/O.
    pub fn create_unlocked(cred: Credential, salt: [u8; 16], kdf: KdfParams) -> Result<Vault> {
        let key: [u8; 32] = *derive_key(&cred.material(), &salt, &kdf)?;
        Ok(Vault { state: Some(Unlocked { key, accounts: Vec::new(), kdf, salt, mode: cred.mode() }) })
    }

    /// Pure: decrypt + parse from injected bytes using the given credential. No I/O.
    pub fn unlock_from_bytes(file_bytes: &[u8], cred: Credential) -> Result<Vault> {
        let (header, _) = crypto::parse_header(file_bytes)?;
        let key = derive_key(&cred.material(), &header.salt, &header.kdf)?;
        let plaintext = decrypt_with_derived_key(&key, file_bytes)?;
        let accounts: Vec<Account> =
            serde_json::from_slice(&plaintext).map_err(|_| AppError::Crypto)?;
        Ok(Vault { state: Some(Unlocked { key: *key, accounts, kdf: header.kdf, salt: header.salt, mode: header.mode }) })
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

    /// Pure: rearrange `accounts` to match `ids`, which must be a permutation of the
    /// current account ids. Locked -> Err(Crypto). Non-permutation -> Err(Other), unchanged.
    pub fn reorder(&mut self, ids: &[String]) -> Result<()> {
        let u = self.state.as_mut().ok_or(AppError::Crypto)?;
        let is_permutation = {
            let current: std::collections::HashSet<&str> = u.accounts.iter().map(|a| a.id.as_str()).collect();
            let requested: std::collections::HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();
            ids.len() == u.accounts.len() && requested.len() == ids.len() && requested == current
        };
        if !is_permutation {
            return Err(AppError::Other("invalid account order".into()));
        }
        let mut map: std::collections::HashMap<String, Account> =
            u.accounts.drain(..).map(|a| (a.id.clone(), a)).collect();
        u.accounts = ids.iter().map(|id| map.remove(id).expect("validated permutation")).collect();
        Ok(())
    }

    /// Pure: encrypt current accounts with the stored key + injected nonce. No I/O.
    pub fn serialize(&self, nonce: [u8; 12]) -> Result<Vec<u8>> {
        let u = self.state.as_ref().ok_or(AppError::Crypto)?;
        let header = VaultHeader { version: 1, mode: u.mode, kdf: u.kdf, salt: u.salt, nonce };
        let plaintext = serde_json::to_vec(&u.accounts).map_err(|_| AppError::Crypto)?;
        encrypt(&u.key, &header, &plaintext)
    }

    // Symmetric in-memory accessor (peek_mode reads the file); kept for future callers.
    #[allow(dead_code)]
    /// Return the current credential mode. Err(AppError::Crypto) if the vault is locked.
    pub fn mode(&self) -> Result<Mode> {
        self.state.as_ref().map(|u| u.mode).ok_or(AppError::Crypto)
    }

    /// Returns Ok iff `cred` re-derives the current content key.
    /// Use before a destructive `rekey` so a mistyped current credential cannot lock the user out.
    pub fn verify_current(&self, cred: Credential) -> Result<()> {
        let u = self.state.as_ref().ok_or(AppError::Crypto)?;
        let derived: [u8; 32] = *derive_key(&cred.material(), &u.salt, &u.kdf)?;
        let ok = derived == u.key;
        let mut derived = derived;
        derived.zeroize();
        if ok { Ok(()) } else { Err(AppError::Crypto) }
    }

    /// Switch the content key + mode to a new credential (re-derived from injected salt/kdf).
    /// The next `serialize` re-encrypts the accounts under the new key.
    pub fn rekey(&mut self, new: Credential, salt: [u8; 16], kdf: KdfParams) -> Result<()> {
        // Check state first so we don't derive an unnecessary key if the vault is locked.
        let u = self.state.as_mut().ok_or(AppError::Crypto)?;
        let new_mode = new.mode();
        let mut new_key: Zeroizing<[u8; 32]> = derive_key(&new.material(), &salt, &kdf)?;
        u.key.zeroize();
        u.key = *new_key;
        new_key.zeroize();
        u.salt = salt;
        u.kdf = kdf;
        u.mode = new_mode;
        Ok(())
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
        let v0 = Vault::create_unlocked(Credential::Password(b"pw"), [9u8; 16], fast_kdf()).unwrap();
        assert!(v0.is_unlocked());
        let mut v = v0;
        v.add(acc("id1")).unwrap();
        let bytes = v.serialize([3u8; 12]).unwrap();
        let v2 = Vault::unlock_from_bytes(&bytes, Credential::Password(b"pw")).unwrap();
        assert_eq!(v2.accounts().unwrap().len(), 1);
        assert_eq!(v2.accounts().unwrap()[0].issuer, "GitHub");
        assert_eq!(v2.accounts().unwrap()[0].id, "id1");
    }

    #[test]
    fn wrong_password_fails() {
        let v = Vault::create_unlocked(Credential::Password(b"pw"), [9u8; 16], fast_kdf()).unwrap();
        let bytes = v.serialize([3u8; 12]).unwrap();
        assert!(matches!(Vault::unlock_from_bytes(&bytes, Credential::Password(b"nope")), Err(AppError::Crypto)));
    }

    #[test]
    fn tamper_fails() {
        let v = Vault::create_unlocked(Credential::Password(b"pw"), [9u8; 16], fast_kdf()).unwrap();
        let mut bytes = v.serialize([3u8; 12]).unwrap();
        let n = bytes.len() - 1;
        bytes[n] ^= 0x01;
        assert!(matches!(Vault::unlock_from_bytes(&bytes, Credential::Password(b"pw")), Err(AppError::Crypto)));
    }

    #[test]
    fn remove_in_memory() {
        let mut v = Vault::create_unlocked(Credential::Password(b"pw"), [1u8; 16], fast_kdf()).unwrap();
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
        assert!(v.mode().is_err());
        assert!(v.verify_current(Credential::Password(b"pw")).is_err());
        assert!(v.rekey(Credential::Password(b"pw"), [0u8; 16], fast_kdf()).is_err());
    }

    #[test]
    fn password_mode_round_trips() {
        let v = Vault::create_unlocked(Credential::Password(b"pw"), [9u8; 16], fast_kdf()).unwrap();
        let bytes = v.serialize([3u8; 12]).unwrap();
        let v2 = Vault::unlock_from_bytes(&bytes, Credential::Password(b"pw")).unwrap();
        assert!(v2.is_unlocked());
        assert_eq!(peek_mode(&bytes).unwrap(), crate::vault::crypto::Mode::Password);
    }

    #[test]
    fn keyfile_mode_round_trips() {
        let v = Vault::create_unlocked(Credential::KeyFile(b"\x00\x01\x02keyfile"), [1u8; 16], fast_kdf()).unwrap();
        let bytes = v.serialize([4u8; 12]).unwrap();
        assert_eq!(peek_mode(&bytes).unwrap(), crate::vault::crypto::Mode::Keyfile);
        assert!(Vault::unlock_from_bytes(&bytes, Credential::KeyFile(b"\x00\x01\x02keyfile")).is_ok());
        assert!(matches!(Vault::unlock_from_bytes(&bytes, Credential::KeyFile(b"wrong")), Err(AppError::Crypto)));
    }

    #[test]
    fn composite_mode_needs_both() {
        let cred = Credential::Both { password: b"pw", keyfile: b"kf-bytes" };
        let v = Vault::create_unlocked(cred, [2u8; 16], fast_kdf()).unwrap();
        let bytes = v.serialize([5u8; 12]).unwrap();
        assert_eq!(peek_mode(&bytes).unwrap(), crate::vault::crypto::Mode::Composite);
        assert!(Vault::unlock_from_bytes(&bytes, Credential::Both { password: b"pw", keyfile: b"kf-bytes" }).is_ok());
        // Password alone (wrong mode/material) must fail.
        assert!(matches!(Vault::unlock_from_bytes(&bytes, Credential::Password(b"pw")), Err(AppError::Crypto)));
    }

    #[test]
    fn verify_current_matches_only_correct_credential() {
        let v = Vault::create_unlocked(Credential::Password(b"pw"), [9u8; 16], fast_kdf()).unwrap();
        assert!(v.verify_current(Credential::Password(b"pw")).is_ok());
        assert!(matches!(v.verify_current(Credential::Password(b"nope")), Err(AppError::Crypto)));
    }

    #[test]
    fn rekey_password_to_composite_then_unlock_with_both() {
        let mut v = Vault::create_unlocked(Credential::Password(b"pw"), [9u8; 16], fast_kdf()).unwrap();
        v.add(acc("id1")).unwrap();
        v.rekey(Credential::Both { password: b"pw", keyfile: b"kf" }, [7u8; 16], fast_kdf()).unwrap();
        assert_eq!(v.mode().unwrap(), crate::vault::crypto::Mode::Composite);
        let bytes = v.serialize([3u8; 12]).unwrap();
        // Old password-only credential no longer opens it.
        assert!(matches!(Vault::unlock_from_bytes(&bytes, Credential::Password(b"pw")), Err(AppError::Crypto)));
        // Both works and data survived.
        let v2 = Vault::unlock_from_bytes(&bytes, Credential::Both { password: b"pw", keyfile: b"kf" }).unwrap();
        assert_eq!(v2.accounts().unwrap().len(), 1);
    }

    #[test]
    fn reorder_to_permutation() {
        let mut v = Vault::create_unlocked(Credential::Password(b"pw"), [9u8; 16], fast_kdf()).unwrap();
        v.add(acc("a")).unwrap(); v.add(acc("b")).unwrap(); v.add(acc("c")).unwrap();
        v.reorder(&["c".to_string(), "a".to_string(), "b".to_string()]).unwrap();
        let ids: Vec<&str> = v.accounts().unwrap().iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, vec!["c", "a", "b"]);
    }

    #[test]
    fn reorder_rejects_non_permutation_and_leaves_unchanged() {
        let mut v = Vault::create_unlocked(Credential::Password(b"pw"), [9u8; 16], fast_kdf()).unwrap();
        v.add(acc("a")).unwrap(); v.add(acc("b")).unwrap();
        assert!(matches!(v.reorder(&["a".to_string()]), Err(AppError::Other(_))));                          // missing
        assert!(matches!(v.reorder(&["a".to_string(), "b".to_string(), "x".to_string()]), Err(AppError::Other(_)))); // extra
        assert!(matches!(v.reorder(&["a".to_string(), "a".to_string()]), Err(AppError::Other(_))));         // duplicate
        let ids: Vec<&str> = v.accounts().unwrap().iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b"]); // unchanged
    }

    #[test]
    fn reorder_locked_errors() {
        let mut v = Vault::new();
        assert!(matches!(v.reorder(&["a".to_string()]), Err(AppError::Crypto)));
    }
}
