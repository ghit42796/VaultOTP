# Remove xcap + Sans-IO Refactor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the xcap screen-capture dependency/path, and refactor the VaultOTP backend so the entire core is sans-IO-pure with all I/O (fs, rand, uuid, clock) confined to the `commands.rs` edge and `storage.rs` adapter.

**Architecture:** Method A (parameter injection): core modules become pure functions over in-memory data; `commands.rs` is the single composition root that generates salt/nonce (rand), ids (uuid), reads the clock, and performs file I/O via `storage`, then calls the pure core. No new traits.

**Tech Stack:** Rust (Tauri 2), Svelte + TypeScript. Crypto unchanged (Argon2id, AES-256-GCM). Existing crates only.

## Global Constraints

- **NO git operations** (user constraint): the project is intentionally not a git repo. SKIP every "Commit" step; each task ends with a verification step instead. Do not run `git init/add/commit`.
- **Do NOT use `sed`** — use Edit/Write for all file changes; modify shared files (`commands.rs`, `main.rs`, `Cargo.toml`) by Read-then-Edit, preserving unrelated content.
- **Toolchain PATH:** before any cargo command, in bash run `export PATH="$HOME/.cargo/bin:$PATH"` (cargo/rustc 1.96.0 MSVC are installed but not on the default PATH). node/npm are on PATH.
- **Behavior preservation:** identical cryptography (Argon2id + AES-256-GCM, header-as-AAD), vault file MAGIC stays `ATOTP1\0`, opaque `AppError::Crypto` for all crypto failures, master key zeroized on lock, secrets never cross IPC.
- **Sans-IO target:** after this plan, the only files allowed to use `rand::`, `Uuid::new_v4`, `std::fs`/`storage`, or `SystemTime::now` are `commands.rs` (rand/uuid/clock/fs-via-storage), `storage.rs` (fs), `main.rs` (bootstrap `create_dir_all`, tick), and `session.rs` (OS events). All core modules (`model`, `secret`, `totp`, `otpauth`, `migration`, `vault/*`, `backup`, `qr`) must be free of them.
- **Evidence:** every "verify" step must run the stated command and observe the stated output (RED before impl, GREEN after). cargo test currently passes 39/0; npm test 2/2; svelte-check 0 errors.
- TDD throughout; DRY; YAGNI.

## File structure (after this plan)

| File | Responsibility | Purity |
|------|----------------|--------|
| `src-tauri/src/model.rs` | Account/Algorithm data model | pure (no uuid) |
| `src-tauri/src/secret.rs` | Base32 decode | pure (unchanged) |
| `src-tauri/src/totp.rs` | RFC 6238 codes | pure (unchanged) |
| `src-tauri/src/otpauth.rs` | otpauth:// parse | pure (id="") |
| `src-tauri/src/migration.rs` | GA protobuf parse | pure (id="") |
| `src-tauri/src/vault/kdf.rs` | Argon2id KDF | pure (unchanged) |
| `src-tauri/src/vault/crypto.rs` | AES-256-GCM + header | pure (unchanged) |
| `src-tauri/src/vault/mod.rs` | Vault pure state machine | pure (no fs/rand) |
| `src-tauri/src/backup.rs` | Encrypted backup codec | pure (no fs/rand) |
| `src-tauri/src/qr.rs` | QR decode from bytes | pure (no fs/xcap) |
| `src-tauri/src/storage.rs` | Atomic file I/O adapter | edge (unchanged) |
| `src-tauri/src/commands.rs` | Tauri commands / composition root | edge (rand/uuid/clock/fs) |
| `src-tauri/src/session.rs` | OS session-lock listener | edge (unchanged) |
| `src-tauri/src/main.rs` | Bootstrap + tick | edge |

---

### Task A1: Remove xcap and the screen-capture path

**Files:**
- Modify: `src-tauri/src/qr.rs` (delete `capture_region`)
- Modify: `src-tauri/src/commands.rs` (delete `decode_qr_region` + handler registration)
- Modify: `src-tauri/Cargo.toml` (delete `xcap`)
- Modify: `src/lib/ipc.ts` (delete `decodeQrRegion`)
- Modify: `src/components/AddFromQr.svelte` (delete screen button + `fromScreen`)
- Modify: `.github/workflows/build.yml` (delete Xvfb step)
- Modify: `README.md` (drop xcap + screen-scan mention)

