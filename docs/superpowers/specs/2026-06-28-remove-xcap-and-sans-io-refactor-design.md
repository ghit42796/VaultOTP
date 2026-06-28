# VaultOTP — Remove xcap/Screen-Capture + Sans-IO Refactor — Design

- **Date:** 2026-06-28
- **Status:** Approved design, pending implementation plan
- **Scope:** Two coordinated changes to the existing VaultOTP codebase — (1) remove the xcap dependency and the screen-capture QR path, (2) refactor the backend to fully conform to the sans-IO principle (method A: parameter injection).
- **Driver:** Findings of `docs/superpowers/audits/2026-06-28-dependency-and-sansio-audit.md` (§3 xcap = highest-privilege non-official surface; §5 sans-IO deviations).

## 1. Goals & non-goals

### Goals
- **Remove xcap entirely**, including the whole `dlopen2` / `dbus` / `xcb` transitive FFI subtree, by deleting the screen-capture QR-add path.
- **Make the backend core sans-IO-pure**: no module other than the `commands.rs` edge (and the `storage.rs` adapter) performs I/O — no filesystem, no `rand`, no `uuid`, no system clock inside core logic.
- **Preserve all external behavior**: identical cryptography, identical opaque errors, identical user-facing features (minus screen capture), identical security properties.

### Non-goals (YAGNI)
- No trait-based dependency injection (`Rng`/`Storage` traits) — that is hexagonal/DI, less faithful to sans-IO than passing concrete values; rejected.
- No change to the clock handling — it is **already** sans-IO compliant (`totp::generate(unix_time)` injects time; the sole `SystemTime::now()` is `commands.rs::now_unix()` at the edge).
- No new features; no replacement for screen capture (file-based QR import remains).
- No change to the vault file format, MAGIC (`ATOTP1\0`), or crypto algorithms.

## 2. Part A — Remove xcap / screen capture

The only code use of xcap is `qr.rs::capture_region`, reached via the "Scan screen for QR" button. Removing that path removes the entire xcap subtree.

### Files changed
| File | Change |
|------|--------|
| `src-tauri/src/qr.rs` | Delete `capture_region()` (the only `use xcap`). Keep pure `decode_image_bytes` (see Part B for the `decode_image_file` move). |
| `src-tauri/src/commands.rs` | Delete the `decode_qr_region` command; remove it from `generate_handler!` (16 → 15 commands). |
| `src-tauri/Cargo.toml` | Delete `xcap = "0.0.15"`. |
| `src/lib/ipc.ts` | Delete `decodeQrRegion`. |
| `src/components/AddFromQr.svelte` | Delete the "Scan screen for QR" button and `fromScreen()`. QR tab keeps only "Choose image…". |
| `.github/workflows/build.yml` | Delete the Linux Xvfb step (only needed for xcap capture tests). |
| `README.md` | Remove xcap from the tech table; update the QR/scan description. |

### Verification
- `cargo build` succeeds; `Cargo.lock` no longer contains `xcap`, `dlopen2`, `dlopen2_derive`, `dbus`, `libdbus-sys`, `xcb`.
- `cargo test` green (qr `decode_image_bytes` / fixture test retained).
- `npx svelte-check` 0 errors; `npm run build` succeeds.

## 3. Part B — Sans-IO refactor (method A: parameter injection)

### Principle
Core modules become **pure functions**: they never touch `fs`, `rand`, `uuid`, or the clock. `commands.rs` is the **single I/O edge / composition root** — it generates randomness (salt, nonce) and ids (uuid), reads the clock, and performs all file I/O via `storage`, then calls the pure core. This is the most sans-IO-faithful approach: the core holds no I/O references and receives entropy/time as data.

### Invariant: ID assignment at the edge
No core code generates an id. Every `Account` leaves the core with `id == ""` (empty string). The edge (`commands.rs`) assigns a fresh `Uuid::new_v4()` to every account immediately before it enters the vault. The vault therefore always contains accounts with non-empty ids; the empty-id state exists only transiently inside a single command body.

### New / changed interfaces

**`vault/kdf.rs`** — unchanged (already pure: `derive_key(password, salt, params)`).

**`vault/crypto.rs`** — unchanged (already pure: `encrypt(key, header, plaintext)`, `decrypt(password, bytes)`, `decrypt_with_key(password, bytes)` operate on byte slices; nonce comes from `header.nonce`).

