use crate::error::Result;
use crate::model::Account;
use crate::totp;
use crate::vault::{Credential, Vault};
use crate::vault::crypto::Mode;
use crate::vault::kdf::KdfParams;
use rand::RngCore;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub vault: Mutex<Vault>,
    /// Path of the vault currently being worked with. Repointed by `set_current_vault`.
    pub current: Mutex<PathBuf>,
    /// Directory holding `config.json` (the app config dir).
    pub config_dir: PathBuf,
}

impl AppState {
    /// The active vault path (cloned out of the mutex).
    pub fn current_path(&self) -> PathBuf {
        self.current.lock().unwrap().clone()
    }
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

fn read_keyfile(path: &str) -> Result<Vec<u8>> {
    crate::storage::read_file(std::path::Path::new(path))
}

fn mode_str(m: Mode) -> &'static str {
    match m {
        Mode::Password => "password",
        Mode::Keyfile => "keyfile",
        Mode::Composite => "composite",
    }
}

fn parse_mode(s: &str) -> Result<Mode> {
    match s {
        "password" => Ok(Mode::Password),
        "keyfile" => Ok(Mode::Keyfile),
        "composite" => Ok(Mode::Composite),
        _ => Err(crate::error::AppError::Crypto),
    }
}

#[tauri::command]
pub fn vault_exists(state: tauri::State<AppState>) -> bool {
    crate::storage::exists(&state.current_path())
}

#[tauri::command]
pub fn create_vault(
    state: tauri::State<AppState>,
    mode: String,
    password: Option<String>,
    keyfile_path: Option<String>,
) -> Result<()> {
    let kf = match keyfile_path { Some(p) => Some(read_keyfile(&p)?), None => None };
    let cred = match (parse_mode(&mode)?, password.as_deref(), kf.as_deref()) {
        (Mode::Password, Some(p), _) => Credential::Password(p.as_bytes()),
        (Mode::Keyfile, _, Some(k)) => Credential::KeyFile(k),
        (Mode::Composite, Some(p), Some(k)) => Credential::Both { password: p.as_bytes(), keyfile: k },
        _ => return Err(crate::error::AppError::Crypto),
    };
    let v = Vault::create_unlocked(cred, random_salt(), KdfParams::default())?;
    let bytes = v.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.current_path(), &bytes)?;
    *state.vault.lock().unwrap() = v;
    Ok(())
}

#[tauri::command]
pub fn unlock(
    state: tauri::State<AppState>,
    password: Option<String>,
    keyfile_path: Option<String>,
) -> Result<()> {
    let bytes = crate::storage::read_file(&state.current_path())?;
    let mode = crate::vault::peek_mode(&bytes)?;
    let kf = match keyfile_path { Some(p) => Some(read_keyfile(&p)?), None => None };
    let cred = match (mode, password.as_deref(), kf.as_deref()) {
        (Mode::Password, Some(p), _) => Credential::Password(p.as_bytes()),
        (Mode::Keyfile, _, Some(k)) => Credential::KeyFile(k),
        (Mode::Composite, Some(p), Some(k)) => Credential::Both { password: p.as_bytes(), keyfile: k },
        _ => return Err(crate::error::AppError::Crypto),
    };
    let v = Vault::unlock_from_bytes(&bytes, cred)?;
    *state.vault.lock().unwrap() = v;
    Ok(())
}

#[tauri::command]
pub fn vault_mode(state: tauri::State<AppState>) -> Result<String> {
    let bytes = crate::storage::read_file(&state.current_path())?;
    Ok(mode_str(crate::vault::peek_mode(&bytes)?).to_string())
}

#[tauri::command]
pub fn generate_keyfile(out_path: String) -> Result<()> {
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    crate::storage::write_atomic(std::path::Path::new(&out_path), &buf)
}

#[tauri::command]
pub fn add_keyfile(state: tauri::State<AppState>, password: String, keyfile_path: String) -> Result<()> {
    let kf = read_keyfile(&keyfile_path)?;
    let mut g = state.vault.lock().unwrap();
    // Vault is currently password-only; confirm the typed password before re-keying.
    g.verify_current(Credential::Password(password.as_bytes()))?;
    g.rekey(Credential::Both { password: password.as_bytes(), keyfile: &kf }, random_salt(), KdfParams::default())?;
    let bytes = g.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.current_path(), &bytes)
}

#[tauri::command]
pub fn remove_keyfile(state: tauri::State<AppState>, password: String, keyfile_path: String) -> Result<()> {
    let kf = read_keyfile(&keyfile_path)?;
    let mut g = state.vault.lock().unwrap();
    // Confirm the current composite credential, then drop to password-only.
    g.verify_current(Credential::Both { password: password.as_bytes(), keyfile: &kf })?;
    g.rekey(Credential::Password(password.as_bytes()), random_salt(), KdfParams::default())?;
    let bytes = g.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.current_path(), &bytes)
}

