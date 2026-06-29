# Design Spec — Custom Vault Paths, Key-File Authentication, and Secret Export

- **Date:** 2026-06-29
- **Status:** Approved (design); pending implementation plan
- **Scope:** Three user-facing features layered on the existing VaultOTP app, plus the
  on-disk-format and crypto changes they require.

---

## 1. Goals

1. **Custom vault paths / multiple vaults** — read and save the encrypted vault from
   user-chosen locations, and switch between multiple vault files (document-style app).
2. **Key-file authentication** — in addition to a master password, support a key file as
   a credential. A vault is protected by one of three modes: password only, key file only,
   or password **and** key file (both required).
3. **Export selected account secrets** — multi-select accounts and export their TOTP
   secrets in interoperable formats (otpauth QR PNG, otpauth text, Google Authenticator
   migration QR).

## 2. Non-goals / explicit notes

- **No database. No SQL.** This project has no PostgreSQL/SQLite/ORM of any kind.
  Persistence is exactly two things: the encrypted vault file
  (`src-tauri/src/storage.rs` → AES-256-GCM blob) and non-secret UI settings in browser
  `localStorage` (`src/lib/settings.ts`). The only code named "migration" is Google
  Authenticator `otpauth-migration://` import (`src-tauri/src/migration.rs`), not DB
  migrations. **Therefore this work produces no raw SQL and no table/column comments** —
  there are no tables or columns. This note exists so the standing "DB ops → Postgres raw
  SQL with comments" convention is on record as not-applicable here.
- **"Either-or" (password OR key, independently) is explicitly dropped.** The chosen
  password+key mode is **AND** (composite, both required), not OR. See §6.
- **Encrypted backup stays password-only.** The existing `export_backup`/`import_backup`
  (`src-tauri/src/backup.rs`) is an independent portable artifact and is out of scope for
  key files (YAGNI).

## 3. Current architecture (evidence)

- **Single fixed vault path.** `vault_path()` returns `app_config_dir()/vault.bin`
  (`src-tauri/src/main.rs:35-40`); `AppState` holds a single immutable
  `path: PathBuf` (`src-tauri/src/commands.rs:11-14`). Every read/write flows through
  `state.path`.
- **Direct key derivation.** `derive_key(password, salt, kdf)` = `Argon2id(SHA-256(password))`
  — a KeePass-style composite-key pattern, currently password-only
  (`src-tauri/src/vault/kdf.rs:32-55`). The derived 32-byte key encrypts the accounts JSON
  directly with AES-256-GCM.
- **File layout.** `MAGIC "ATOTP1\0" (7) | header_len: u32 LE (4) | header_json | ciphertext+tag`,
  where `MAGIC | header_len | header_json` is the AEAD AAD
  (`src-tauri/src/vault/crypto.rs:8,23-31`). Header JSON = `{version, kdf, salt, nonce}`.
- **Sans-IO core.** All randomness (`random_salt`, `random_nonce`) and filesystem access
  live at the commands edge (`src-tauri/src/commands.rs:50-60`); the `vault`/`crypto`/`kdf`
  modules are pure (bytes + injected values in, bytes out). This boundary is a deliberate,
  audited property (`docs/superpowers/audits/2026-06-28-dependency-and-sansio-audit.md`).
- **Secrets never cross IPC to the renderer.** Commands return computed codes only;
  `CodeView` has no secret field and a test asserts the serialized form contains no
  `"secret"` (`src-tauri/src/commands.rs:201-209`). `model::Account` zeroizes its secret on
  drop (`src-tauri/src/model.rs`).
- **Frontend.** Unlock/create flow in `src/routes/Unlock.svelte` calls `unlock`/`createVault`
  via `src/lib/ipc.ts`; the dialog plugin (`@tauri-apps/plugin-dialog`) is already wired for
  backup open/save in `src/components/Settings.svelte`, and the capability allows
  `dialog:allow-open` / `dialog:allow-save` (`src-tauri/capabilities/default.json:6-10`).
- **Existing encoders/decoders.** `parse_otpauth` exists but there is **no otpauth builder**
  (`src-tauri/src/otpauth.rs:34`). QR **decode** exists via `rqrr`+`image`
  (`src-tauri/src/qr.rs:3`) but there is **no QR encoder**. `prost = "0.13"` is already a
  dependency used to **decode** GA migration protobuf (`src-tauri/Cargo.toml:23`,
  `src-tauri/src/migration.rs`).

## 4. Feature 1 — Custom paths / multiple vaults

The app becomes document-based: a vault picker precedes unlock, with a "current vault" and
a recent-vaults list.

### 4.1 State

- `AppState.path: PathBuf` → `AppState.current: Mutex<Option<PathBuf>>` (the open vault),
  alongside the existing `vault: Mutex<Vault>` (`src-tauri/src/commands.rs:11-14`).
- **Recent list + last-opened** persisted in a Rust-managed `app_config_dir()/config.json`
  (paths are non-secret), read at startup. Chosen over `localStorage` so it is app-level and
  survives a webview-storage clear. Schema: `{ recent: [string], last: string|null }`,
  most-recent-first, delisted entries pruned when the file no longer exists.

