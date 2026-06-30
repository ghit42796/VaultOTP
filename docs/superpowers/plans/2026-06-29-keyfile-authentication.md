# Key-File Authentication & Vault Format `mode` — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a vault be protected by one of three credential modes — password only, key file only, or password **and** key file (both required) — while keeping existing password-only vaults openable.

**Architecture:** The content key is still derived directly (no key-slot envelope) because each vault has exactly one unlock path. A `mode` field is added to the encrypted header (authenticated as AAD); the KDF input varies by mode. The pure `vault`/`crypto`/`kdf` core takes a `Credential` plus injected randomness; the Tauri commands edge reads key-file bytes and supplies salts/nonces. Existing v1 files (no `mode` field) deserialize as `password` and are rewritten with the field on the next save ("read old, write new").

**Tech Stack:** Rust (Tauri 2 backend), `argon2`, `aes-gcm`, `sha2`, `zeroize`, `serde`/`serde_json`; Svelte 4 + TypeScript frontend; `@tauri-apps/plugin-dialog`; tests via `cargo test` and `vitest`.

This is Plan 1 of 3 (see `docs/superpowers/specs/2026-06-29-custom-path-keyfile-export-design.md`). Plan 2 = multi-vault custom paths; Plan 3 = account secret export.

## Global Constraints

- **Sans-IO core:** `src-tauri/src/vault/**`, `kdf`, `crypto` stay pure — no `rand`, no clock, no filesystem. All randomness (salt, nonce, generated key files) and all file I/O live in `src-tauri/src/commands.rs` / `main.rs`. (Audited property: `docs/superpowers/audits/2026-06-28-dependency-and-sansio-audit.md`.)
- **No new dependencies** in this plan. Only crates already in `src-tauri/Cargo.toml` (`argon2`, `aes-gcm`, `sha2`, `zeroize`, `serde`, `serde_json`, `rand`).
- **No database / no SQL** is produced anywhere — the app has no DB.
- **Opaque failures:** every unlock/verify failure returns `AppError::Crypto`; never reveal which check failed.
- **Zeroize key material:** content keys and derived material are `Zeroizing`/zeroized on drop, as today (`src-tauri/src/vault/mod.rs:17-21`).
- **KDF defaults unchanged:** `KdfParams::default()` = 64 MiB / 3 iters / 4 lanes (`src-tauri/src/vault/kdf.rs:16-22`).
- **Rust test command:** `cargo test --manifest-path src-tauri/Cargo.toml <name>`.
- **Frontend test command:** `npm test` (vitest run); type-check: `npm run check` is absent — use `npx svelte-check --tsconfig ./tsconfig.json`.
- **Commit style:** Conventional Commits; end the message body with `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

---

## File Structure

- `src-tauri/src/vault/crypto.rs` — add `Mode` enum; add `mode` field to `VaultHeader`; add `decrypt_with_derived_key`. (Header & AAD owner.)
- `src-tauri/src/vault/kdf.rs` — rename `derive_key` param to `material`; add `composite_material`. (KDF owner.)
- `src-tauri/src/vault/mod.rs` — add `Credential` enum; make `create_unlocked`/`unlock_from_bytes` credential-aware; add `verify_current`, `rekey`, `mode`, and free fn `peek_mode`; store `mode` in `Unlocked`. (Vault state machine.)
- `src-tauri/src/commands.rs` — `create_vault`/`unlock` become mode-aware; add `vault_mode`, `add_keyfile`, `remove_keyfile`, `change_password`, `generate_keyfile`. (IPC edge: reads key-file bytes, injects randomness.)
- `src-tauri/src/main.rs` — register the new commands.
- `src/lib/ipc.ts` + `src/lib/ipc.test.ts` — typed bindings for the new/changed commands.
- `src/routes/Unlock.svelte` — mode-aware create/unlock UI (password / key-file picker / both).
- `src/components/Settings.svelte` — "Key file & password" management section.

---

### Task 1: `Mode` enum + header field (read-compat for v1)

**Files:**
- Modify: `src-tauri/src/vault/crypto.rs:13-21` (the `VaultHeader` struct) and add `Mode` near the top.
- Test: `src-tauri/src/vault/crypto.rs` (inline `#[cfg(test)]`).

