# Account Secret Export — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the user multi-select accounts and export their TOTP secrets in three interoperable formats — one otpauth QR PNG per account, a single otpauth text file, or one Google Authenticator migration QR — with all encoding and file writing done in Rust so secrets never reach the webview.

**Architecture:** Add pure encoders to the sans-IO core: `otpauth::build_otpauth` (inverse of the existing `parse_otpauth`), `qr::encode_png` (render via the already-present `image` crate from a new `qrcode` dependency), and `migration::build_migration` (reuse the existing hand-written prost `MigrationPayload`/`OtpParameters` messages). A single Tauri command `export_secrets(ids, path, format)` at the commands edge reads secrets from the in-memory unlocked vault, calls the pure encoders, and writes files; the frontend only sends account ids + destination + format.

**Tech Stack:** Rust (Tauri 2), `qrcode` (NEW), `image` 0.25, `prost` 0.13, `base32` (all already present except `qrcode`); Svelte 4 + TS; tests via `cargo test` and `vitest`.

This is Plan 3 of 3 (spec: `docs/superpowers/specs/2026-06-29-custom-path-keyfile-export-design.md`). Plans 1 (key-file auth) and 2 (multi-vault paths) are already implemented in the working tree.

## Global Constraints

- **Sans-IO core:** the encoders in `otpauth.rs`/`qr.rs`/`migration.rs` are PURE (bytes/strings in, bytes/strings out) — no filesystem, no rand, no clock. Only `export_secrets` in `commands.rs` does I/O.
- **Secrets never reach the renderer:** the frontend sends only `(ids, path, format)`; Rust pulls secrets from the unlocked vault, encodes, and writes. Never return a secret or an otpauth/migration URI to the frontend. QR is written to a PNG file by Rust, never displayed on-screen.
- **One new dependency only:** `qrcode` (QR *encoder*). It MUST get an audit entry (provenance, license, RustSec/CVE check) appended to `docs/superpowers/audits/2026-06-28-dependency-and-sansio-audit.md`. Use `qrcode` with `default-features = false` and build the PNG with the already-present `image` crate to avoid an `image`-version conflict. No other new crates.
- **No database / no SQL.**
- **Export is unlock-gated:** `export_secrets` reads the in-memory unlocked vault; a locked vault yields `AppError::Crypto`.
- **Opaque crypto failures** stay `AppError::Crypto`; encode/IO failures use `AppError::Other(String)` or the existing `AppError::QrDecode`/`Migration` where apt (pick one and be consistent — see tasks).
- **Rust test command:** `cargo test --manifest-path src-tauri/Cargo.toml <filter>` — BINARY crate; never pass `--lib`.
- **Frontend:** `npm test` (vitest); `npx svelte-check --tsconfig ./tsconfig.json`; `npm run build`. No Svelte component-test harness — `.svelte` verified by svelte-check + build.
- **Format identifiers (exact strings):** `"otpauth_qr"` (one PNG per account → `path` is a DIRECTORY), `"otpauth_text"` (`path` is a `.txt` file), `"google_migration"` (`path` is a single `.png` file).

## Current State (verified)

- `src-tauri/src/otpauth.rs`: `parse_otpauth(uri) -> Result<Account>` exists; a private `url_decode`. NO builder.
- `src-tauri/src/qr.rs`: `decode_image_bytes(bytes) -> Result<Vec<String>>` (via `rqrr` + `image`). NO encoder.
- `src-tauri/src/migration.rs`: hand-written prost `Message` structs `OtpParameters` (fields: `secret: Vec<u8>`, `name`, `issuer`, `algorithm: i32` [1=SHA1,2=SHA256,3=SHA512], `digits: i32` [1=SIX,2=EIGHT], `r#type: i32` [1=HOTP,2=TOTP], `counter: i64`) and `MigrationPayload` (`otp_parameters`, `version`, `batch_size`, `batch_index`, `batch_id`); `parse_migration(uri)`; private `base64_decode`. A test-only `base64_encode` exists in `#[cfg(test)]`.
- `src-tauri/src/model.rs`: `Account { id, issuer, label, secret, algorithm: Algorithm, digits: u32, period: u64, kind: String }`; `enum Algorithm { Sha1, Sha256, Sha512 }`.
- `src-tauri/src/commands.rs`: `AppState { vault: Mutex<Vault>, current: Mutex<PathBuf>, config_dir: PathBuf }`; `Vault::snapshot() -> Result<Vec<Account>>` returns the unlocked accounts (clone); `random_nonce()`/helpers exist; `lib.rs`/`main.rs` exposes `pub fn otpauth_decode`.
- `src/routes/Main.svelte`: renders `AccountCard` per code, has `Settings`/`AddMenu`; `currentCodes`/`removeAccount` via ipc.