**Interfaces:**
- Consumes: nothing new.
- Produces: a build with no xcap subtree; `qr.rs` retains `pub fn decode_image_bytes(&[u8]) -> Result<Vec<String>>` and `pub fn decode_image_file(path: &str) -> Result<Vec<String>>` (the latter is removed in Task B3). `commands` exposes 15 commands (was 16).

- [ ] **Step 1: Delete `capture_region` from `qr.rs`**

In `src-tauri/src/qr.rs`, remove the entire `pub fn capture_region(...) { ... }` function (the only `use xcap::Monitor;` user). Leave `decode_image_bytes` and `decode_image_file` intact.

- [ ] **Step 2: Delete the `decode_qr_region` command**

In `src-tauri/src/commands.rs`, delete the whole `#[tauri::command] pub fn decode_qr_region(...) { ... }` function, and remove the `commands::decode_qr_region,` line from the `tauri::generate_handler![ ... ]` macro in `src-tauri/src/main.rs`.

- [ ] **Step 3: Remove the xcap dependency**

In `src-tauri/Cargo.toml`, delete the line `xcap = "0.0.15"` from `[dependencies]`.

- [ ] **Step 4: Remove the frontend screen-scan UI + IPC**

In `src/lib/ipc.ts`, delete the `export const decodeQrRegion = ...` line. In `src/components/AddFromQr.svelte`, delete the `async function fromScreen() { ... }` and the `<button on:click={fromScreen} ...>Scan screen for QR</button>` element; remove `decodeQrRegion` from the `import` statement (keep `decodeQrFile`, `addFromUri`).

- [ ] **Step 5: Remove CI Xvfb step + README mention**

In `.github/workflows/build.yml`, delete the "Start virtual display (Linux — needed for xcap screen capture …)" step (the `Xvfb` run and any `DISPLAY` env tied only to it). In `README.md`, remove `xcap` from the tech/dependency table and reword any "scan screen" QR description to "scan a QR image file".

- [ ] **Step 6: Verify build is clean and xcap subtree is gone**

Run: `cd src-tauri && export PATH="$HOME/.cargo/bin:$PATH" && cargo build 2>&1 | tail -5`
Expected: compiles successfully.
Run: `grep -E "^name = \"(xcap|dlopen2|dlopen2_derive|dbus|libdbus-sys|xcb)\"" src-tauri/Cargo.lock; echo "exit=$?"`
Expected: no matches (cargo build rewrote `Cargo.lock`). If any remain, ensure no other crate depends on them and re-run build.

- [ ] **Step 7: Verify full suites**

Run: `cd src-tauri && export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | grep "test result"`
Expected: `ok. 39 passed; 0 failed` (no test depended on `capture_region`).
Run: `cd .. && npx svelte-check 2>&1 | tail -2 && npm run build 2>&1 | tail -3`
Expected: svelte-check 0 errors (a11y warnings allowed); vite build succeeds.

- [ ] **Step 8: Done** (no commit — per Global Constraints)

---

### Task B1: Make `Vault` a pure state machine + move persistence to `commands.rs`

**Files:**
- Modify: `src-tauri/src/vault/mod.rs` (new pure API; remove fs/rand; rewrite tests)
- Modify: `src-tauri/src/commands.rs` (add rand helpers; rewrite vault-touching commands)

**Interfaces:**
- Consumes: `vault::kdf::{KdfParams, derive_key}`, `vault::crypto::{VaultHeader, encrypt, decrypt_with_key}`, `model::Account`, `storage`, `error::{AppError, Result}`.
- Produces (on `Vault`):
  - `pub fn new() -> Vault`
  - `pub fn is_unlocked(&self) -> bool`
  - `pub fn create_unlocked(password: &[u8], salt: [u8;16], kdf: KdfParams) -> Result<Vault>`
  - `pub fn unlock_from_bytes(file_bytes: &[u8], password: &[u8]) -> Result<Vault>`
  - `pub fn lock(&mut self)`
  - `pub fn accounts(&self) -> Result<&[Account]>`
  - `pub fn snapshot(&self) -> Result<Vec<Account>>`
  - `pub fn add(&mut self, account: Account) -> Result<()>` (in-memory only)
  - `pub fn remove(&mut self, id: &str) -> Result<()>` (in-memory only)
  - `pub fn serialize(&self, nonce: [u8;12]) -> Result<Vec<u8>>`
  - Produces (in `commands.rs`): `fn random_salt() -> [u8;16]`, `fn random_nonce() -> [u8;12]`.

