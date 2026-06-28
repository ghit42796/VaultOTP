use crate::error::Result;
use crate::model::Account;
use crate::totp;
use crate::vault::Vault;
use crate::vault::kdf::KdfParams;
use rand::RngCore;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub vault: Mutex<Vault>,
    pub path: PathBuf,
}

#[derive(Serialize, Clone)]
pub struct AccountView {
    pub id: String,
    pub issuer: String,
    pub label: String,
}

#[derive(Serialize, Clone)]
pub struct CodeView {
    pub id: String,
    pub issuer: String,
    pub label: String,
    pub code: String,
    pub remaining: u64,
}

pub fn to_view(a: &Account) -> AccountView {
    AccountView { id: a.id.clone(), issuer: a.issuer.clone(), label: a.label.clone() }
}

pub fn to_code_view(a: &Account, now: u64) -> Result<CodeView> {
    Ok(CodeView {
        id: a.id.clone(),
        issuer: a.issuer.clone(),
        label: a.label.clone(),
        code: totp::generate(a, now)?,
        remaining: totp::remaining_seconds(a, now),
    })
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn random_salt() -> [u8; 16] {
    let mut s = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut s);
    s
}

fn random_nonce() -> [u8; 12] {
    let mut n = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut n);
    n
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[tauri::command]
pub fn vault_exists(state: tauri::State<AppState>) -> bool {
    crate::storage::exists(&state.path)
}

#[tauri::command]
pub fn create_vault(state: tauri::State<AppState>, password: String) -> Result<()> {
    let v = Vault::create_unlocked(password.as_bytes(), random_salt(), KdfParams::default())?;
    let bytes = v.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.path, &bytes)?;
    *state.vault.lock().unwrap() = v;
    Ok(())
}

#[tauri::command]
pub fn unlock(state: tauri::State<AppState>, password: String) -> Result<()> {
    let bytes = crate::storage::read_file(&state.path)?;
    let v = Vault::unlock_from_bytes(&bytes, password.as_bytes())?;
    *state.vault.lock().unwrap() = v;
    Ok(())
}

#[tauri::command]
pub fn lock(state: tauri::State<AppState>) -> Result<()> {
    state.vault.lock().unwrap().lock();
    Ok(())
}

#[tauri::command]
pub fn is_unlocked(state: tauri::State<AppState>) -> bool {
    state.vault.lock().unwrap().is_unlocked()
}

#[tauri::command]
pub fn list_accounts(state: tauri::State<AppState>) -> Result<Vec<AccountView>> {
    let v = state.vault.lock().unwrap();
    Ok(v.accounts()?.iter().map(to_view).collect())
}

#[tauri::command]
pub fn current_codes(state: tauri::State<AppState>) -> Result<Vec<CodeView>> {
    let now = now_unix();
    let v = state.vault.lock().unwrap();
    v.accounts()?.iter().map(|a| to_code_view(a, now)).collect()
}

#[tauri::command]
pub fn add_manual(state: tauri::State<AppState>, issuer: String, label: String, secret: String) -> Result<()> {
    crate::secret::decode_secret(&secret)?;
    let mut acc = Account::new(issuer, label, secret);
    acc.id = new_id();
    let mut g = state.vault.lock().unwrap();
    g.add(acc)?;
    let bytes = g.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.path, &bytes)
}

#[tauri::command]
pub fn add_from_uri(state: tauri::State<AppState>, uri: String) -> Result<()> {
    let mut acc = crate::otpauth::parse_otpauth(&uri)?;
    acc.id = new_id();
    let mut g = state.vault.lock().unwrap();
    g.add(acc)?;
    let bytes = g.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.path, &bytes)
}

#[tauri::command]
pub fn decode_qr_file(path: String) -> Result<Vec<String>> {
    let bytes = crate::storage::read_file(std::path::Path::new(&path))?;
    let strings = crate::qr::decode_image_bytes(&bytes)?;
    Ok(strings.into_iter().filter(|s| s.starts_with("otpauth")).collect())
}

#[tauri::command]
pub fn preview_migration(uri: String) -> Result<Vec<AccountView>> {
    Ok(crate::migration::parse_migration(&uri)?.iter().map(to_view).collect())
}

#[tauri::command]
pub fn import_migration(state: tauri::State<AppState>, uri: String, selected_indices: Vec<usize>) -> Result<usize> {
    let accounts = crate::migration::parse_migration(&uri)?;
    let mut g = state.vault.lock().unwrap();
    let mut count = 0;
    for (i, acc) in accounts.into_iter().enumerate() {
        if selected_indices.contains(&i) {
            let mut acc = acc;
            acc.id = new_id();
            g.add(acc)?;
            count += 1;
        }
    }
    let bytes = g.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.path, &bytes)?;
    Ok(count)
}

#[tauri::command]
pub fn remove_account(state: tauri::State<AppState>, id: String) -> Result<()> {
    let mut g = state.vault.lock().unwrap();
    g.remove(&id)?;
    let bytes = g.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.path, &bytes)
}

#[tauri::command]
pub fn export_backup(state: tauri::State<AppState>, path: String, password: String) -> Result<()> {
    let accounts = state.vault.lock().unwrap().snapshot()?;
    let bytes = crate::backup::export_encrypted(&accounts, password.as_bytes(), random_salt(), random_nonce())?;
    crate::storage::write_atomic(std::path::Path::new(&path), &bytes)
}

#[tauri::command]
pub fn import_backup(state: tauri::State<AppState>, path: String, password: String) -> Result<usize> {
    let bytes = crate::storage::read_file(std::path::Path::new(&path))?;
    let accounts = crate::backup::import_encrypted(&bytes, password.as_bytes())?;
    let mut g = state.vault.lock().unwrap();
    let n = accounts.len();
    // vault mutex held for full loop + serialize + write; no partial-write possible
    for acc in accounts {
        g.add(acc)?; // backup accounts already carry ids (no dedup check)
    }
    let out = g.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.path, &out)?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_view_has_no_secret_field_and_valid_code() {
        let a = Account::new("GitHub".into(), "alice".into(), "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".into());
        let cv = to_code_view(&a, 59).unwrap();
        assert_eq!(cv.code.len(), 6);
        // serialize and confirm 'secret' is absent
        let json = serde_json::to_string(&cv).unwrap();
        assert!(!json.contains("secret"));
        assert!(json.contains("\"code\""));
    }

    #[test]
    fn decode_qr_file_reads_fixture_and_filters_otpauth() {
        // cwd for `cargo test` is the crate root (src-tauri/); fixture path is relative to it.
        let out = decode_qr_file("tests/fixtures/qr_otpauth.png".into()).unwrap();
        assert!(out.iter().any(|s| s.starts_with("otpauth://totp/")));
    }

    #[test]
    fn decode_qr_file_missing_path_errors() {
        // storage::read_file fails for a non-existent path; error propagates.
        assert!(decode_qr_file("tests/fixtures/does_not_exist.png".into()).is_err());
    }
}