**Interfaces:**
- Produces: `pub enum Mode { Password, Keyfile, Composite }` (serde `lowercase`, `Default = Password`); `VaultHeader.mode: Mode` with `#[serde(default)]`.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src-tauri/src/vault/crypto.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml header_without_mode_field_defaults_to_password`
Expected: FAIL — `cannot find type Mode` / `no field mode`.

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/vault/crypto.rs`, add after the imports (near line 6):

```rust
/// Which credential(s) unlock a vault. Stored in the header and authenticated as AAD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Password,
    Keyfile,
    Composite,
}
```

Then add the field to `VaultHeader` (between `version` and `kdf`):

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib vault::crypto`
Expected: PASS (new tests plus the existing `round_trip`, `wrong_password_fails`, `tampered_*`).
Note: existing tests construct `VaultHeader { version, kdf, salt, nonce }` (e.g. `fast_header`, line 128) — update each literal to include `mode: Mode::Password,`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/vault/crypto.rs
git commit -m "feat(vault): add Mode enum and serde-default header.mode field

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: KDF — credential material helper

**Files:**
- Modify: `src-tauri/src/vault/kdf.rs:32-55` (`derive_key`).
- Test: `src-tauri/src/vault/kdf.rs` (inline `#[cfg(test)]`).

**Interfaces:**
- Consumes: `KdfParams` (existing).
- Produces: `derive_key(material: &[u8], salt: &[u8;16], params: &KdfParams) -> Result<Zeroizing<[u8;32]>>` (param renamed `password`→`material`; behavior identical). `composite_material(password: &[u8], keyfile: &[u8]) -> Vec<u8>` = `SHA-256(password) ‖ SHA-256(keyfile)` (64 bytes).

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src-tauri/src/vault/kdf.rs`:

```rust
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
        assert_ne!(base, composite_material(b"pw2", b"kf"));
        assert_ne!(base, composite_material(b"pw", b"kf2"));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml composite_material_is_two_sha256_digests`
Expected: FAIL — `cannot find function composite_material`.

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/vault/kdf.rs`, rename the `derive_key` first parameter and add the helper. Change the signature line `pub fn derive_key(password: &[u8],` to `pub fn derive_key(material: &[u8],`, and inside the body change `Sha256::digest(password)` to `Sha256::digest(material)`. Then add below `derive_key`:

```rust
/// Build the KDF input for a composite (password + key file) credential:
/// `SHA-256(password) ‖ SHA-256(keyfile)` (64 bytes). Feeding this into
/// `derive_key` applies the existing SHA-256 + Argon2id stretch on top.
pub fn composite_material(password: &[u8], keyfile: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut v = Vec::with_capacity(64);
    v.extend_from_slice(&Sha256::digest(password));
    v.extend_from_slice(&Sha256::digest(keyfile));
    v
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib vault::kdf`
Expected: PASS (new tests plus existing `same_input_same_key`, `different_*`). The existing tests call `derive_key(b"hunter2", ...)` — positional, so the rename needs no test edits.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/vault/kdf.rs
git commit -m "feat(vault): add composite_material; rename derive_key param to material

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: `Credential` + mode-aware create/unlock/serialize

**Files:**
- Modify: `src-tauri/src/vault/mod.rs` (`Unlocked`, `create_unlocked`, `unlock_from_bytes`, `serialize`).
- Modify: `src-tauri/src/vault/crypto.rs` (add `decrypt_with_derived_key`).
- Test: `src-tauri/src/vault/mod.rs` (inline `#[cfg(test)]`).