- [ ] **Step 1: Write the failing pure-vault tests**

Replace the `#[cfg(test)] mod tests { ... }` block in `src-tauri/src/vault/mod.rs` with:
```rust
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
        assert!(v.serialize([0u8; 12]).is_err());
    }
}
```

- [ ] **Step 2: Run tests — verify they fail to compile**

Run: `cd src-tauri && export PATH="$HOME/.cargo/bin:$PATH" && cargo test vault::tests 2>&1 | tail -8`
Expected: FAIL — `create_unlocked`/`unlock_from_bytes`/`serialize` not found (old API still present).

- [ ] **Step 3: Rewrite `vault/mod.rs` to the pure API**

Replace the `impl Vault { ... }` block and remove the old `fn persist`, the `use rand::RngCore;` import, and the `use crate::storage;` import (no longer used here). Keep `pub mod kdf; pub mod crypto;`, the `Unlocked` struct + its `Drop`, and the `parse_header_only` helper. New `impl`:
```rust
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
        let header = parse_header_only(file_bytes)?;
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
```
Ensure the top-of-file `use` lines import `decrypt_with_key` (alongside `encrypt`, `VaultHeader`) and `derive_key`, `KdfParams`. Remove `rand` and `storage` imports.

- [ ] **Step 4: Run tests — verify vault pure tests pass**

Run: `cd src-tauri && export PATH="$HOME/.cargo/bin:$PATH" && cargo test vault:: 2>&1 | grep "test result"`
Expected: PASS (kdf + crypto + the 5 new vault tests). The build will still FAIL overall because `commands.rs` calls the old API — that is fixed in Step 5; run `cargo test vault::` (module-scoped) which compiles the lib; if the lib fails to compile due to commands.rs, proceed to Step 5 then re-run.

- [ ] **Step 5: Rewrite the vault-touching commands in `commands.rs`**

In `src-tauri/src/commands.rs`, add near the top (after imports) the rand helpers, and rewrite the six commands. Add `use rand::RngCore;`, `use crate::vault::Vault;`, `use crate::vault::kdf::KdfParams;`, `use crate::storage;` if not present.
```rust
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
pub fn add_manual(state: tauri::State<AppState>, issuer: String, label: String, secret: String) -> Result<()> {
    crate::secret::decode_secret(&secret)?;
    let acc = Account::new(issuer, label, secret); // id assigned at edge in Task B4
    let mut g = state.vault.lock().unwrap();
    g.add(acc)?;
    let bytes = g.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.path, &bytes)
}

#[tauri::command]
pub fn add_from_uri(state: tauri::State<AppState>, uri: String) -> Result<()> {
    let acc = crate::otpauth::parse_otpauth(&uri)?;
    let mut g = state.vault.lock().unwrap();
    g.add(acc)?;
    let bytes = g.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.path, &bytes)
}

#[tauri::command]
pub fn import_migration(state: tauri::State<AppState>, uri: String, selected_indices: Vec<usize>) -> Result<usize> {
    let accounts = crate::migration::parse_migration(&uri)?;
    let mut g = state.vault.lock().unwrap();
    let mut count = 0;
    for (i, acc) in accounts.into_iter().enumerate() {
        if selected_indices.contains(&i) {
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
```
Keep `lock`, `is_unlocked`, `list_accounts`, `current_codes`, `vault_exists`, `preview_migration`, `decode_qr_file`, `export_backup`, `import_backup` as they are for now (backup/qr refactored in B2/B3; id-at-edge in B4).

- [ ] **Step 6: Verify full build + suite**

Run: `cd src-tauri && export PATH="$HOME/.cargo/bin:$PATH" && cargo build 2>&1 | tail -3 && cargo test 2>&1 | grep "test result"`
Expected: build succeeds; `ok. N passed; 0 failed` (N = 39 minus the 3 old vault tests plus the 5 new = 41; exact count may differ — the key is **0 failed**).

- [ ] **Step 7: Confirm vault is I/O-free**

Run: `grep -nE "std::fs|thread_rng|rand::|storage::|SystemTime" src-tauri/src/vault/mod.rs; echo "exit=$?"`
Expected: no matches (exit=1). Vault is now pure.