---

## File Structure

- `src-tauri/src/otpauth.rs` — add pure `build_otpauth(&Account) -> String` + private `percent_encode`.
- `src-tauri/Cargo.toml` — add `qrcode` (default-features=false).
- `src-tauri/src/qr.rs` — add pure `encode_png(data: &str) -> Result<Vec<u8>>` (render QR modules into an `image` Luma8 buffer, encode PNG).
- `docs/superpowers/audits/2026-06-28-dependency-and-sansio-audit.md` — append a `qrcode` audit entry.
- `src-tauri/src/migration.rs` — add pure `build_migration(&[Account]) -> String`; promote a non-test `base64_encode`; add private `percent_encode_b64` for the `data` param.
- `src-tauri/src/commands.rs` — add `export_secrets` command + a pure `sanitize_filename` helper; register in `main.rs`.
- `src-tauri/src/main.rs` — register `export_secrets`.
- `src/lib/ipc.ts` + `src/lib/ipc.test.ts` — `exportSecrets(ids, path, format)` binding.
- `src/routes/Main.svelte` (+ `src/components/AccountCard.svelte`) — multi-select mode + "Export selected…" flow.

---

### Task 1: `build_otpauth` (otpauth URI builder)

**Files:**
- Modify: `src-tauri/src/otpauth.rs`
- Test: `src-tauri/src/otpauth.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Produces: `pub fn build_otpauth(a: &Account) -> String` — emits `otpauth://totp/{issuer:label|label}?secret=..&issuer=..&algorithm=..&digits=..&period=..`, percent-encoding label/issuer. Round-trips through the existing `parse_otpauth`.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src-tauri/src/otpauth.rs`:

```rust
    #[test]
    fn build_then_parse_round_trips() {
        let mut a = Account::new("GitHub".into(), "alice@example.com".into(), "JBSWY3DPEHPK3PXP".into());
        a.algorithm = Algorithm::Sha256;
        a.digits = 8;
        a.period = 60;
        let uri = build_otpauth(&a);
        assert!(uri.starts_with("otpauth://totp/"));
        let back = parse_otpauth(&uri).unwrap();
        assert_eq!(back.issuer, "GitHub");
        assert_eq!(back.label, "alice@example.com");
        assert_eq!(back.secret, "JBSWY3DPEHPK3PXP");
        assert_eq!(back.algorithm, Algorithm::Sha256);
        assert_eq!(back.digits, 8);
        assert_eq!(back.period, 60);
    }

    #[test]
    fn build_without_issuer_omits_issuer_param() {
        let a = Account::new(String::new(), "solo".into(), "JBSWY3DPEHPK3PXP".into());
        let uri = build_otpauth(&a);
        assert!(!uri.contains("issuer="));
        let back = parse_otpauth(&uri).unwrap();
        assert_eq!(back.label, "solo");
        assert_eq!(back.issuer, "");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml otpauth::tests::build_then_parse_round_trips`