#[tauri::command]
pub fn change_password(
    state: tauri::State<AppState>,
    current_password: String,
    current_keyfile_path: Option<String>,
    new_password: String,
    new_keyfile_path: Option<String>,
) -> Result<()> {
    let cur_kf = match current_keyfile_path { Some(p) => Some(read_keyfile(&p)?), None => None };
    let new_kf = match new_keyfile_path { Some(p) => Some(read_keyfile(&p)?), None => None };
    let mut g = state.vault.lock().unwrap();
    let current = match cur_kf.as_deref() {
        Some(k) => Credential::Both { password: current_password.as_bytes(), keyfile: k },
        None => Credential::Password(current_password.as_bytes()),
    };
    g.verify_current(current)?;
    let new = match new_kf.as_deref() {
        Some(k) => Credential::Both { password: new_password.as_bytes(), keyfile: k },
        None => Credential::Password(new_password.as_bytes()),
    };
    g.rekey(new, random_salt(), KdfParams::default())?;
    let bytes = g.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.current_path(), &bytes)
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
    crate::storage::write_atomic(&state.current_path(), &bytes)
}

#[tauri::command]
pub fn add_from_uri(state: tauri::State<AppState>, uri: String) -> Result<()> {
    let mut acc = crate::otpauth::parse_otpauth(&uri)?;
    acc.id = new_id();
    let mut g = state.vault.lock().unwrap();
    g.add(acc)?;
    let bytes = g.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.current_path(), &bytes)
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
    crate::storage::write_atomic(&state.current_path(), &bytes)?;
    Ok(count)
}

#[tauri::command]
pub fn remove_account(state: tauri::State<AppState>, id: String) -> Result<()> {
    let mut g = state.vault.lock().unwrap();
    g.remove(&id)?;
    let bytes = g.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.current_path(), &bytes)
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
    crate::storage::write_atomic(&state.current_path(), &out)?;
    Ok(n)
}

/// Shared logic for `save_vault_as`, testable without a Tauri `State` wrapper.
/// Serializes the in-memory (unlocked) vault and writes it to `path`.
/// Returns `Err(AppError::Crypto)` if the vault is locked.
/// Does NOT change `state.current` or touch the recent list.
fn save_vault_as_inner(state: &AppState, path: String) -> Result<()> {
    let g = state.vault.lock().unwrap();
    let bytes = g.serialize(random_nonce())?; // Err(Crypto) if locked
    crate::storage::write_atomic(std::path::Path::new(&path), &bytes)
}

#[tauri::command]
pub fn save_vault_as(state: tauri::State<AppState>, path: String) -> Result<()> {
    save_vault_as_inner(&state, path)
}

/// Map a string to path-safe chars: ASCII alnum/-/_ are kept, everything else becomes '_'.
fn clean_part(part: &str) -> String {
    part.chars().map(|ch| {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' { ch } else { '_' }
    }).collect()
}

/// Pure: a filesystem-safe base name from issuer+label (ASCII alnum/-/_ kept, others → '_').
fn sanitize_filename(issuer: &str, label: &str) -> String {
    let mut s = clean_part(issuer);
    if !issuer.is_empty() && !label.is_empty() {
        s.push('-');
    }
    s.push_str(&clean_part(label));
    let trimmed = s.trim_matches('_').to_string();
    if trimmed.is_empty() { "account".to_string() } else { trimmed }
}

/// Shared logic for `export_secrets`, testable without a Tauri `State` wrapper.
fn export_secrets_inner(state: &AppState, ids: Vec<String>, path: String, format: String) -> Result<usize> {
    let all = state.vault.lock().unwrap().snapshot()?; // Err(Crypto) if locked
    let selected: Vec<Account> = all.into_iter().filter(|a| ids.contains(&a.id)).collect();
    if selected.is_empty() {
        return Err(crate::error::AppError::Other("no accounts selected".into()));
    }
    match format.as_str() {
        "otpauth_text" => {
            let body = selected.iter().map(crate::otpauth::build_otpauth).collect::<Vec<_>>().join("\n");
            crate::storage::write_atomic(std::path::Path::new(&path), body.as_bytes())?;
            Ok(selected.len())
        }
        "google_migration" => {
            let uri = crate::migration::build_migration(&selected)?;
            let png = crate::qr::encode_png(&uri).map_err(|_| crate::error::AppError::Other("Too many accounts for a single migration QR; export fewer".into()))?;
            crate::storage::write_atomic(std::path::Path::new(&path), &png)?;
            Ok(selected.len())
        }
        "otpauth_qr" => {
            // `path` is a directory; one PNG per account, de-duplicating base names.
            let dir = std::path::Path::new(&path);
            let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
            for a in &selected {
                let base = sanitize_filename(&a.issuer, &a.label);
                let mut name = format!("{base}.png");
                let mut n = 1usize;
                while used.contains(&name) {
                    n += 1;
                    name = format!("{}-{}.png", sanitize_filename(&a.issuer, &a.label), n);
                }
                used.insert(name.clone());
                let uri = crate::otpauth::build_otpauth(a);
                let png = crate::qr::encode_png(&uri)?;
                crate::storage::write_atomic(&dir.join(&name), &png)?;
            }
            Ok(selected.len())
        }
        _ => Err(crate::error::AppError::Other("unknown export format".into())),
    }
}