- [ ] **Step 8: Done** (no commit)

---

### Task B2: Make `backup.rs` pure + move its I/O to `commands.rs`

**Files:**
- Modify: `src-tauri/src/backup.rs` (pure signatures; rewrite test)
- Modify: `src-tauri/src/commands.rs` (rewrite `export_backup`/`import_backup`)

**Interfaces:**
- Consumes: `vault::crypto::{VaultHeader, encrypt, decrypt}`, `vault::kdf::{KdfParams, derive_key}`, `model::Account`, `error::{AppError, Result}`, and (in commands) `storage`, `random_salt`, `random_nonce`.
- Produces:
  - `pub fn export_encrypted(accounts: &[Account], password: &[u8], salt: [u8;16], nonce: [u8;12]) -> Result<Vec<u8>>`
  - `pub fn import_encrypted(file_bytes: &[u8], password: &[u8]) -> Result<Vec<Account>>`

- [ ] **Step 1: Write the failing pure-backup test**

Replace the `#[cfg(test)] mod tests` block in `src-tauri/src/backup.rs` with:
```rust
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
        assert_eq!(restored[0].id, "id1");
        assert!(matches!(import_encrypted(&bytes, b"wrong"), Err(AppError::Crypto)));
    }
}
```

- [ ] **Step 2: Run test — verify it fails to compile**

Run: `cd src-tauri && export PATH="$HOME/.cargo/bin:$PATH" && cargo test backup::tests 2>&1 | tail -8`
Expected: FAIL — signatures don't match (old `export_encrypted(accounts, password, out_path)` takes a path).

- [ ] **Step 3: Rewrite `backup.rs` to pure signatures**

Replace the two functions (remove `use crate::storage;` and `use rand::RngCore;`; keep crypto/kdf imports):
```rust
use crate::error::{AppError, Result};
use crate::model::Account;
use crate::vault::crypto::{decrypt, encrypt, VaultHeader};
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
    let header = VaultHeader { version: 1, kdf, salt, nonce };
    let plaintext = serde_json::to_vec(accounts).map_err(|_| AppError::Crypto)?;
    encrypt(&key, &header, &plaintext)
}

/// Pure: decrypt a backup blob to accounts. bytes injected.
pub fn import_encrypted(file_bytes: &[u8], password: &[u8]) -> Result<Vec<Account>> {
    let plaintext = decrypt(password, file_bytes)?;
    serde_json::from_slice(&plaintext).map_err(|_| AppError::Crypto)
}
```

- [ ] **Step 4: Run test — verify pass**

Run: `cd src-tauri && export PATH="$HOME/.cargo/bin:$PATH" && cargo test backup::tests 2>&1 | grep "test result"`
Expected: PASS (the lib may not fully build until Step 5 updates commands; if so, do Step 5 then re-run).

- [ ] **Step 5: Rewrite `export_backup`/`import_backup` in `commands.rs`**

```rust
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
    for acc in accounts {
        g.add(acc)?; // backup accounts already carry ids
    }
    let out = g.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.path, &out)?;
    Ok(n)
}
```

- [ ] **Step 6: Verify full build + suite + purity**

Run: `cd src-tauri && export PATH="$HOME/.cargo/bin:$PATH" && cargo build 2>&1 | tail -3 && cargo test 2>&1 | grep "test result"`
Expected: build OK; `0 failed`.
Run: `grep -nE "std::fs|thread_rng|rand::|storage::" src-tauri/src/backup.rs; echo "exit=$?"`
Expected: no matches (exit=1).

- [ ] **Step 7: Done** (no commit)

---

### Task B3: Make `qr.rs` pure + move file read to the edge

**Files:**
- Modify: `src-tauri/src/qr.rs` (remove `decode_image_file`; keep pure `decode_image_bytes`)
- Modify: `src-tauri/src/commands.rs` (rewrite `decode_qr_file` to read bytes then decode)

**Interfaces:**
- Consumes: `qr::decode_image_bytes`, `storage::read_file`.
- Produces: `qr` exposes only `pub fn decode_image_bytes(bytes: &[u8]) -> Result<Vec<String>>`.

- [ ] **Step 1: Remove `decode_image_file` from `qr.rs`**

Delete the `pub fn decode_image_file(path: &str) -> Result<Vec<String>> { ... }` function. The existing tests (`decodes_fixture_qr` using `include_bytes!`, `non_image_bytes_error`) call `decode_image_bytes` only and remain unchanged.