### 4.2 Paths and capabilities

- Vault files and key files are read/written **in Rust** via `storage::read_file` /
  `storage::write_atomic` (`src-tauri/src/storage.rs`), the same pattern `decode_qr_file` and
  `export_backup` already use for arbitrary paths (`src-tauri/src/commands.rs:137-184`).
  Native `std::fs` is not gated by the Tauri JS ACL, so **arbitrary vault/key-file paths need
  no new capability**. Only the file-picker dialogs are used from the frontend, and those are
  already allowed.

### 4.3 UX

- **New `src/routes/VaultPicker.svelte`** (startup, before unlock): list recent vaults
  (path + existence), "Open vault…" (open dialog), "Create new vault…" (save dialog → choose
  path + filename, default suggestion `vault.bin`).
- **Save As** = "Save a copy" semantics: write the current serialized vault to a new path;
  the active vault stays the original file (`current` unchanged).

## 5. Feature 2 — Key-file authentication

### 5.1 Protection modes (chosen at creation, changeable in Settings)

| Mode | header `mode` | Content key = | Unlock needs |
|---|---|---|---|
| Password only | `password` | `Argon2id(SHA-256(pw), salt)` — **identical to v1** | password |
| Key file only | `keyfile` | `Argon2id(SHA-256(keyfile_bytes), salt)` | key file |
| Password + key file (AND) | `composite` | `Argon2id(SHA-256( SHA-256(pw) ‖ SHA-256(keyfile) ), salt)` | both together |

Each vault has exactly **one** credential configuration → exactly one way to unlock.
Because there is never more than one unlock path (no OR), **no key-slot/VMK envelope is
needed**: the content key is derived directly, exactly like the current v1 mechanism, with
only the KDF input varying by mode. This is the chosen simplification over an envelope scheme.

### 5.2 Key-file source

Both supported (user choice at setup):
- **Use existing file** — any file's bytes become the key material (KeePass-classic). UI
  warns that a low-entropy/public file is a weak factor; recommends generating one.
- **App-generated key file** — `CSPRNG(32 bytes)` written via save dialog. High-entropy,
  content never changes.

### 5.3 Crypto / core changes (pure, sans-IO preserved)

- `src/vault/kdf.rs`: keep `derive_key(material, salt, kdf)`; vary `material` by mode:
  - password → `pw_bytes`
  - keyfile → `keyfile_bytes`
  - composite → `SHA-256(pw_bytes) ‖ SHA-256(keyfile_bytes)` (then the existing SHA-256 +
    Argon2id steps run as today).
- `src/vault/mod.rs`: `create_unlocked(...)` and `unlock_from_bytes(bytes, Credential)` take
  `Credential = Password(&[u8]) | KeyFile(&[u8]) | Both { password, keyfile }`.
- `src/vault/crypto.rs`: header JSON gains `#[serde(default)] mode: Mode` (default
  `password`). `mode` is inside the AAD region → authenticated; tampering with `mode` fails
  the GCM tag (prevents downgrade attacks). File layout and MAGIC unchanged.
- **Changing credentials** (add/remove key file, change password, switch mode) re-derives the
  content key and re-encrypts the (small) accounts blob with a fresh nonce. Cheap; no envelope.

### 5.4 Backward compatibility / migration ("read old, write new")

- A v1 file has no `mode` field → deserializes as `mode = password` and unlocks with the
  existing password path. No re-encryption on read.
- On the next write, the header is re-serialized **with** the `mode` field. Only when the
  user actually switches to `keyfile`/`composite` does the content key change and the blob get
  re-encrypted.
- Versioning: keep a single MAGIC; rely on the `mode` field (with serde default) rather than a
  hard version bump. A `version` field remains present for forward signaling.

### 5.5 UX

- Create-vault wizard gains a "protection mode" step (password / key file / both).
- "Generate key file…" → save dialog to choose where the key file is stored.
- `src/routes/Unlock.svelte` reads the opened vault's `mode` (returned by `open_vault`) and
  shows the matching fields: password, key-file picker, or both.
- `src/components/Settings.svelte` gains a "Key file" section: add existing file…, generate
  key file…, remove key file, change password, and "Open a different vault…" / "Save vault
  as…". A note warns: losing **all** required credentials means **no recovery**.

## 6. Feature 3 — Export selected account secrets

### 6.1 Security posture (deliberate relaxation)

Exporting raw TOTP secrets intentionally breaks the "secrets never leave the vault"
invariant. To minimize blast radius, **all encoding and file-writing happen in Rust**: the
frontend sends only `(selected ids, destination path, format)`; Rust reads secrets from the
in-memory unlocked vault, encodes, and `write_atomic`s the file. **Secrets never reach the
webview/renderer.** (Consequence: QR is written to a PNG file by Rust, not displayed
on-screen — displaying a QR would route the secret-bearing image through the renderer.)