#[tauri::command]
pub fn export_secrets(state: tauri::State<AppState>, ids: Vec<String>, path: String, format: String) -> Result<usize> {
    export_secrets_inner(&state, ids, path, format)
}

/// Shared logic for `reorder_accounts`, testable without a Tauri `State` wrapper.
/// Reorders the in-memory vault to match `ids` (must be a permutation), then persists.
fn reorder_accounts_inner(state: &AppState, ids: Vec<String>) -> Result<()> {
    let mut g = state.vault.lock().unwrap();
    g.reorder(&ids)?;
    let bytes = g.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.current_path(), &bytes)
}

#[tauri::command]
pub fn reorder_accounts(state: tauri::State<AppState>, ids: Vec<String>) -> Result<()> {
    reorder_accounts_inner(&state, ids)
}

/// Shared logic for `set_current_vault`, testable without a Tauri `State` wrapper.
fn set_current_vault_inner(state: &AppState, path: String) -> Result<()> {
    *state.current.lock().unwrap() = std::path::PathBuf::from(&path);
    let mut cfg = crate::config::load(&state.config_dir);
    cfg.touch(&path, 10);
    crate::config::save(&state.config_dir, &cfg)
}

#[tauri::command]
pub fn set_current_vault(state: tauri::State<AppState>, path: String) -> Result<()> {
    set_current_vault_inner(&state, path)
}

#[tauri::command]
pub fn list_recent_vaults(state: tauri::State<AppState>) -> Result<Vec<crate::config::RecentVaultView>> {
    let cfg = crate::config::load(&state.config_dir);
    Ok(crate::config::recent_views(&cfg.recent, |p| std::path::Path::new(p).exists()))
}