- [ ] **Step 2: Run tests — verify qr tests still pass, but commands fails to compile**

Run: `cd src-tauri && export PATH="$HOME/.cargo/bin:$PATH" && cargo build 2>&1 | tail -6`
Expected: FAIL — `commands.rs` still calls `crate::qr::decode_image_file`.

- [ ] **Step 3: Rewrite `decode_qr_file` in `commands.rs`**

```rust
#[tauri::command]
pub fn decode_qr_file(path: String) -> Result<Vec<String>> {
    let bytes = crate::storage::read_file(std::path::Path::new(&path))?;
    let strings = crate::qr::decode_image_bytes(&bytes)?;
    Ok(strings.into_iter().filter(|s| s.starts_with("otpauth")).collect())
}
```

- [ ] **Step 4: Verify build + suite + purity**

Run: `cd src-tauri && export PATH="$HOME/.cargo/bin:$PATH" && cargo build 2>&1 | tail -3 && cargo test qr:: 2>&1 | grep "test result" && cargo test 2>&1 | grep "test result"`
Expected: build OK; qr tests pass; full suite `0 failed`.
Run: `grep -nE "std::fs|xcap" src-tauri/src/qr.rs; echo "exit=$?"`
Expected: no matches (exit=1).

- [ ] **Step 5: Done** (no commit)

---

### Task B4: Move ID generation to the edge (core produces `id == ""`)

**Files:**
- Modify: `src-tauri/src/model.rs` (Account::new → id="")
- Modify: `src-tauri/src/otpauth.rs` (parse_otpauth → id="")
- Modify: `src-tauri/src/migration.rs` (parse_migration → id="")
- Modify: `src-tauri/src/commands.rs` (add `new_id()`; assign ids in add_manual/add_from_uri/import_migration)

**Interfaces:**
- Consumes: `uuid` (in commands only).
- Produces: `fn new_id() -> String` in `commands.rs`; all core constructors/parsers yield `Account.id == ""`.

- [ ] **Step 1: Update `model.rs` test to expect empty id**

In `src-tauri/src/model.rs`, replace the `new_account_has_uuid_and_defaults` test with:
```rust
    #[test]
    fn new_account_has_empty_id_and_defaults() {
        let a = Account::new("GitHub".into(), "alice".into(), "JBSWY3DPEHPK3PXP".into());
        assert_eq!(a.id, "");
        assert_eq!(a.digits, 6);
        assert_eq!(a.period, 30);
        assert_eq!(a.algorithm, Algorithm::Sha1);
        assert_eq!(a.kind, "totp");
    }
```

- [ ] **Step 2: Run test — verify it fails**

Run: `cd src-tauri && export PATH="$HOME/.cargo/bin:$PATH" && cargo test model:: 2>&1 | tail -8`
Expected: FAIL — `Account::new` still sets a uuid (`a.id == ""` assertion fails).

- [ ] **Step 3: Make core constructors/parsers produce empty ids**

In `src-tauri/src/model.rs`, in `Account::new`, change `id: uuid::Uuid::new_v4().to_string(),` to `id: String::new(),`. Remove the `use uuid...` import if it becomes unused (model.rs no longer needs uuid).
In `src-tauri/src/otpauth.rs`, in the returned `Account { ... }`, change `id: uuid::Uuid::new_v4().to_string(),` to `id: String::new(),`. Remove unused uuid import.
In `src-tauri/src/migration.rs`, in the per-entry `accounts.push(Account { ... })`, change `id: uuid::Uuid::new_v4().to_string(),` to `id: String::new(),`. Remove unused uuid import.

- [ ] **Step 4: Add id-equality assertions to otpauth/migration tests**

In `src-tauri/src/otpauth.rs` `parses_full_uri` test, add `assert_eq!(a.id, "");`.
In `src-tauri/src/migration.rs` `parses_single_totp_entry` test, add `assert_eq!(accounts[0].id, "");`.

- [ ] **Step 5: Assign ids at the edge in `commands.rs`**