**Interfaces:**
- Consumes: `derive_key`, `composite_material` (Task 2); `Mode` (Task 1).
- Produces:
  - `pub enum Credential<'a> { Password(&'a [u8]), KeyFile(&'a [u8]), Both { password: &'a [u8], keyfile: &'a [u8] } }` with `pub fn mode(&self) -> Mode`.
  - `create_unlocked(cred: Credential, salt: [u8;16], kdf: KdfParams) -> Result<Vault>`
  - `unlock_from_bytes(file_bytes: &[u8], cred: Credential) -> Result<Vault>`
  - `serialize(&self, nonce: [u8;12]) -> Result<Vec<u8>>` (now writes `mode`)
  - `crypto::decrypt_with_derived_key(key: &[u8;32], file_bytes: &[u8]) -> Result<Vec<u8>>`

- [ ] **Step 1: Write the failing test**

Replace the `create_serialize_unlock_cycle` test in `src-tauri/src/vault/mod.rs` and add mode tests. Add to the `tests` module:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib vault::tests::password_mode_round_trips`
Expected: FAIL — `cannot find Credential` / arity mismatch on `create_unlocked`.

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/vault/crypto.rs`, add (after `decrypt_with_key`):

```rust
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
```

In `src-tauri/src/vault/mod.rs`, update imports and replace the `Unlocked`/`Vault` impl pieces. Set the imports near the top to:

```rust
use crate::error::{AppError, Result};
use crate::model::Account;
use crypto::{decrypt_with_derived_key, encrypt, Mode, VaultHeader};
use kdf::{composite_material, derive_key, KdfParams};
use zeroize::{Zeroize, Zeroizing};
```

Add the `Credential` type (after the `use` lines):

```rust
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
            Credential::Both { password, keyfile } => Zeroizing::new(composite_material(password, keyfile)),
        }
    }
}

/// Read a vault file's `mode` without unlocking it (header is plaintext).
pub fn peek_mode(file_bytes: &[u8]) -> Result<Mode> {
    let (header, _) = crypto::parse_header(file_bytes)?;
    Ok(header.mode)
}
```

Add `mode: Mode` to `Unlocked`:

```rust
struct Unlocked {
    key: [u8; 32],
    accounts: Vec<Account>,
    kdf: KdfParams,
    salt: [u8; 16],
    mode: Mode,
}
```

Replace `create_unlocked`, `unlock_from_bytes`, and `serialize`:

```rust
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
```

```rust
    /// Pure: encrypt current accounts with the stored key + injected nonce. No I/O.
    pub fn serialize(&self, nonce: [u8; 12]) -> Result<Vec<u8>> {
        let u = self.state.as_ref().ok_or(AppError::Crypto)?;
        let header = VaultHeader { version: 1, mode: u.mode, kdf: u.kdf, salt: u.salt, nonce };
        let plaintext = serde_json::to_vec(&u.accounts).map_err(|_| AppError::Crypto)?;
        encrypt(&u.key, &header, &plaintext)
    }
```