**`model.rs`**
- `Account::new(issuer: String, label: String, secret: String) -> Account` — sets `id = String::new()`. Remove the `uuid` call. (Defaults: algorithm SHA1, digits 6, period 30, kind "totp" — unchanged.)

**`otpauth.rs`**
- `parse_otpauth(uri: &str) -> Result<Account>` — unchanged signature, but the returned `Account` has `id == ""` (no uuid minted). All parsing behavior (URL-decode, issuer/label, defaults, strict algorithm/digits/period, secret validation) unchanged.

**`migration.rs`**
- `parse_migration(uri: &str) -> Result<Vec<Account>>` — unchanged signature, but every returned `Account` has `id == ""`. All protobuf/base64/enum-mapping behavior unchanged.

**`vault/mod.rs`** — convert `Vault` to a pure in-memory state machine:
- `Vault::new() -> Vault` — locked (unchanged).
- `Vault::create_unlocked(password: &[u8], salt: [u8;16], kdf: KdfParams) -> Result<Vault>` — pure: derive key, empty accounts, store salt+kdf. **No fs, no rng** (salt injected). Returns an unlocked `Vault`.
- `Vault::unlock_from_bytes(file_bytes: &[u8], password: &[u8]) -> Result<Vault>` — pure: `decrypt_with_key` the bytes, parse accounts + recover salt/kdf from header. **No fs** (bytes injected).
- `vault.lock(&mut self)` — clears `Unlocked` (zeroizes key) — unchanged.
- `vault.is_unlocked()`, `vault.accounts() -> Result<&[Account]>`, `vault.snapshot() -> Result<Vec<Account>>` — pure getters (unchanged).
- `vault.add(&mut self, account: Account) -> Result<()>` — mutate in-memory only (Err if locked). **No fs.**
- `vault.remove(&mut self, id: &str) -> Result<()>` — mutate in-memory only. **No fs.**
- `vault.serialize(&self, nonce: [u8;12]) -> Result<Vec<u8>>` — pure: build `VaultHeader { version, kdf, salt, nonce }`, encrypt current accounts with the stored key. **No rng** (nonce injected). Returns the bytes to persist.
- **Remove** `persist()` and the fs path in `unlock`; remove `parse_header_only` fs coupling (header parsing for salt/kdf recovery moves into `unlock_from_bytes`, operating on the injected bytes).

**`backup.rs`** — make pure:
- `export_encrypted(accounts: &[Account], password: &[u8], salt: [u8;16], nonce: [u8;12]) -> Result<Vec<u8>>` — pure (salt+nonce injected; returns bytes; no fs, no rng).
- `import_encrypted(file_bytes: &[u8], password: &[u8]) -> Result<Vec<Account>>` — pure (bytes injected; no fs).

**`qr.rs`** — pure only:
- Keep `decode_image_bytes(&[u8]) -> Result<Vec<String>>` (already pure).
- Remove `decode_image_file` (its `std::fs::read` moves to the edge) and `capture_region` (removed in Part A). `qr.rs` ends up with a single pure function.

**`storage.rs`** — unchanged (the fs I/O adapter: `read_file`, `write_atomic`, `exists`).

**`commands.rs`** — becomes the composition root / edge. Add private helpers:
- `fn random_salt() -> [u8;16]` and `fn random_nonce() -> [u8;12]` (the only `rand::thread_rng()` calls in the codebase).
- `fn new_id() -> String` (the only `Uuid::new_v4()` call).
- `now_unix()` stays (the only `SystemTime::now()`).