Export requires an unlocked vault and a one-time confirmation warning ("writes plaintext
secrets; store securely / delete afterward").

### 6.2 Formats (all three supported)

1. **otpauth:// QR PNG** — one PNG per selected account, each encoding
   `otpauth://totp/{issuer}:{label}?secret=…&issuer=…&algorithm=…&digits=…&period=…`.
   Scannable by phone authenticators. Files named per account (issuer/label, sanitized).
2. **otpauth:// text (.txt)** — one file, one otpauth URI per line. Re-importable via the
   existing `add_from_uri` (`src-tauri/src/commands.rs:127`).
3. **Google Authenticator single migration QR** — one `otpauth-migration://offline?data=…`
   QR PNG encoding all selected accounts (protobuf + base64), mirroring the import format in
   `src-tauri/src/migration.rs`.

### 6.3 Core changes

- `src/otpauth.rs`: add `build_otpauth(account) -> String` (inverse of `parse_otpauth`;
  percent-encode label/issuer, emit params; pure).
- `src/qr.rs`: add `encode_png(data: &str) -> Result<Vec<u8>>` using a QR **encoder** crate,
  rasterized to PNG via the already-present `image` crate (pure).
- `src/migration.rs`: add `build_migration(accounts) -> String` (prost-encode the same
  protobuf currently decoded, base64, wrap as `otpauth-migration://`; pure). Reuses existing
  `prost` dependency.
- New command `export_secrets(ids: Vec<String>, path: String, format: ExportFormat) -> Result<usize>`
  at the commands edge: pulls secrets from the unlocked vault, calls the pure encoders, writes
  via `write_atomic`.

### 6.4 UX

- `src/routes/Main.svelte` gains a multi-select mode (checkboxes on `AccountCard`) and an
  "Export selected…" action → format chooser → save dialog → write. Shows the plaintext-secret
  warning before writing.

## 7. Dependencies

- **One new crate**: a QR **encoder** (e.g. `qrcode`). Per the project's dependency-provenance
  discipline (`docs/superpowers/audits/2026-06-28-dependency-and-sansio-audit.md`), the
  implementation plan must add an audit entry (provenance, license, CVE/RustSec check) before
  adoption.
- No other new dependencies: `image`, `base32`, `prost`, `rand`, `sha2`, `aes-gcm`,
  `tauri-plugin-dialog` are already present (`src-tauri/Cargo.toml`).

## 8. Commands / IPC surface (summary)

New or changed (`src-tauri/src/commands.rs`, `src/lib/ipc.ts`, handler list in
`src-tauri/src/main.rs:66-82`):

| Command | Purpose |
|---|---|
| `list_recent_vaults()` | recent paths + which exist |
| `create_vault_at(path, mode, password?, keyfile_path?)` | create a vault at a chosen path → set current |
| `open_vault(path)` | set current; return its `mode` so the UI knows what to ask |
| `unlock(credential…)` | unlock current via password / key file / both |
| `add_keyfile(keyfile_path)` / `generate_keyfile(out_path)` / `remove_keyfile()` | manage the key-file slot on the unlocked vault |
| `change_password(new_password)` | re-derive + re-encrypt under a new password |
| `save_vault_as(path)` | write a copy of the current vault to a new path (current unchanged) |
| `current_vault_path()` | display the active vault path |
| `export_secrets(ids, path, format)` | export selected account secrets (Rust-side encode + write) |

Superseded: `vault_exists`, `unlock`, `create_vault` (replaced by the path-aware variants).
All mutating commands continue to write to `state.current`.

## 9. Security considerations

- Unlock failures stay the single opaque `AppError::Crypto` — no oracle about which check
  failed; wrong password and wrong key file are indistinguishable
  (`src-tauri/src/vault/crypto.rs:79-82`).
- All key material wrapped in `Zeroizing` / zeroized on drop, as today
  (`src-tauri/src/vault/mod.rs:17-21`).
- `mode` authenticated as AAD → no silent downgrade.
- Export is the one deliberate secret-egress path: unlock-gated, Rust-only encoding,
  explicit warning, atomic write.
- Generated key files written with `write_atomic`; UI states there is no recovery if all
  required credentials are lost.

## 10. Testing strategy (TDD, hermetic — matches existing style)

- **KDF/crypto:** round-trip for each mode (password / keyfile / composite); wrong-credential
  fails; tamper of ciphertext, header, and `mode` fails.
- **Migration of format:** read a v1 fixture with a password, save, re-open (now carries
  `mode`); add a key file → re-open requires the new credential(s).
- **Multi-vault:** create at paths A and B, switch, each unlocks independently; recent-list
  pruning when a file is missing.
- **otpauth build/parse round-trip:** `build_otpauth` output re-parses via `parse_otpauth` to
  the same account.
- **QR encode→decode round-trip:** `encode_png` output decodes via existing
  `decode_image_bytes` back to the same otpauth URI.
- **GA migration build→parse round-trip:** `build_migration` output re-parses via existing
  `parse_migration`.
- **Export command:** selected ids only; output file contains exactly the selected accounts;
  unlock required.

## 11. Open risks

- QR-encoder crate choice + audit entry must be settled in the plan before coding.
- File-naming collisions when exporting multiple per-account PNGs (same issuer/label) — need a
  disambiguation rule (e.g., append id suffix).
- Sanitizing issuer/label for filenames and for otpauth path/percent-encoding.