In the existing `tests` module, the `acc`/`fast_kdf` helpers stay. Update the old `wrong_password_fails`, `tamper_fails`, `remove_in_memory`, `locked_ops_error`, `create_serialize_unlock_cycle` tests to call `Vault::create_unlocked(Credential::Password(b"pw"), ...)` and `Vault::unlock_from_bytes(&bytes, Credential::Password(b"pw"))`. (Delete the now-replaced `create_serialize_unlock_cycle` body or convert it to `password_mode_round_trips`.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib vault`
Expected: PASS (all mode round-trips + updated existing tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/vault/mod.rs src-tauri/src/vault/crypto.rs
git commit -m "feat(vault): credential-aware create/unlock with mode + peek_mode

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: `verify_current` + `rekey` (change credentials in place)

**Files:**
- Modify: `src-tauri/src/vault/mod.rs` (add two methods + a `mode` getter).
- Test: `src-tauri/src/vault/mod.rs` (inline `#[cfg(test)]`).

**Interfaces:**
- Produces:
  - `verify_current(&self, cred: Credential) -> Result<()>` — Ok iff `cred` re-derives the current content key.
  - `rekey(&mut self, new: Credential, salt: [u8;16], kdf: KdfParams) -> Result<()>` — switch key+mode in memory; next `serialize` re-encrypts content under the new key.
  - `mode(&self) -> Result<Mode>`.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src-tauri/src/vault/mod.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib vault::tests::rekey_password_to_composite_then_unlock_with_both`
Expected: FAIL — `no method named rekey` / `verify_current` / `mode`.

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/vault/mod.rs`, add inside `impl Vault` (after `serialize`):

```rust
    pub fn mode(&self) -> Result<Mode> {
        self.state.as_ref().map(|u| u.mode).ok_or(AppError::Crypto)
    }

    /// Constant-purpose check: returns Ok iff `cred` re-derives the current content key.
    /// Use before a destructive `rekey` so a mistyped current credential cannot lock the user out.
    pub fn verify_current(&self, cred: Credential) -> Result<()> {
        let u = self.state.as_ref().ok_or(AppError::Crypto)?;
        let key: [u8; 32] = *derive_key(&cred.material(), &u.salt, &u.kdf)?;
        let ok = key == u.key;
        let mut key = key;
        key.zeroize();
        if ok { Ok(()) } else { Err(AppError::Crypto) }
    }

    /// Switch the content key + mode to a new credential (re-derived from injected salt/kdf).
    /// The next `serialize` re-encrypts the accounts under the new key.
    pub fn rekey(&mut self, new: Credential, salt: [u8; 16], kdf: KdfParams) -> Result<()> {
        let new_key: [u8; 32] = *derive_key(&new.material(), &salt, &kdf)?;
        let u = self.state.as_mut().ok_or(AppError::Crypto)?;
        u.key.zeroize();
        u.key = new_key;
        u.salt = salt;
        u.kdf = kdf;
        u.mode = new.mode();
        Ok(())
    }
```

Ensure `Mode` is imported in scope (already imported in Task 3 via `use crypto::{..., Mode, ...}`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib vault`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/vault/mod.rs
git commit -m "feat(vault): verify_current + rekey for in-place credential changes

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Commands — mode-aware create/unlock + `vault_mode`

**Files:**
- Modify: `src-tauri/src/commands.rs` (`create_vault`, `unlock`; add `vault_mode` + a `mode_str`/`read_keyfile` helper).
- Modify: `src-tauri/src/main.rs:66-82` (register `vault_mode`).
- Test: `src-tauri/src/commands.rs` (inline `#[cfg(test)]`).

**Interfaces:**
- Consumes: `Vault::{create_unlocked, unlock_from_bytes, peek_mode}`, `Credential`, `Mode`.
- Produces (Tauri commands):
  - `create_vault(state, mode: String, password: Option<String>, keyfile_path: Option<String>) -> Result<()>`
  - `unlock(state, password: Option<String>, keyfile_path: Option<String>) -> Result<()>`
  - `vault_mode(state) -> Result<String>` (`"password"|"keyfile"|"composite"`)

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src-tauri/src/commands.rs`:

```rust
    #[test]
    fn mode_str_round_trips_with_parse() {
        assert_eq!(mode_str(crate::vault::crypto::Mode::Password), "password");
        assert_eq!(mode_str(crate::vault::crypto::Mode::Keyfile), "keyfile");
        assert_eq!(mode_str(crate::vault::crypto::Mode::Composite), "composite");
        assert!(matches!(parse_mode("password").unwrap(), crate::vault::crypto::Mode::Password));
        assert!(parse_mode("bogus").is_err());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml mode_str_round_trips_with_parse`
Expected: FAIL — `cannot find function mode_str` / `parse_mode`.

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/commands.rs`, update the top `use` block to include the vault types:

```rust
use crate::vault::{Credential, Vault};
use crate::vault::crypto::Mode;
use crate::vault::kdf::KdfParams;
```

Add helpers (near `now_unix`, after line 67):

```rust
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
```

Replace `create_vault` (lines 74-81) and `unlock` (lines 83-89), and add `vault_mode`:

```rust
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
    crate::storage::write_atomic(&state.path, &bytes)?;
    *state.vault.lock().unwrap() = v;
    Ok(())
}

#[tauri::command]
pub fn unlock(
    state: tauri::State<AppState>,
    password: Option<String>,
    keyfile_path: Option<String>,
) -> Result<()> {
    let bytes = crate::storage::read_file(&state.path)?;
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
    let bytes = crate::storage::read_file(&state.path)?;
    Ok(mode_str(crate::vault::peek_mode(&bytes)?).to_string())
}
```

In `src-tauri/src/main.rs`, add `commands::vault_mode,` to the `generate_handler!` list (after `commands::vault_exists,`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml` then `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: PASS and a clean build. (The `code_view_has_no_secret_field…` and qr tests are unaffected.)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(commands): mode-aware create_vault/unlock + vault_mode

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: Commands — manage key file & password

**Files:**
- Modify: `src-tauri/src/commands.rs` (add `add_keyfile`, `remove_keyfile`, `change_password`, `generate_keyfile`).
- Modify: `src-tauri/src/main.rs` (register the four commands).
- Test: `src-tauri/src/commands.rs` (inline `#[cfg(test)]`).

**Interfaces:**
- Consumes: `Vault::{verify_current, rekey}`, `Credential`.
- Produces (Tauri commands): `add_keyfile(state, password, keyfile_path) -> Result<()>`, `remove_keyfile(state, password) -> Result<()>`, `change_password(state, current_password, current_keyfile_path: Option<String>, new_password, new_keyfile_path: Option<String>) -> Result<()>`, `generate_keyfile(out_path) -> Result<()>`.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src-tauri/src/commands.rs` (a pure check that generate writes 32 random bytes; full command flow is covered by the vault-layer tests in Task 4):

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml generate_keyfile_writes_32_random_bytes`
Expected: FAIL — `cannot find function generate_keyfile`.

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/commands.rs`, add these commands (after `unlock`/`vault_mode`):

```rust
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
    // The vault is currently password-only; confirm the typed password before re-keying.
    g.verify_current(Credential::Password(password.as_bytes()))?;
    g.rekey(Credential::Both { password: password.as_bytes(), keyfile: &kf }, random_salt(), KdfParams::default())?;
    let bytes = g.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.path, &bytes)
}

#[tauri::command]
pub fn remove_keyfile(state: tauri::State<AppState>, password: String, keyfile_path: String) -> Result<()> {
    let kf = read_keyfile(&keyfile_path)?;
    let mut g = state.vault.lock().unwrap();
    // Confirm the current composite credential, then drop to password-only.
    g.verify_current(Credential::Both { password: password.as_bytes(), keyfile: &kf })?;
    g.rekey(Credential::Password(password.as_bytes()), random_salt(), KdfParams::default())?;
    let bytes = g.serialize(random_nonce())?;
    crate::storage::write_atomic(&state.path, &bytes)
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
    crate::storage::write_atomic(&state.path, &bytes)
}
```

In `src-tauri/src/main.rs`, add to `generate_handler!`: `commands::add_keyfile,`, `commands::remove_keyfile,`, `commands::change_password,`, `commands::generate_keyfile,`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml` then `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: PASS + clean build.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(commands): add/remove key file, change password, generate key file

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 7: IPC bindings + Unlock UI (mode-aware)

**Files:**
- Modify: `src/lib/ipc.ts` (change `createVault`/`unlock` signatures; add `vaultMode`, key-file commands).
- Modify: `src/lib/ipc.test.ts` (cover the new argument shapes).
- Modify: `src/routes/Unlock.svelte` (mode selector on create; key-file picker on unlock).

**Interfaces:**
- Consumes (backend): commands from Tasks 5-6.
- Produces (TS): `createVault(mode, password?, keyfilePath?)`, `unlock(password?, keyfilePath?)`, `vaultMode()`, `generateKeyfile(outPath)`, `addKeyfile(password, keyfilePath)`, `removeKeyfile(password, keyfilePath)`, `changePassword(currentPassword, currentKeyfilePath, newPassword, newKeyfilePath)`.

- [ ] **Step 1: Write the failing test**

Replace the `unlock forwards password` test in `src/lib/ipc.test.ts` and add a create test:

```ts
  it("unlock forwards optional password + keyfile path", async () => {
    invokeMock.mockResolvedValue(undefined);
    await unlock("pw", "/k.vaultkey");
    expect(invokeMock).toHaveBeenCalledWith("unlock", { password: "pw", keyfilePath: "/k.vaultkey" });
  });

  it("createVault forwards mode and credentials", async () => {
    invokeMock.mockResolvedValue(undefined);
    await createVault("composite", "pw", "/k.vaultkey");
    expect(invokeMock).toHaveBeenCalledWith("create_vault", {
      mode: "composite",
      password: "pw",
      keyfilePath: "/k.vaultkey",
    });
  });
```

Update the import line in the test to: `import { unlock, currentCodes, createVault } from "./ipc";`

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/lib/ipc.test.ts`
Expected: FAIL — `unlock` arity / `createVault` not exported with that signature.

- [ ] **Step 3: Write minimal implementation**

In `src/lib/ipc.ts`, replace the `createVault` and `unlock` lines (5-7) and add the new bindings:

```ts
export const vaultExists = () => invoke<boolean>("vault_exists");
export const vaultMode = () => invoke<"password" | "keyfile" | "composite">("vault_mode");
export const createVault = (mode: string, password?: string, keyfilePath?: string) =>
  invoke<void>("create_vault", { mode, password, keyfilePath });
export const unlock = (password?: string, keyfilePath?: string) =>
  invoke<void>("unlock", { password, keyfilePath });
export const generateKeyfile = (outPath: string) => invoke<void>("generate_keyfile", { outPath });
export const addKeyfile = (password: string, keyfilePath: string) =>
  invoke<void>("add_keyfile", { password, keyfilePath });
export const removeKeyfile = (password: string, keyfilePath: string) =>
  invoke<void>("remove_keyfile", { password, keyfilePath });
export const changePassword = (
  currentPassword: string,
  currentKeyfilePath: string | undefined,
  newPassword: string,
  newKeyfilePath: string | undefined,
) =>
  invoke<void>("change_password", { currentPassword, currentKeyfilePath, newPassword, newKeyfilePath });
```

In `src/routes/Unlock.svelte`, make create/unlock mode-aware. Add to the `<script>` (imports + state):

```ts
  import { vaultExists, vaultMode, createVault, unlock } from "../lib/ipc";
  import { open } from "@tauri-apps/plugin-dialog";
  // ...existing state...
  let mode: "password" | "keyfile" | "composite" = "password"; // create: chosen; unlock: read from vault
  let keyfilePath = "";

  async function pickKeyfile() {
    const p = await open({ multiple: false });
    if (typeof p === "string") keyfilePath = p;
  }
```

In `onMount`, after `exists = await vaultExists();`, when it exists read the mode:

```ts
    if (exists) {
      try { mode = await vaultMode(); } catch { /* leave default */ }
    }
```

Replace the `submit()` create/unlock calls so they pass the right credentials by mode:

```ts
      if (exists) {
        const pw = mode === "keyfile" ? undefined : password;
        const kf = mode === "password" ? undefined : (keyfilePath || undefined);
        if (mode !== "password" && !kf) { error = "Select a key file"; return; }
        await unlock(pw, kf);
      } else {
        if (mode !== "keyfile") {
          if (password.length < 8) { error = "Password must be at least 8 characters"; return; }
          if (password !== confirmPassword) { error = "Passwords do not match"; return; }
        }
        if (mode !== "password" && !keyfilePath) { error = "Select a key file"; return; }
        await createVault(
          mode,
          mode === "keyfile" ? undefined : password,
          mode === "password" ? undefined : keyfilePath,
        );
      }
```

In the markup, add (create only) a 3-way mode selector and (when `mode !== "password"`) a key-file picker row. Place inside the create branch and before the buttons:

```svelte
  {#if !exists}
    <div class="modes">
      <button class:active={mode === "password"} on:click={() => (mode = "password")}>Password</button>
      <button class:active={mode === "keyfile"} on:click={() => (mode = "keyfile")}>Key file</button>
      <button class:active={mode === "composite"} on:click={() => (mode = "composite")}>Both</button>
    </div>
  {/if}

  {#if mode !== "keyfile"}
    <input class="field" type="password" bind:value={password} placeholder="Master password" />
  {/if}
  {#if !exists && mode !== "keyfile"}
    <input class="field" type="password" bind:value={confirmPassword} placeholder="Confirm password" />
  {/if}
  {#if mode !== "password"}
    <button class="vo-ghost" on:click={pickKeyfile}>{keyfilePath ? "Key file ✓" : "Select key file…"}</button>
  {/if}
```

(Remove the old unconditional password inputs that this replaces; keep the strength meter under the password input when `!exists && mode !== "keyfile"`.)

- [ ] **Step 4: Verify**

Run: `npm test -- src/lib/ipc.test.ts` → PASS.
Run: `npx svelte-check --tsconfig ./tsconfig.json` → no new errors in `Unlock.svelte`.
Manual: `npm run tauri dev`, create a "Both" vault (password + generated key file), lock, unlock with both; confirm a password-only vault still unlocks.

- [ ] **Step 5: Commit**

```bash
git add src/lib/ipc.ts src/lib/ipc.test.ts src/routes/Unlock.svelte
git commit -m "feat(ui): mode-aware create/unlock with key-file picker

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 8: Settings — key file & password management

**Files:**
- Modify: `src/components/Settings.svelte` (add a "Security" section).

**Interfaces:**
- Consumes (TS): `addKeyfile`, `removeKeyfile`, `generateKeyfile`, `changePassword`, `vaultMode` from `src/lib/ipc.ts`; `open`/`save` from `@tauri-apps/plugin-dialog`.

- [ ] **Step 1: Write the section (no automated test — component layer)**

In `src/components/Settings.svelte` `<script>`, extend the imports and add handlers:

```ts
  import { exportBackup, importBackup, vaultMode, addKeyfile, removeKeyfile, generateKeyfile, changePassword } from "../lib/ipc";
  import { open, save } from "@tauri-apps/plugin-dialog";
  let secMode: "password" | "keyfile" | "composite" = "password";
  let secPw = "", secStatus = "", secError = "";

  async function refreshMode() { try { secMode = await vaultMode(); } catch {} }
  refreshMode();

  async function doGenerateKeyfile(): Promise<string | null> {
    const path = await save({ defaultPath: "vaultotp.vaultkey" });
    if (typeof path !== "string") return null;
    await generateKeyfile(path);
    return path;
  }
```

The add-key-file UX offers two buttons — "Generate key file…" and "Use existing file…" — as separate handlers to keep the path source explicit:

```ts
  async function addGenerated() {
    secError = ""; secStatus = "";
    if (!secPw) { secError = "Enter your current password"; return; }
    const path = await doGenerateKeyfile();
    if (!path) return;
    try { await addKeyfile(secPw, path); secStatus = "Key file added."; secPw = ""; await refreshMode(); }
    catch (e) { secError = String(e); }
  }

  async function addExisting() {
    secError = ""; secStatus = "";
    if (!secPw) { secError = "Enter your current password"; return; }
    const path = await open({ multiple: false });
    if (typeof path !== "string") return;
    try { await addKeyfile(secPw, path); secStatus = "Key file added."; secPw = ""; await refreshMode(); }
    catch (e) { secError = String(e); }
  }

  async function dropKeyfile() {
    secError = ""; secStatus = "";
    if (!secPw) { secError = "Enter your current password"; return; }
    const path = await open({ multiple: false });
    if (typeof path !== "string") { secError = "Select your current key file to confirm"; return; }
    try { await removeKeyfile(secPw, path); secStatus = "Key file removed."; secPw = ""; await refreshMode(); }
    catch (e) { secError = String(e); }
  }
```

In the markup, add a `<section class="setting">` after the appearance section:

```svelte
    <section class="setting">
      <h3>Security</h3>
      <p class="hint">Mode: {secMode === "composite" ? "Password + key file" : secMode === "keyfile" ? "Key file only" : "Password only"}</p>
      <input class="field" type="password" bind:value={secPw} placeholder="Current password" />
      {#if secMode === "password"}
        <div class="row">
          <button class="vo-ghost" on:click={addGenerated}>Generate key file…</button>
          <button class="vo-ghost" on:click={addExisting}>Use existing file…</button>
        </div>
      {/if}
      {#if secMode === "composite"}
        <div class="row"><button class="vo-ghost" on:click={dropKeyfile}>Remove key file…</button></div>
      {/if}
      {#if secError}<p class="err">{secError}</p>{/if}
      {#if secStatus}<p class="hint">{secStatus}</p>{/if}
      <p class="hint">If you lose all required credentials, the vault cannot be recovered. A generated key file has stronger entropy than an existing file.</p>
    </section>
```

(Reuse existing `.field`, `.row`, `.err`, `.hint` styles if present; otherwise add minimal styles mirroring the appearance section.)

- [ ] **Step 2: Verify**

Run: `npx svelte-check --tsconfig ./tsconfig.json` → no new errors.
Manual: `npm run tauri dev`. From a password-only vault: enter password → "Generate key file…" → save → mode shows "Password + key file"; lock; unlock requires both. Then "Remove key file…" with the saved key file → back to password-only.

- [ ] **Step 3: Commit**

```bash
git add src/components/Settings.svelte
git commit -m "feat(ui): Settings security section to manage key file

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Self-Review

**Spec coverage (key-file portions of the spec):**
- Three modes password/keyfile/composite — Tasks 1-3, 5, 7. ✓
- Direct derivation, no VMK envelope — Task 3 (content key from credential material). ✓
- Key-file source = existing file **and** app-generated — Task 6 `generate_keyfile` + Task 8 both buttons. ✓
- `mode` authenticated as AAD — Task 1 field sits inside header JSON, which `encrypt`/`decrypt` already treat as AAD (`crypto.rs:23-31`). ✓
- v1 read-compat + write-new — Task 1 `#[serde(default)]`; Task 3 `serialize` always writes `mode`. ✓
- Opaque `AppError::Crypto` on failure — Tasks 3-6. ✓
- Change credentials (add/remove key file, change password) — Tasks 4, 6, 8. ✓
- Zeroize material — Task 3 `Credential::material` returns `Zeroizing`; Task 4 zeroizes old/compare keys. ✓
- Backup stays password-only — untouched (`backup.rs` still calls `derive_key(password, …)`, which works as `material`). ✓
- No new deps / no SQL — Global Constraints. ✓

**Out of scope here (Plans 2 & 3):** multi-vault paths/`open_vault`/`save_vault_as`/`VaultPicker`; secret export + QR encoder dep. The `state.path` single fixed vault remains as-is in this plan.

**Placeholder scan:** no TBD/TODO; every code step shows full code. (The `doAddKeyfile` half-sketch in Task 8 Step 1 is explicitly superseded by `addGenerated`/`addExisting` in the same step — remove it when implementing.)

**Type consistency:** `Mode`/`Credential` names, `mode_str`/`parse_mode`, `peek_mode`, `rekey`/`verify_current`, and the IPC arg keys (`keyfilePath`, `currentKeyfilePath`, `newKeyfilePath`) are used identically across tasks and match the Tauri camelCase↔snake_case convention (`keyfile_path` ↔ `keyfilePath`).