Add the helper and update the three add paths:
```rust
fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
```
In `add_manual`, change `let acc = Account::new(issuer, label, secret);` to:
```rust
    let mut acc = Account::new(issuer, label, secret);
    acc.id = new_id();
```
In `add_from_uri`, change `let acc = crate::otpauth::parse_otpauth(&uri)?;` to:
```rust
    let mut acc = crate::otpauth::parse_otpauth(&uri)?;
    acc.id = new_id();
```
In `import_migration`, inside the loop, set the id before adding:
```rust
        if selected_indices.contains(&i) {
            let mut acc = acc;
            acc.id = new_id();
            g.add(acc)?;
            count += 1;
        }
```
(`import_backup` is unchanged — backup accounts keep their existing ids.)

- [ ] **Step 6: Verify build, suite, and core purity of uuid**

Run: `cd src-tauri && export PATH="$HOME/.cargo/bin:$PATH" && cargo build 2>&1 | tail -3 && cargo test 2>&1 | grep "test result"`
Expected: build OK; `0 failed`.
Run: `grep -rnE "Uuid::new_v4|uuid::" src-tauri/src/model.rs src-tauri/src/otpauth.rs src-tauri/src/migration.rs src-tauri/src/vault src-tauri/src/backup.rs src-tauri/src/totp.rs src-tauri/src/secret.rs; echo "exit=$?"`
Expected: no matches (exit=1) — uuid only in `commands.rs`.

- [ ] **Step 7: Done** (no commit)

---

### Task B5: Sans-IO verification gate + final full verification

**Files:** none modified (verification only; fix-ups land in the relevant task above if a check fails).

**Interfaces:** none.

- [ ] **Step 1: Assert core modules are free of all I/O primitives**

Run:
```bash
grep -rnE "std::fs|thread_rng|rand::|SystemTime|Uuid::new_v4|storage::|xcap" \
  src-tauri/src/model.rs src-tauri/src/secret.rs src-tauri/src/totp.rs \
  src-tauri/src/otpauth.rs src-tauri/src/migration.rs \
  src-tauri/src/vault/mod.rs src-tauri/src/vault/kdf.rs src-tauri/src/vault/crypto.rs \
  src-tauri/src/backup.rs src-tauri/src/qr.rs
echo "exit=$?"
```
Expected: **no matches (exit=1)** — every core module is pure.

- [ ] **Step 2: Confirm I/O lives only at the edges**

Run: `grep -rlnE "thread_rng|Uuid::new_v4|SystemTime::now" src-tauri/src | sort`
Expected: only `src-tauri/src/commands.rs`.
Run: `grep -rlnE "std::fs::" src-tauri/src | sort`
Expected: only `src-tauri/src/storage.rs` and `src-tauri/src/main.rs` (bootstrap `create_dir_all`). (`commands.rs` does fs via `storage::`, not `std::fs::` directly.)

- [ ] **Step 3: Confirm xcap subtree is gone**

Run: `grep -cE "^name = \"(xcap|dlopen2|dbus|xcb)\"" src-tauri/Cargo.lock`
Expected: `0`.

- [ ] **Step 4: Full test + build pass (Rust + frontend)**

Run: `cd src-tauri && export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | grep "test result" && cargo build --release 2>&1 | tail -2`
Expected: `0 failed`; release build succeeds.
Run: `cd .. && npm test 2>&1 | grep -E "Tests|passed" && npx svelte-check 2>&1 | tail -1 && npm run build 2>&1 | tail -2`
Expected: vitest 2/2; svelte-check 0 errors; vite build OK.

- [ ] **Step 5: Done** (no commit)

---

## Notes for the implementer
- **Why this order:** A1 is independent. B1 changes the most-depended-on module (`Vault`) together with its only caller (`commands.rs`) so the build never stays broken across a task boundary. B2/B3 mirror that pairing for backup/qr. B4 flips id-generation to the edge last, in one atomic move (core → "", edge → assigns), so no intermediate state ships empty ids. B5 is the sans-IO proof.
- **`KdfParams::default()` in tests:** `backup` tests run a real 64 MiB Argon2 (~0.1–0.3 s) because backup always uses default params; vault tests inject `fast_kdf()` and are instant. Both acceptable.
- **Persistence-failure semantics** are unchanged from the original (mutate-in-memory then write; a write error returns `Err`). `create_vault`/`unlock` are slightly stronger here — they only commit the new `Vault` into `AppState` after a successful write/read.
- **Don't** change `vault/crypto.rs`, `vault/kdf.rs`, `storage.rs`, `secret.rs`, `totp.rs` logic — they are already pure; touching them is out of scope.
