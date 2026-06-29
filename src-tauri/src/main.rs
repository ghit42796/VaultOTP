#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backup;
mod commands;
mod error;
mod migration;
mod model;
mod otpauth;
mod qr;
mod secret;
mod session;
mod storage;
mod totp;
mod vault;

pub fn otpauth_decode(s: &str) -> String {
    // percent-decode (data is also base64 which has no %, but the data param itself is URL-encoded)
    let mut out = String::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = ((b[i+1] as char).to_digit(16), (b[i+2] as char).to_digit(16)) {
                out.push((h * 16 + l) as u8 as char);
                i += 3;
                continue;
            }
        }
        out.push(b[i] as char);
        i += 1;
    }
    out
}

fn vault_path(app: &tauri::App) -> std::path::PathBuf {
    use tauri::Manager;
    let dir = app.path().app_config_dir().expect("config dir");
    std::fs::create_dir_all(&dir).ok();
    dir.join("vault.bin")
}

fn main() {
    use tauri::Manager;
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let path = vault_path(app);
            app.manage(commands::AppState {
                vault: std::sync::Mutex::new(vault::Vault::new()),
                path,
            });

            // Background tick: emit "tick" every second so the frontend can
            // pull fresh TOTP codes.
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                let _ = tauri::Emitter::emit(&handle, "tick", ());
            });

            // OS session-lock auto-lock (real WTS listener on Windows).
            session::start_session_lock_listener(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault_exists,
            commands::create_vault,
            commands::unlock,
            commands::lock,
            commands::is_unlocked,
            commands::list_accounts,
            commands::current_codes,
            commands::add_manual,
            commands::add_from_uri,
            commands::decode_qr_file,
            commands::preview_migration,
            commands::import_migration,
            commands::remove_account,
            commands::export_backup,
            commands::import_backup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