Expected: FAIL — `cannot find function build_otpauth`.

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/otpauth.rs`, add (after `parse_otpauth`):

```rust
/// Percent-encode all bytes except RFC 3986 unreserved (ALPHA / DIGIT / -._~).
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        let unreserved = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~');
        if unreserved {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

/// Build an `otpauth://totp/...` URI for an account. Inverse of `parse_otpauth`.
pub fn build_otpauth(a: &Account) -> String {
    let label_path = if a.issuer.is_empty() {
        percent_encode(&a.label)
    } else {
        format!("{}:{}", percent_encode(&a.issuer), percent_encode(&a.label))
    };
    let algo = match a.algorithm {
        Algorithm::Sha1 => "SHA1",
        Algorithm::Sha256 => "SHA256",
        Algorithm::Sha512 => "SHA512",
    };
    let mut uri = format!("otpauth://totp/{}?secret={}", label_path, a.secret);
    if !a.issuer.is_empty() {
        uri.push_str(&format!("&issuer={}", percent_encode(&a.issuer)));
    }
    uri.push_str(&format!("&algorithm={}&digits={}&period={}", algo, a.digits, a.period));
    uri
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml otpauth`
Expected: PASS (new + existing parse tests). Then `cargo build --manifest-path src-tauri/Cargo.toml` (a `dead_code` warning for `build_otpauth` until Task 4 is expected).

- [ ] **Step 5: Commit** — SKIP under no-git.

```bash
git add src-tauri/src/otpauth.rs
git commit -m "feat(otpauth): build_otpauth URI builder (inverse of parse)"
```

---

### Task 2: `qrcode` dependency + `encode_png`

**Files:**
- Modify: `src-tauri/Cargo.toml` (add `qrcode`)
- Modify: `src-tauri/src/qr.rs` (add `encode_png`)
- Modify: `docs/superpowers/audits/2026-06-28-dependency-and-sansio-audit.md` (append qrcode entry)
- Test: `src-tauri/src/qr.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Produces: `pub fn encode_png(data: &str) -> Result<Vec<u8>>` — a PNG byte vector of a QR code encoding `data`. Round-trips through the existing `decode_image_bytes`.

- [ ] **Step 1: Add and verify the dependency**

Add to `src-tauri/Cargo.toml` `[dependencies]`:

```toml
qrcode = { version = "0.14", default-features = false }
```

Run `cargo build --manifest-path src-tauri/Cargo.toml` once to fetch it. THEN verify the actual `qrcode` API you will use against the real crate (do NOT guess): inspect the fetched source/docs — e.g. `cargo doc -p qrcode --no-deps` then read, or read `~/.cargo/registry/src/*/qrcode-0.14*/src/lib.rs`. Confirm the exact names for: constructing (`QrCode::new(bytes)`), getting the module count (`code.width()`), and reading modules as light/dark (e.g. `code.to_colors() -> Vec<Color>` with `Color::Dark`/`Color::Light`, or `code.into_colors()`). Adapt Step 3's code to the real API if the names differ — the goal (iterate modules → Luma8 pixels) is fixed; the method names must match the crate.

- [ ] **Step 2: Write the failing test**

Add to the `tests` module in `src-tauri/src/qr.rs`:

```rust
    #[test]
    fn encode_png_round_trips_through_decode() {
        let uri = "otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example";
        let png = encode_png(uri).unwrap();
        // PNG magic
        assert_eq!(&png[..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        let decoded = decode_image_bytes(&png).unwrap();
        assert!(decoded.iter().any(|s| s == uri));
    }

    #[test]
    fn encode_png_rejects_oversized_input() {
        // QR has a capacity ceiling; a huge string must error, not panic.
        let big = "a".repeat(10_000);
        assert!(encode_png(&big).is_err());
    }
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml qr::tests::encode_png_round_trips_through_decode`
Expected: FAIL — `cannot find function encode_png`.

- [ ] **Step 4: Write minimal implementation**

In `src-tauri/src/qr.rs`, add (adapting method names to the API you verified in Step 1):

```rust
use image::{ImageBuffer, Luma};
use std::io::Cursor;

/// Render `data` as a QR code and return PNG bytes. Pure (no I/O).
/// Each module is scaled to an 8x8 pixel block with a 4-module quiet zone.
pub fn encode_png(data: &str) -> Result<Vec<u8>> {
    let code = qrcode::QrCode::new(data.as_bytes()).map_err(|_| AppError::QrDecode)?;
    let modules = code.width(); // module count per side
    let colors = code.to_colors(); // row-major, len == modules*modules; Color::Dark | Color::Light

    const SCALE: u32 = 8;
    const QUIET: u32 = 4; // modules
    let side = (modules as u32 + 2 * QUIET) * SCALE;
    let mut img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_pixel(side, side, Luma([255u8]));

    for (i, c) in colors.iter().enumerate() {
        let is_dark = matches!(c, qrcode::Color::Dark);
        if !is_dark { continue; }
        let mx = (i % modules) as u32;
        let my = (i / modules) as u32;
        let x0 = (QUIET + mx) * SCALE;
        let y0 = (QUIET + my) * SCALE;
        for dy in 0..SCALE {
            for dx in 0..SCALE {
                img.put_pixel(x0 + dx, y0 + dy, Luma([0u8]));
            }
        }
    }

    let mut buf = Vec::new();
    image::DynamicImage::ImageLuma8(img)
        .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|_| AppError::QrDecode)?;
    Ok(buf)
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml qr`
Expected: PASS (new + existing decode tests). Then `cargo build --manifest-path src-tauri/Cargo.toml`.

- [ ] **Step 6: Append the dependency audit entry**

Append a section to `docs/superpowers/audits/2026-06-28-dependency-and-sansio-audit.md` for `qrcode`: its version, source (crates.io), license (verify from the crate metadata — `cargo metadata` or the crate's `Cargo.toml` `license` field), transitive deps pulled in with `default-features=false`, and a RustSec/advisory check result (check https://rustsec.org / `cargo audit` if available, else note the manual check). State the reason it was added (QR PNG export) and that `default-features=false` avoids an `image`-version conflict. Record actual findings — do not write "TODO".

- [ ] **Step 7: Commit** — SKIP under no-git.

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/qr.rs docs/superpowers/audits/2026-06-28-dependency-and-sansio-audit.md
git commit -m "feat(qr): encode_png QR generator + qrcode dependency audit entry"
```

---

### Task 3: `build_migration` (Google Authenticator export URI)

**Files:**
- Modify: `src-tauri/src/migration.rs`
- Test: `src-tauri/src/migration.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Produces: `pub fn build_migration(accounts: &[Account]) -> Result<String>` — a single `otpauth-migration://offline?data=<url-encoded base64 protobuf>` URI. Round-trips through the existing `parse_migration`.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src-tauri/src/migration.rs`:

```rust
    #[test]
    fn build_then_parse_migration_round_trips() {
        let mut a = Account::new("Example".into(), "alice".into(),
            base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &[0x48,0x65,0x6c,0x6c,0x6f]));
        a.algorithm = Algorithm::Sha256;
        a.digits = 8;
        let uri = build_migration(&[a]).unwrap();
        assert!(uri.starts_with("otpauth-migration://offline?data="));
        let back = parse_migration(&uri).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].issuer, "Example");
        assert_eq!(back[0].label, "alice");
        assert_eq!(back[0].algorithm, Algorithm::Sha256);
        assert_eq!(back[0].digits, 8);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml migration::tests::build_then_parse_migration_round_trips`
Expected: FAIL — `cannot find function build_migration`.

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/migration.rs`, add a non-test `base64_encode` (standard alphabet; identical body to the test helper at the bottom of the file — move it OUT of `#[cfg(test)]` to module scope so both the encoder and tests share it; delete the duplicate test copy), a `data`-param percent encoder, and `build_migration`:

```rust
/// Standard-alphabet Base64 encoder (module scope; shared by build_migration and tests).
fn base64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        out.push(T[(b[0] >> 2) as usize] as char);
        out.push(T[(((b[0] & 0x03) << 4) | (b[1] >> 4)) as usize] as char);
        if chunk.len() > 1 { out.push(T[(((b[1] & 0x0f) << 2) | (b[2] >> 6)) as usize] as char); } else { out.push('='); }
        if chunk.len() > 2 { out.push(T[(b[2] & 0x3f) as usize] as char); } else { out.push('='); }
    }
    out
}

/// Percent-encode the base64 `data` value so it survives a URL round-trip
/// (encodes +, /, = and any non-unreserved byte). `crate::otpauth_decode`
/// reverses this on parse.
fn percent_encode_b64(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

/// Build a single-batch Google Authenticator migration URI for `accounts` (TOTP).
pub fn build_migration(accounts: &[Account]) -> Result<String> {
    if accounts.is_empty() {
        return Err(AppError::Migration);
    }
    let mut params = Vec::with_capacity(accounts.len());
    for a in accounts {
        let secret = crate::secret::decode_secret(&a.secret)?; // base32 -> raw bytes (Err(InvalidSecret) if bad)
        let algorithm = match a.algorithm {
            Algorithm::Sha1 => 1,
            Algorithm::Sha256 => 2,
            Algorithm::Sha512 => 3,
        };
        let digits = if a.digits == 8 { 2 } else { 1 };
        params.push(OtpParameters {
            secret,
            name: a.label.clone(),
            issuer: a.issuer.clone(),
            algorithm,
            digits,
            r#type: 2, // TOTP
            counter: 0,
        });
    }
    let payload = MigrationPayload {
        otp_parameters: params,
        version: 1,
        batch_size: 1,
        batch_index: 0,
        batch_id: 0,
    };
    let mut raw = Vec::new();
    payload.encode(&mut raw).map_err(|_| AppError::Migration)?;
    let b64 = base64_encode(&raw);
    Ok(format!("otpauth-migration://offline?data={}", percent_encode_b64(&b64)))
}
```

(After moving `base64_encode` to module scope, remove the now-duplicate `fn base64_encode` inside the `#[cfg(test)]` module so the tests use the shared one.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml migration`
Expected: PASS (new round-trip + the 4 existing tests). Then `cargo build --manifest-path src-tauri/Cargo.toml`.

- [ ] **Step 5: Commit** — SKIP under no-git.

```bash
git add src-tauri/src/migration.rs
git commit -m "feat(migration): build_migration GA export URI (single batch)"
```

---

### Task 4: `export_secrets` command

**Files:**
- Modify: `src-tauri/src/commands.rs` (add `export_secrets` + pure `sanitize_filename`)
- Modify: `src-tauri/src/main.rs` (register)
- Test: `src-tauri/src/commands.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: `otpauth::build_otpauth`, `qr::encode_png`, `migration::build_migration`, `Vault::snapshot`, `storage::write_atomic`.
- Produces (Tauri command): `export_secrets(state, ids: Vec<String>, path: String, format: String) -> Result<usize>`; pure helper `sanitize_filename(issuer: &str, label: &str) -> String`.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src-tauri/src/commands.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml export_secrets_text_writes_selected_only`
Expected: FAIL — `cannot find function export_secrets_inner` / `sanitize_filename`.

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/commands.rs`, add:

```rust
/// Pure: a filesystem-safe base name from issuer+label (ASCII alnum/-/_ kept, others → '_').
fn sanitize_filename(issuer: &str, label: &str) -> String {
    let mut s = String::new();
    let mut push_clean = |part: &str| {
        for ch in part.chars() {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' { s.push(ch); } else { s.push('_'); }
        }
    };
    push_clean(issuer);
    if !issuer.is_empty() && !label.is_empty() { s.push('-'); }
    push_clean(label);
    let trimmed = s.trim_matches('_').to_string();
    if trimmed.is_empty() { "account".to_string() } else { trimmed }
}

/// Shared logic for `export_secrets`, testable without a Tauri `State`.
fn export_secrets_inner(state: &AppState, ids: Vec<String>, path: String, format: String) -> Result<usize> {
    let all = state.vault.lock().unwrap().snapshot()?; // Err(Crypto) if locked
    let selected: Vec<Account> = all.into_iter().filter(|a| ids.contains(&a.id)).collect();
    if selected.is_empty() {
        return Err(AppError::Other("no accounts selected".into()));
    }
    match format.as_str() {
        "otpauth_text" => {
            let body = selected.iter().map(crate::otpauth::build_otpauth).collect::<Vec<_>>().join("\n");
            crate::storage::write_atomic(std::path::Path::new(&path), body.as_bytes())?;
            Ok(selected.len())
        }
        "google_migration" => {
            let uri = crate::migration::build_migration(&selected)?;
            let png = crate::qr::encode_png(&uri)?;
            crate::storage::write_atomic(std::path::Path::new(&path), &png)?;
            Ok(selected.len())
        }
        "otpauth_qr" => {
            // `path` is a directory; one PNG per account, de-duplicating base names.
            let dir = std::path::Path::new(&path);
            let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
            for a in &selected {
                let mut base = sanitize_filename(&a.issuer, &a.label);
                let mut name = format!("{base}.png");
                let mut n = 1;
                while used.contains(&name) {
                    n += 1;
                    base = format!("{}-{}", sanitize_filename(&a.issuer, &a.label), n);
                    name = format!("{base}.png");
                }
                used.insert(name.clone());
                let uri = crate::otpauth::build_otpauth(a);
                let png = crate::qr::encode_png(&uri)?;
                crate::storage::write_atomic(&dir.join(name), &png)?;
            }
            Ok(selected.len())
        }
        _ => Err(AppError::Other("unknown export format".into())),
    }
}

#[tauri::command]
pub fn export_secrets(state: tauri::State<AppState>, ids: Vec<String>, path: String, format: String) -> Result<usize> {
    export_secrets_inner(&state, ids, path, format)
}
```

In `src-tauri/src/main.rs`, add `commands::export_secrets,` to `generate_handler!`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml` (whole suite)
Expected: PASS. Then `cargo build --manifest-path src-tauri/Cargo.toml` → the Task 1-3 `dead_code` warnings (`build_otpauth`/`encode_png`/`build_migration`) clear now that `export_secrets` consumes them.

- [ ] **Step 5: Commit** — SKIP under no-git.

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(commands): export_secrets (text/qr/migration), secrets stay in Rust"
```

---

### Task 5: IPC binding + test

**Files:**
- Modify: `src/lib/ipc.ts`, `src/lib/ipc.test.ts`

**Interfaces:**
- Produces (TS): `exportSecrets(ids: string[], path: string, format: ExportFormat): Promise<number>` where `ExportFormat = "otpauth_qr" | "otpauth_text" | "google_migration"`.

- [ ] **Step 1: Write the failing test**

Add to `src/lib/ipc.test.ts` (extend the import line to include `exportSecrets`):

```ts
  it("exportSecrets forwards ids, path, format", async () => {
    invokeMock.mockResolvedValue(2);
    const n = await exportSecrets(["id1", "id2"], "/out.txt", "otpauth_text");
    expect(invokeMock).toHaveBeenCalledWith("export_secrets", { ids: ["id1", "id2"], path: "/out.txt", format: "otpauth_text" });
    expect(n).toBe(2);
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/lib/ipc.test.ts`
Expected: FAIL — `exportSecrets` not exported.

- [ ] **Step 3: Write minimal implementation**

Add to `src/lib/ipc.ts`:

```ts
export type ExportFormat = "otpauth_qr" | "otpauth_text" | "google_migration";
export const exportSecrets = (ids: string[], path: string, format: ExportFormat) =>
  invoke<number>("export_secrets", { ids, path, format });
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test -- src/lib/ipc.test.ts` → PASS. Then `npx svelte-check --tsconfig ./tsconfig.json` → no new errors.

- [ ] **Step 5: Commit** — SKIP under no-git.

```bash
git add src/lib/ipc.ts src/lib/ipc.test.ts
git commit -m "feat(ipc): exportSecrets binding"
```

---

### Task 6: Main multi-select + export flow

**Files:**
- Modify: `src/routes/Main.svelte`
- Modify: `src/components/AccountCard.svelte` (optional checkbox in select mode)

**Interfaces:**
- Consumes (TS): `exportSecrets`, `type ExportFormat` from `src/lib/ipc.ts`; `open`/`save` from `@tauri-apps/plugin-dialog`.

- [ ] **Step 1: Add select-mode + export UI to `Main.svelte`**

Read the ACTUAL current `src/routes/Main.svelte` and `src/components/AccountCard.svelte` first. Then add to `Main.svelte` `<script>`:

```ts
  import { exportSecrets, type ExportFormat } from "../lib/ipc";
  import { open, save } from "@tauri-apps/plugin-dialog";

  let selectMode = false;
  let selected = new Set<string>();
  let exportStatus = "", exportError = "";

  function toggleSelect(id: string) {
    if (selected.has(id)) selected.delete(id); else selected.add(id);
    selected = selected; // reassign for Svelte reactivity
  }

  async function doExport(format: ExportFormat) {
    exportError = ""; exportStatus = "";
    const ids = [...selected];
    if (!ids.length) { exportError = "Select at least one account"; return; }
    let path: string | null = null;
    if (format === "otpauth_qr") {
      const d = await open({ directory: true });
      path = typeof d === "string" ? d : null;
    } else {
      const def = format === "otpauth_text" ? "vaultotp-secrets.txt" : "vaultotp-migration.png";
      const f = await save({ defaultPath: def });
      path = typeof f === "string" ? f : null;
    }
    if (!path) return;
    try {
      const n = await exportSecrets(ids, path, format);
      exportStatus = `Exported ${n} account(s). Stored as PLAINTEXT secrets — keep the file safe and delete it when done.`;
      selectMode = false; selected = new Set();
    } catch (e) { exportError = String(e); }
  }
```

Add UI: a header "Select" toggle that sets `selectMode`, per-card checkboxes (pass `selectMode`/selection down to `AccountCard`, or render a checkbox overlay in `Main`), and an export bar shown when `selectMode` with three buttons (QR / Text / Google) calling `doExport(...)`, plus a visible plaintext-secret warning before/while exporting. Reuse existing tokens/classes (`.tool`, `vo-ghost`, `.hint`, `.err`). Keep it consistent with the existing header tools in `Main.svelte` (the `＋`/`🔒`/`⚙` buttons).

- [ ] **Step 2: Verify**

Run: `npx svelte-check --tsconfig ./tsconfig.json` → no new errors.
Run: `npm run build` → succeeds.
Run: `npm test` → existing suite green.
Manual (if a desktop session is available): `npm run tauri dev` — enter select mode, check 2 accounts, Export → Text → save dialog → the `.txt` holds exactly those otpauth URIs; Export → QR → directory dialog → one PNG per selected account; Export → Google → a single migration PNG that Google Authenticator imports.

- [ ] **Step 3: Commit** — SKIP under no-git.

```bash
git add src/routes/Main.svelte src/components/AccountCard.svelte
git commit -m "feat(ui): multi-select account export (qr/text/migration)"
```

---

## Self-Review

**Spec coverage (export portions of the spec §6):**
- Multi-select accounts — Task 6. ✓
- Three formats: otpauth QR PNG (per account), otpauth text, Google migration QR — Tasks 1-4. ✓
- All encoding + writing in Rust; secrets never reach the renderer (frontend sends only ids+path+format; QR written to file, not displayed) — Task 4 + Global Constraints. ✓
- otpauth builder / QR encoder / migration encoder — Tasks 1/2/3, each round-tripped against the existing parser/decoder. ✓
- One new dep (`qrcode`) with an audit entry — Task 2. ✓
- Unlock-gated; opaque failures — Task 4 (`snapshot()` errors when locked). ✓
- Plaintext-secret warning to the user — Task 6. ✓

**Placeholder scan:** none. Task 2 Step 1's "verify the qrcode API against the real crate and adapt" is an evidence-grounding instruction (the crate is external and must not be guessed), with the concrete fallback approach and the exact code given.

**Type/name consistency:** `build_otpauth`, `encode_png`, `build_migration`, `export_secrets`/`export_secrets_inner`, `sanitize_filename`, and format strings `"otpauth_qr"|"otpauth_text"|"google_migration"` are identical across Tasks 4-6 (Rust match arms ↔ TS `ExportFormat` ↔ command args `{ ids, path, format }`).

**Known carry-over (resolve at final review):** the Plan-1 `Vault::mode()` dead-code warning persists (no Plan-3 task consumes it). Single-batch migration QR has a capacity ceiling — `encode_png` errors (not panics) on oversized input (Task 2 test covers it); very large selections to `google_migration` surface that error to the user.