Command bodies orchestrate pure core + I/O:
- `create_vault(pw)`: `salt=random_salt()` → `v=Vault::create_unlocked(pw, salt, KdfParams::default())` → store `v` in state → `bytes=v.serialize(random_nonce())` → `storage::write_atomic(path, bytes)`.
- `unlock(pw)`: `bytes=storage::read_file(path)` → `v=Vault::unlock_from_bytes(bytes, pw)` → store in state.
- `add_manual(issuer,label,secret)`: validate via `secret::decode_secret` → `acc=Account::new(...)` → `acc.id=new_id()` → `v.add(acc)` → `bytes=v.serialize(random_nonce())` → `write_atomic`.
- `add_from_uri(uri)`: `acc=parse_otpauth(uri)?` → `acc.id=new_id()` → `v.add(acc)` → serialize → write.
- `import_migration(uri, selected_indices)`: `accts=parse_migration(uri)?` → for each selected index, `acc.id=new_id()` → `v.add(acc)` → after all, `serialize(random_nonce())` **once** → write. Returns count.
- `remove_account(id)`: `v.remove(id)` → serialize → write.
- `export_backup(path,pw)`: `accts=v.snapshot()?` → `salt=random_salt()`, `nonce=random_nonce()` → `bytes=backup::export_encrypted(accts,pw,salt,nonce)?` → `storage::write_atomic(path,bytes)`.
- `import_backup(path,pw)`: `bytes=storage::read_file(path)?` → `accts=backup::import_encrypted(bytes,pw)?` → for each, keep its existing id (backups carry ids) → `v.add` → serialize once → write. Returns count.
- `decode_qr_file(path)`: `bytes=storage::read_file(path)?` → `qr::decode_image_bytes(bytes)?` → filter `starts_with("otpauth")`.
- `list_accounts` / `current_codes` / `lock` / `is_unlocked` / `vault_exists` / `preview_migration` — logic unchanged (preview maps `parse_migration` → `AccountView`; ids may be empty in preview, which is fine — the UI selects by index).

`AppState { vault: Mutex<Vault>, path: PathBuf }` is unchanged; the difference is that mutation+serialize+write now happen in the command body rather than inside `Vault::persist`.

### Persistence-failure semantics
Preserve current behavior: a command mutates the in-memory `Vault`, then serializes and writes; if `write_atomic` returns `Err`, the command returns that `Err`. (Same as today, where `persist()` ran after the in-memory mutation.) No rollback is introduced — out of scope.

## 4. Testing strategy

Sans-IO purity makes core tests deterministic and filesystem-free.

### Core unit tests (rewritten to pure interfaces)
- **`vault/mod.rs`**: use fixed salt/nonce — `create_unlocked(pw, [9u8;16], kdf)` → `add(acc)` → `bytes = serialize([3u8;12])` → `unlock_from_bytes(bytes, pw)` → assert accounts. No temp files. Wrong-password / tamper tests assert `unlock_from_bytes(tampered, ...)` → `Err(Crypto)`.
- **`backup.rs`**: `export_encrypted(accts, pw, salt, nonce)` → `import_encrypted(bytes, pw)` → assert round-trip; wrong password → `Err(Crypto)`. No temp files.
- **`model.rs`**: `Account::new(...)` → assert `id == ""` and default fields.
- **`otpauth.rs` / `migration.rs`**: update assertions to expect `id == ""`; all other parsed-field assertions unchanged (RFC/otpauth/protobuf behavior preserved).

### Unchanged, must stay green
`totp.rs` (RFC 6238 vectors), `kdf.rs`, `crypto.rs` (AAD tamper detection), `secret.rs` (Base32), `qr.rs` (`decode_image_bytes` + committed fixture), `storage.rs` (atomic-write disk round-trip — now the sole guardian of on-disk persistence). `commands.rs` `to_view`/`to_code_view` purity tests including the "no secret in serialized view" assertion.

### Edge (commands.rs)
The fs orchestration needs the Tauri runtime → not unit-tested (as today); covered by `cargo build` + manual verification.

### Process
TDD: first update the tests to the new pure signatures (compile-fails / RED), then refactor the core until green.

## 5. Behavior preservation (zero user-facing change)
- **Crypto:** same Argon2id + AES-256-GCM, same header/AAD; salt/nonce still freshly generated per create/save (now at the edge); nonce never reused.
- **Security:** opaque `AppError::Crypto`; master key zeroized on lock; secrets never cross IPC — all unchanged.
- **Features:** unlock / add (manual, QR image, GA import) / remove / encrypted backup behave identically. GA import now serializes once after adding all selected (faster; same result).
- **Dependencies:** `Cargo.lock` shrinks by the xcap subtree; all other deps unchanged.

## 6. Out of scope / future
- Trait-based DI, rollback-on-write-failure, multi-monitor region-select QR (the removed feature), Svelte 5 upgrade (separate audit item).