#[tauri::command]
pub fn current_vault_path(state: tauri::State<AppState>) -> String {
    state.current_path().to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_keyfile_writes_32_random_bytes() {
        let dir = std::env::temp_dir().join(format!("votp_kf_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("k.vaultkey");
        generate_keyfile(path.to_string_lossy().into_owned()).unwrap();
        let a = std::fs::read(&path).unwrap();
        assert_eq!(a.len(), 32);
        // Overwrite and confirm it differs (CSPRNG, not constant).
        generate_keyfile(path.to_string_lossy().into_owned()).unwrap();
        let b = std::fs::read(&path).unwrap();
        assert_ne!(a, b);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn mode_str_round_trips_with_parse() {
        assert_eq!(mode_str(crate::vault::crypto::Mode::Password), "password");
        assert_eq!(mode_str(crate::vault::crypto::Mode::Keyfile), "keyfile");
        assert_eq!(mode_str(crate::vault::crypto::Mode::Composite), "composite");
        assert!(matches!(parse_mode("password").unwrap(), crate::vault::crypto::Mode::Password));
        assert!(parse_mode("bogus").is_err());
    }

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

    #[test]
    fn app_state_current_path_clones_current() {
        let st = AppState {
            vault: Mutex::new(Vault::new()),
            current: Mutex::new(std::path::PathBuf::from("/vaults/a.bin")),
            config_dir: std::path::PathBuf::from("/cfg"),
        };
        assert_eq!(st.current_path(), std::path::PathBuf::from("/vaults/a.bin"));
        // mutating current is reflected
        *st.current.lock().unwrap() = std::path::PathBuf::from("/vaults/b.bin");
        assert_eq!(st.current_path(), std::path::PathBuf::from("/vaults/b.bin"));
    }

    #[test]
    fn set_current_vault_updates_state_and_recent() {
        let dir = std::env::temp_dir().join(format!("votp_sel_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let st = AppState {
            vault: Mutex::new(Vault::new()),
            current: Mutex::new(dir.join("old.bin")),
            config_dir: dir.clone(),
        };
        set_current_vault_inner(&st, dir.join("new.bin").to_string_lossy().into_owned()).unwrap();
        assert_eq!(st.current_path(), dir.join("new.bin"));
        let cfg = crate::config::load(&dir);
        assert_eq!(cfg.last.as_deref(), Some(dir.join("new.bin").to_string_lossy().as_ref()));
        assert_eq!(cfg.recent.len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_vault_as_errors_when_locked() {
        let st = AppState {
            vault: Mutex::new(Vault::new()), // locked
            current: Mutex::new(std::path::PathBuf::from("/x.bin")),
            config_dir: std::path::PathBuf::from("/cfg"),
        };
        // Locked vault cannot serialize → opaque error, nothing written.
        assert!(save_vault_as_inner(&st, "/tmp/should-not-exist-copy.bin".to_string()).is_err());
    }

    #[test]
    fn save_vault_as_writes_copy_without_changing_current() {
        let dir = std::env::temp_dir().join(format!("votp_saveas_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        // Build an unlocked vault directly.
        let v = Vault::create_unlocked(Credential::Password(b"pw"), [9u8; 16], KdfParams::default()).unwrap();
        let st = AppState {
            vault: Mutex::new(v),
            current: Mutex::new(dir.join("orig.bin")),
            config_dir: dir.clone(),
        };
        let copy = dir.join("copy.bin");
        save_vault_as_inner(&st, copy.to_string_lossy().into_owned()).unwrap();
        assert!(copy.exists());
        // current unchanged
        assert_eq!(st.current_path(), dir.join("orig.bin"));
        // copy opens with the same credential
        let bytes = std::fs::read(&copy).unwrap();
        assert!(Vault::unlock_from_bytes(&bytes, Credential::Password(b"pw")).is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn sanitize_filename_is_path_safe() {
        assert_eq!(sanitize_filename("Git/Hub", "a:b c"), "Git_Hub-a_b_c");
        assert_eq!(sanitize_filename("", ""), "account");
    }

    #[test]
    fn export_secrets_text_writes_selected_only() {
        let dir = std::env::temp_dir().join(format!("votp_exp_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        // Unlocked vault with two accounts.
        let mut v = Vault::create_unlocked(Credential::Password(b"pw"), [9u8; 16], KdfParams::default()).unwrap();
        let mut a1 = Account::new("GitHub".into(), "alice".into(), "JBSWY3DPEHPK3PXP".into());
        a1.id = "id1".into();
        let mut a2 = Account::new("AWS".into(), "bob".into(), "JBSWY3DPEHPK3PXP".into());
        a2.id = "id2".into();
        v.add(a1).unwrap(); v.add(a2).unwrap();
        let st = AppState { vault: Mutex::new(v), current: Mutex::new(dir.join("v.bin")), config_dir: dir.clone() };

        let out = dir.join("secrets.txt");
        let n = export_secrets_inner(&st, vec!["id1".into()], out.to_string_lossy().into_owned(), "otpauth_text".into()).unwrap();
        assert_eq!(n, 1);
        let text = std::fs::read_to_string(&out).unwrap();
        assert!(text.contains("otpauth://totp/") && text.contains("GitHub"));
        assert!(!text.contains("AWS")); // only the selected id was exported
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn export_secrets_errors_when_locked() {
        let st = AppState { vault: Mutex::new(Vault::new()), current: Mutex::new(std::path::PathBuf::from("/x")), config_dir: std::path::PathBuf::from("/c") };
        assert!(export_secrets_inner(&st, vec!["id1".into()], "/tmp/none.txt".into(), "otpauth_text".into()).is_err());
    }

    #[test]
    fn reorder_accounts_inner_persists_new_order() {
        let dir = std::env::temp_dir().join(format!("votp_reorder_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vault::create_unlocked(Credential::Password(b"pw"), [9u8; 16], KdfParams::default()).unwrap();
        for id in ["id1", "id2", "id3"] {
            let mut a = Account::new("Iss".into(), id.into(), "JBSWY3DPEHPK3PXP".into());
            a.id = id.into();
            v.add(a).unwrap();
        }
        let path = dir.join("v.bin");
        let st = AppState { vault: Mutex::new(v), current: Mutex::new(path.clone()), config_dir: dir.clone() };
        reorder_accounts_inner(&st, vec!["id3".into(), "id1".into(), "id2".into()]).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let v2 = Vault::unlock_from_bytes(&bytes, Credential::Password(b"pw")).unwrap();
        let ids: Vec<String> = v2.accounts().unwrap().iter().map(|a| a.id.clone()).collect();
        assert_eq!(ids, vec!["id3".to_string(), "id1".to_string(), "id2".to_string()]);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reorder_accounts_inner_errors_when_locked() {
        let st = AppState {
            vault: Mutex::new(Vault::new()),
            current: Mutex::new(std::path::PathBuf::from("/x")),
            config_dir: std::path::PathBuf::from("/c"),
        };
        assert!(reorder_accounts_inner(&st, vec!["id1".into()]).is_err());
    }
}
