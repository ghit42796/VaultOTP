# Offline TOTP Desktop App Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a fully offline, cross-platform (Windows/macOS/Linux) TOTP authenticator desktop app — a Google Authenticator replacement with a master-password-encrypted local vault.

**Architecture:** Tauri app. All secrets, key derivation, crypto, and TOTP computation live in the Rust backend; the Svelte/TypeScript frontend only renders codes and drives interactions over Tauri IPC. The vault is a single encrypted file using KeePass-style cryptography (composite key → Argon2id → AES-256-GCM with the header as AEAD associated data).

**Tech Stack:** Rust (Tauri 2), Svelte + TypeScript + Vite, Argon2id, AES-256-GCM, HMAC-SHA1/256/512, rqrr (QR decode), prost (protobuf), xcap (screen capture).

## Global Constraints

- Rust edition **2021**; crate versions: `tauri 2`, `argon2 0.5`, `aes-gcm 0.10`, `hmac 0.12`, `sha1 0.10`, `sha2 0.10`, `rqrr 0.8`, `image 0.25`, `prost 0.13`, `base32 0.5`, `serde 1`, `serde_json 1`, `zeroize 1`, `uuid 1` (features `v4`), `xcap 0.0`, `thiserror 1`, `rand 0.8`.
- Frontend: Svelte 4 + TypeScript + Vite; `@tauri-apps/api 2`, `@tauri-apps/plugin-dialog 2`; tests via `vitest`.
- **Secrets never cross IPC.** Frontend receives only `{issuer, label, code, remaining_seconds}` — never the Base32 secret.
- All crypto failures (wrong password, tampered file) return one opaque error: `"Incorrect password or corrupted vault"`.
- All vault writes are atomic: write to `vault.bin.tmp`, fsync, then rename over `vault.bin`.
- Sensitive byte buffers (master key, derived key, decrypted secret bytes) use `zeroize` on drop.
- TDD throughout: failing test → run (fail) → minimal impl → run (pass) → commit.
- TOTP defaults: algorithm SHA1, 6 digits, 30s period.

---

### Task 1: Scaffold Tauri + Svelte project

**Files:**
- Create: `package.json`, `vite.config.ts`, `svelte.config.js`, `tsconfig.json`, `index.html`, `src/main.ts`, `src/App.svelte`
- Create: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/build.rs`, `src-tauri/src/main.rs`
- Create: `.gitignore`

**Interfaces:**
- Produces: a buildable Tauri app shell with one `greet`-style command stubbed out and the dev server wired up. Later tasks replace `main.rs` internals.

- [ ] **Step 1: Create `.gitignore`**

```gitignore
node_modules/
dist/
src-tauri/target/
*.log
.DS_Store
```

- [ ] **Step 2: Create `package.json`**

```json
{
  "name": "auth-totp-app",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "tauri": "tauri",
    "test": "vitest run"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-dialog": "^2",
    "svelte": "^4"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^3",
    "@tauri-apps/cli": "^2",
    "@tsconfig/svelte": "^5",
    "svelte-check": "^3",
    "typescript": "^5",
    "vite": "^5",
    "vitest": "^2"
  }
}
```

- [ ] **Step 3: Create frontend config files**

`vite.config.ts`:
```ts
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
});
```

`svelte.config.js`:
```js
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
export default { preprocess: vitePreprocess() };
```

`tsconfig.json`:
```json
{
  "extends": "@tsconfig/svelte/tsconfig.json",
  "compilerOptions": {
    "target": "ESNext",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true
  },
  "include": ["src/**/*.ts", "src/**/*.svelte"]
}
```

`index.html`:
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Auth TOTP</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

`src/main.ts`:
```ts
import App from "./App.svelte";
const app = new App({ target: document.getElementById("app")! });
export default app;
```

`src/App.svelte`:
```svelte
<main><h1>Auth TOTP</h1></main>
```

- [ ] **Step 4: Create `src-tauri/Cargo.toml`**

```toml
[package]
name = "auth-totp-app"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
argon2 = "0.5"
aes-gcm = "0.10"
hmac = "0.12"
sha1 = "0.10"
sha2 = "0.10"
base32 = "0.5"
rqrr = "0.8"
image = "0.25"
prost = "0.13"
zeroize = { version = "1", features = ["derive"] }
uuid = { version = "1", features = ["v4"] }
xcap = "0.0"
rand = "0.8"

[features]
custom-protocol = ["tauri/custom-protocol"]
```

- [ ] **Step 5: Create `src-tauri/build.rs` and `tauri.conf.json`**

`src-tauri/build.rs`:
```rust
fn main() {
    tauri_build::build();
}
```

`src-tauri/tauri.conf.json`:
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Auth TOTP",
  "version": "0.1.0",
  "identifier": "com.authtotp.app",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [{ "title": "Auth TOTP", "width": 420, "height": 680, "resizable": true }],
    "security": { "csp": null }
  },
  "plugins": {},
  "bundle": { "active": true, "targets": "all" }
}
```

- [ ] **Step 6: Create minimal `src-tauri/src/main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn ping() -> String {
    "pong".to_string()
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![ping])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 7: Verify it builds**

Run: `cd src-tauri && cargo build`
Expected: compiles successfully (downloads crates). Frontend deps: `npm install`.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "chore: scaffold Tauri + Svelte project"
```

---

### Task 2: Error type and account model

**Files:**
- Create: `src-tauri/src/error.rs`
- Create: `src-tauri/src/model.rs`
- Modify: `src-tauri/src/main.rs` (add `mod error; mod model;`)

**Interfaces:**
- Produces: `enum AppError` with `Display` and `serde::Serialize`; `Result<T> = std::result::Result<T, AppError>`. `struct Account { id: String, issuer: String, label: String, secret: String, algorithm: Algorithm, digits: u32, period: u64, r#type: String }` and `enum Algorithm { Sha1, Sha256, Sha512 }`, both `Serialize`/`Deserialize`. The opaque crypto error is `AppError::Crypto`.

- [ ] **Step 1: Write the failing test**

`src-tauri/src/error.rs`:
```rust
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Incorrect password or corrupted vault")]
    Crypto,
    #[error("Invalid Base32 secret")]
    InvalidSecret,
    #[error("Could not decode QR code")]
    QrDecode,
    #[error("Invalid migration data")]
    Migration,
    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crypto_error_is_opaque() {
        assert_eq!(AppError::Crypto.to_string(), "Incorrect password or corrupted vault");
    }
    #[test]
    fn crypto_error_serializes_to_string() {
        let json = serde_json::to_string(&AppError::Crypto).unwrap();
        assert_eq!(json, "\"Incorrect password or corrupted vault\"");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test error::`
Expected: FAIL — `error` module not declared in `main.rs`.

- [ ] **Step 3: Wire module and add model**

In `src-tauri/src/main.rs` add after the `#![cfg_attr...]` line:
```rust
mod error;
mod model;
```

`src-tauri/src/model.rs`:
```rust
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Algorithm {
    Sha1,
    Sha256,
    Sha512,
}

impl Default for Algorithm {
    fn default() -> Self { Algorithm::Sha1 }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub issuer: String,
    pub label: String,
    pub secret: String,
    #[serde(default)]
    pub algorithm: Algorithm,
    pub digits: u32,
    pub period: u64,
    #[serde(rename = "type", default = "default_type")]
    pub kind: String,
}

fn default_type() -> String { "totp".to_string() }

impl Account {
    pub fn new(issuer: String, label: String, secret: String) -> Self {
        Account {
            id: uuid::Uuid::new_v4().to_string(),
            issuer,
            label,
            secret,
            algorithm: Algorithm::Sha1,
            digits: 6,
            period: 30,
            kind: "totp".to_string(),
        }
    }
}

impl Drop for Account {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_account_has_uuid_and_defaults() {
        let a = Account::new("GitHub".into(), "alice".into(), "JBSWY3DPEHPK3PXP".into());
        assert_eq!(a.digits, 6);
        assert_eq!(a.period, 30);
        assert_eq!(a.algorithm, Algorithm::Sha1);
        assert_eq!(a.kind, "totp");
        assert_eq!(a.id.len(), 36);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test error:: model::`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/error.rs src-tauri/src/model.rs src-tauri/src/main.rs
git commit -m "feat: add AppError and Account model"
```

---

### Task 3: Base32 secret validation/decoding

**Files:**
- Create: `src-tauri/src/secret.rs`
- Modify: `src-tauri/src/main.rs` (add `mod secret;`)

**Interfaces:**
- Consumes: `error::{AppError, Result}`.
- Produces: `pub fn decode_secret(s: &str) -> Result<Vec<u8>>` — uppercases, strips spaces and `=` padding, decodes RFC 4648 Base32; returns `AppError::InvalidSecret` on bad input.

- [ ] **Step 1: Write the failing test**

`src-tauri/src/secret.rs`:
```rust
use crate::error::{AppError, Result};

/// Decode an RFC 4648 Base32 secret, tolerating spaces, lowercase, and padding.
pub fn decode_secret(s: &str) -> Result<Vec<u8>> {
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect::<String>()
        .trim_end_matches('=')
        .to_uppercase();
    if cleaned.is_empty() {
        return Err(AppError::InvalidSecret);
    }
    base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &cleaned)
        .ok_or(AppError::InvalidSecret)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_known_secret() {
        // "Hello!\xDE\xAD\xBE\xEF" classic test vector
        let bytes = decode_secret("JBSWY3DPEHPK3PXP").unwrap();
        assert_eq!(bytes, vec![0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x21, 0xde, 0xad, 0xbe, 0xef]);
    }
    #[test]
    fn tolerates_spaces_and_lowercase() {
        let a = decode_secret("jbsw y3dp ehpk 3pxp").unwrap();
        let b = decode_secret("JBSWY3DPEHPK3PXP").unwrap();
        assert_eq!(a, b);
    }
    #[test]
    fn rejects_invalid() {
        assert!(matches!(decode_secret("not base32!!"), Err(AppError::InvalidSecret)));
        assert!(matches!(decode_secret("   "), Err(AppError::InvalidSecret)));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test secret::`
Expected: FAIL — `secret` module not declared.

- [ ] **Step 3: Wire the module**

In `src-tauri/src/main.rs` add `mod secret;`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test secret::`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/secret.rs src-tauri/src/main.rs
git commit -m "feat: add Base32 secret decoding"
```

---

### Task 4: TOTP code generation (RFC 6238)

**Files:**
- Create: `src-tauri/src/totp.rs`
- Modify: `src-tauri/src/main.rs` (add `mod totp;`)

**Interfaces:**
- Consumes: `model::{Account, Algorithm}`, `secret::decode_secret`, `error::Result`.
- Produces: `pub fn generate(account: &Account, unix_time: u64) -> Result<String>` (zero-padded code string), and `pub fn remaining_seconds(account: &Account, unix_time: u64) -> u64`.

- [ ] **Step 1: Write the failing test (RFC 6238 vectors)**

`src-tauri/src/totp.rs`:
```rust
use crate::error::Result;
use crate::model::{Account, Algorithm};
use crate::secret::decode_secret;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::{Sha256, Sha512};

fn hmac_digest(algo: Algorithm, key: &[u8], counter: u64) -> Vec<u8> {
    let msg = counter.to_be_bytes();
    match algo {
        Algorithm::Sha1 => {
            let mut m = <Hmac<Sha1>>::new_from_slice(key).unwrap();
            m.update(&msg);
            m.finalize().into_bytes().to_vec()
        }
        Algorithm::Sha256 => {
            let mut m = <Hmac<Sha256>>::new_from_slice(key).unwrap();
            m.update(&msg);
            m.finalize().into_bytes().to_vec()
        }
        Algorithm::Sha512 => {
            let mut m = <Hmac<Sha512>>::new_from_slice(key).unwrap();
            m.update(&msg);
            m.finalize().into_bytes().to_vec()
        }
    }
}

/// RFC 6238 TOTP. `unix_time` is seconds since epoch.
pub fn generate(account: &Account, unix_time: u64) -> Result<String> {
    let key = decode_secret(&account.secret)?;
    let counter = unix_time / account.period;
    let digest = hmac_digest(account.algorithm, &key, counter);
    let offset = (digest[digest.len() - 1] & 0x0f) as usize;
    let bin = ((digest[offset] as u32 & 0x7f) << 24)
        | ((digest[offset + 1] as u32) << 16)
        | ((digest[offset + 2] as u32) << 8)
        | (digest[offset + 3] as u32);
    let modulo = 10u32.pow(account.digits);
    let code = bin % modulo;
    Ok(format!("{:0width$}", code, width = account.digits as usize))
}

pub fn remaining_seconds(account: &Account, unix_time: u64) -> u64 {
    account.period - (unix_time % account.period)
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 6238 Appendix B uses ASCII seed "12345678901234567890" => Base32 below.
    fn vec_account(algo: Algorithm, b32: &str) -> Account {
        let mut a = Account::new("t".into(), "t".into(), b32.into());
        a.algorithm = algo;
        a.digits = 8;
        a.period = 30;
        a
    }

    #[test]
    fn rfc6238_sha1_vectors() {
        let a = vec_account(Algorithm::Sha1, "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
        assert_eq!(generate(&a, 59).unwrap(), "94287082");
        assert_eq!(generate(&a, 1111111109).unwrap(), "07081804");
        assert_eq!(generate(&a, 2000000000).unwrap(), "69279037");
    }

    #[test]
    fn rfc6238_sha256_vector() {
        // 32-byte seed "12345678901234567890123456789012"
        let a = vec_account(Algorithm::Sha256, "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZA");
        assert_eq!(generate(&a, 59).unwrap(), "46119246");
    }

    #[test]
    fn default_six_digits() {
        let mut a = Account::new("t".into(), "t".into(), "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".into());
        a.digits = 6;
        let code = generate(&a, 59).unwrap();
        assert_eq!(code.len(), 6);
        assert_eq!(code, "287082");
    }

    #[test]
    fn remaining_counts_down() {
        let a = Account::new("t".into(), "t".into(), "GEZDGNBVGY3TQOJQ".into());
        assert_eq!(remaining_seconds(&a, 0), 30);
        assert_eq!(remaining_seconds(&a, 29), 1);
        assert_eq!(remaining_seconds(&a, 30), 30);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test totp::`
Expected: FAIL — `totp` module not declared.

- [ ] **Step 3: Wire the module**

In `src-tauri/src/main.rs` add `mod totp;`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test totp::`
Expected: PASS (4 tests). The SHA-256 seed Base32 above encodes the 32-byte ASCII seed; if the vector mismatches, recompute the Base32 of `"12345678901234567890123456789012"` and update the literal — the assertion value `46119246` is fixed by RFC 6238.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/totp.rs src-tauri/src/main.rs
git commit -m "feat: add RFC 6238 TOTP generation"
```

---

### Task 5: Key derivation (composite key → Argon2id)

**Files:**
- Create: `src-tauri/src/vault/mod.rs` (module declaration only for now)
- Create: `src-tauri/src/vault/kdf.rs`
- Modify: `src-tauri/src/main.rs` (add `mod vault;`)

**Interfaces:**
- Consumes: `error::{AppError, Result}`.
- Produces: `pub struct KdfParams { pub mem_kib: u32, pub iterations: u32, pub parallelism: u32 }` with `Default` (64 MiB / 3 / 4), `Serialize`/`Deserialize`. `pub fn derive_key(password: &[u8], salt: &[u8; 16], params: &KdfParams) -> Result<[u8; 32]>`.

- [ ] **Step 1: Write the failing test**

`src-tauri/src/vault/kdf.rs`:
```rust
use crate::error::{AppError, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct KdfParams {
    pub mem_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        KdfParams { mem_kib: 65536, iterations: 3, parallelism: 4 }
    }
}

/// Composite key (KeePass-style): SHA-256(password) then Argon2id.
pub fn derive_key(password: &[u8], salt: &[u8; 16], params: &KdfParams) -> Result<[u8; 32]> {
    use sha2::{Digest, Sha256};
    let composite = Sha256::digest(password);

    let p = Params::new(params.mem_kib, params.iterations, params.parallelism, Some(32))
        .map_err(|_| AppError::Crypto)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, p);
    let mut out = [0u8; 32];
    argon
        .hash_password_into(&composite, salt, &mut out)
        .map_err(|_| AppError::Crypto)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Use small params in tests so they run fast.
    fn fast() -> KdfParams { KdfParams { mem_kib: 8192, iterations: 1, parallelism: 1 } }

    #[test]
    fn same_input_same_key() {
        let salt = [7u8; 16];
        let a = derive_key(b"hunter2", &salt, &fast()).unwrap();
        let b = derive_key(b"hunter2", &salt, &fast()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn different_password_different_key() {
        let salt = [7u8; 16];
        let a = derive_key(b"hunter2", &salt, &fast()).unwrap();
        let b = derive_key(b"hunter3", &salt, &fast()).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn different_salt_different_key() {
        let a = derive_key(b"pw", &[1u8; 16], &fast()).unwrap();
        let b = derive_key(b"pw", &[2u8; 16], &fast()).unwrap();
        assert_ne!(a, b);
    }
}
```

`src-tauri/src/vault/mod.rs`:
```rust
pub mod kdf;
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test vault::kdf::`
Expected: FAIL — `vault` module not declared.

- [ ] **Step 3: Wire the module**

In `src-tauri/src/main.rs` add `mod vault;`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test vault::kdf::`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/vault src-tauri/src/main.rs
git commit -m "feat: add Argon2id key derivation"
```

---

### Task 6: Vault crypto (AES-256-GCM with header as AAD)

**Files:**
- Create: `src-tauri/src/vault/crypto.rs`
- Modify: `src-tauri/src/vault/mod.rs` (add `pub mod crypto;`)

**Interfaces:**
- Consumes: `error::{AppError, Result}`, `vault::kdf::{KdfParams, derive_key}`.
- Produces:
  - `pub struct VaultHeader { pub version: u16, pub kdf: KdfParams, pub salt: [u8;16], pub nonce: [u8;12] }` (`Serialize`/`Deserialize`), with `pub const MAGIC: &[u8] = b"ATOTP1\0"`.
  - `pub fn serialize_header(h: &VaultHeader) -> Vec<u8>` and `pub fn parse_header(bytes: &[u8]) -> Result<(VaultHeader, usize)>` (returns header + offset where ciphertext begins).
  - `pub fn encrypt(key: &[u8;32], header: &VaultHeader, plaintext: &[u8]) -> Result<Vec<u8>>` — returns full file bytes (magic + header-json-len + header-json + ciphertext+tag), header bytes used as AAD.
  - `pub fn decrypt(password: &[u8], file_bytes: &[u8]) -> Result<Vec<u8>>` — parses header, derives key, AES-GCM-decrypts, returns plaintext; any failure → `AppError::Crypto`.

- [ ] **Step 1: Write the failing test**

`src-tauri/src/vault/crypto.rs`:
```rust
use crate::error::{AppError, Result};
use crate::vault::kdf::{derive_key, KdfParams};
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use serde::{Deserialize, Serialize};

pub const MAGIC: &[u8] = b"ATOTP1\0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultHeader {
    pub version: u16,
    pub kdf: KdfParams,
    pub salt: [u8; 16],
    pub nonce: [u8; 12],
}

/// File layout: MAGIC (7) | header_len: u32 LE (4) | header_json | ciphertext+tag
/// The bytes `MAGIC | header_len | header_json` are the AEAD associated data.
fn aad(header_json: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(MAGIC.len() + 4 + header_json.len());
    v.extend_from_slice(MAGIC);
    v.extend_from_slice(&(header_json.len() as u32).to_le_bytes());
    v.extend_from_slice(header_json);
    v
}

pub fn encrypt(key: &[u8; 32], header: &VaultHeader, plaintext: &[u8]) -> Result<Vec<u8>> {
    let header_json = serde_json::to_vec(header).map_err(|_| AppError::Crypto)?;
    let associated = aad(&header_json);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(&header.nonce);
    let ct = cipher
        .encrypt(nonce, Payload { msg: plaintext, aad: &associated })
        .map_err(|_| AppError::Crypto)?;
    let mut out = associated;
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn decrypt(password: &[u8], file_bytes: &[u8]) -> Result<Vec<u8>> {
    if file_bytes.len() < MAGIC.len() + 4 || &file_bytes[..MAGIC.len()] != MAGIC {
        return Err(AppError::Crypto);
    }
    let len_start = MAGIC.len();
    let header_len = u32::from_le_bytes(
        file_bytes[len_start..len_start + 4].try_into().map_err(|_| AppError::Crypto)?,
    ) as usize;
    let header_start = len_start + 4;
    let header_end = header_start + header_len;
    if file_bytes.len() < header_end {
        return Err(AppError::Crypto);
    }
    let header_json = &file_bytes[header_start..header_end];
    let header: VaultHeader = serde_json::from_slice(header_json).map_err(|_| AppError::Crypto)?;

    let key = derive_key(password, &header.salt, &header.kdf)?;
    let associated = aad(header_json);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let nonce = Nonce::from_slice(&header.nonce);
    cipher
        .decrypt(nonce, Payload { msg: &file_bytes[header_end..], aad: &associated })
        .map_err(|_| AppError::Crypto)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fast_header() -> (VaultHeader, [u8; 32]) {
        let kdf = KdfParams { mem_kib: 8192, iterations: 1, parallelism: 1 };
        let salt = [9u8; 16];
        let nonce = [3u8; 12];
        let key = derive_key(b"masterpw", &salt, &kdf).unwrap();
        (VaultHeader { version: 1, kdf, salt, nonce }, key)
    }

    #[test]
    fn round_trip() {
        let (h, key) = fast_header();
        let file = encrypt(&key, &h, b"secret data").unwrap();
        let pt = decrypt(b"masterpw", &file).unwrap();
        assert_eq!(pt, b"secret data");
    }

    #[test]
    fn wrong_password_fails() {
        let (h, key) = fast_header();
        let file = encrypt(&key, &h, b"secret data").unwrap();
        assert!(matches!(decrypt(b"wrongpw", &file), Err(AppError::Crypto)));
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let (h, key) = fast_header();
        let mut file = encrypt(&key, &h, b"secret data").unwrap();
        let last = file.len() - 1;
        file[last] ^= 0x01;
        assert!(matches!(decrypt(b"masterpw", &file), Err(AppError::Crypto)));
    }

    #[test]
    fn tampered_header_fails() {
        let (h, key) = fast_header();
        let mut file = encrypt(&key, &h, b"secret data").unwrap();
        // flip a byte inside the header json region
        file[MAGIC.len() + 4 + 2] ^= 0x01;
        assert!(matches!(decrypt(b"masterpw", &file), Err(AppError::Crypto)));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test vault::crypto::`
Expected: FAIL — `crypto` not in `vault/mod.rs`.

- [ ] **Step 3: Wire the module**

In `src-tauri/src/vault/mod.rs` add `pub mod crypto;`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test vault::crypto::`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/vault
git commit -m "feat: add AES-256-GCM vault crypto with AAD-bound header"
```

---

### Task 7: Atomic storage

**Files:**
- Create: `src-tauri/src/storage.rs`
- Modify: `src-tauri/src/main.rs` (add `mod storage;`)

**Interfaces:**
- Consumes: `error::{AppError, Result}`.
- Produces: `pub fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> Result<()>` (writes `*.tmp`, fsync, rename), `pub fn read_file(path: &std::path::Path) -> Result<Vec<u8>>`, `pub fn exists(path: &std::path::Path) -> bool`.

- [ ] **Step 1: Write the failing test**

`src-tauri/src/storage.rs`:
```rust
use crate::error::{AppError, Result};
use std::io::Write;
use std::path::Path;

pub fn exists(path: &Path) -> bool {
    path.exists()
}

pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|e| AppError::Other(format!("read failed: {e}")))
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = std::fs::File::create(&tmp)
            .map_err(|e| AppError::Other(format!("create tmp failed: {e}")))?;
        f.write_all(bytes).map_err(|e| AppError::Other(format!("write failed: {e}")))?;
        f.sync_all().map_err(|e| AppError::Other(format!("fsync failed: {e}")))?;
    }
    std::fs::rename(&tmp, path).map_err(|e| AppError::Other(format!("rename failed: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_then_read_round_trips() {
        let dir = std::env::temp_dir().join(format!("atotp_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("vault.bin");
        assert!(!exists(&path));
        write_atomic(&path, b"hello").unwrap();
        assert!(exists(&path));
        assert_eq!(read_file(&path).unwrap(), b"hello");
        // overwrite
        write_atomic(&path, b"world!!").unwrap();
        assert_eq!(read_file(&path).unwrap(), b"world!!");
        // no leftover tmp
        assert!(!dir.join("vault.tmp").exists());
        std::fs::remove_dir_all(&dir).ok();
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test storage::`
Expected: FAIL — `storage` module not declared.

- [ ] **Step 3: Wire the module**

In `src-tauri/src/main.rs` add `mod storage;`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test storage::`
Expected: PASS (1 test).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/storage.rs src-tauri/src/main.rs
git commit -m "feat: add atomic vault file storage"
```

---

### Task 8: Vault state machine (lock/unlock/CRUD)

**Files:**
- Modify: `src-tauri/src/vault/mod.rs` (add `Vault` struct)

**Interfaces:**
- Consumes: `vault::crypto::{VaultHeader, encrypt, decrypt, MAGIC}`, `vault::kdf::{KdfParams, derive_key}`, `storage`, `model::Account`, `error::{AppError, Result}`, `rand`.
- Produces:
  - `pub struct Vault` holding an `Option<Unlocked>` (None = locked). `Unlocked` holds `key: [u8;32]` (zeroized on drop), `accounts: Vec<Account>`, `kdf: KdfParams`, `salt: [u8;16]`.
  - `Vault::new() -> Vault` (locked).
  - `pub fn is_unlocked(&self) -> bool`.
  - `pub fn create(&mut self, path, password: &[u8]) -> Result<()>` — fresh salt, empty accounts, persists.
  - `pub fn unlock(&mut self, path, password: &[u8]) -> Result<()>` — reads file, decrypts, loads accounts.
  - `pub fn lock(&mut self)` — drops `Unlocked` (zeroizes key).
  - `pub fn accounts(&self) -> Result<&[Account]>` (Err if locked).
  - `pub fn add(&mut self, path, account: Account) -> Result<()>`, `pub fn remove(&mut self, path, id: &str) -> Result<()>` — mutate then persist.
  - private `fn persist(&self, path) -> Result<()>` — new random nonce each save.

- [ ] **Step 1: Write the failing test**

Append to `src-tauri/src/vault/mod.rs`:
```rust
pub mod kdf;
pub mod crypto;

use crate::error::{AppError, Result};
use crate::model::Account;
use crate::storage;
use crypto::{decrypt, encrypt, VaultHeader};
use kdf::{derive_key, KdfParams};
use rand::RngCore;
use std::path::Path;
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

    pub fn create(&mut self, path: &Path, password: &[u8]) -> Result<()> {
        let kdf = KdfParams::default();
        let mut salt = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut salt);
        let key = derive_key(password, &salt, &kdf)?;
        self.state = Some(Unlocked { key, accounts: Vec::new(), kdf, salt });
        self.persist(path)
    }

    pub fn unlock(&mut self, path: &Path, password: &[u8]) -> Result<()> {
        let file = storage::read_file(path)?;
        let plaintext = decrypt(password, &file)?;
        let accounts: Vec<Account> =
            serde_json::from_slice(&plaintext).map_err(|_| AppError::Crypto)?;
        // re-parse header to recover salt/kdf for future saves
        let header = parse_header_only(&file)?;
        let key = derive_key(password, &header.salt, &header.kdf)?;
        self.state = Some(Unlocked { key, accounts, kdf: header.kdf, salt: header.salt });
        Ok(())
    }

    pub fn lock(&mut self) {
        self.state = None;
    }

    pub fn accounts(&self) -> Result<&[Account]> {
        self.state.as_ref().map(|u| u.accounts.as_slice()).ok_or(AppError::Crypto)
    }

    pub fn add(&mut self, path: &Path, account: Account) -> Result<()> {
        let u = self.state.as_mut().ok_or(AppError::Crypto)?;
        u.accounts.push(account);
        self.persist(path)
    }

    pub fn remove(&mut self, path: &Path, id: &str) -> Result<()> {
        let u = self.state.as_mut().ok_or(AppError::Crypto)?;
        u.accounts.retain(|a| a.id != id);
        self.persist(path)
    }

    fn persist(&self, path: &Path) -> Result<()> {
        let u = self.state.as_ref().ok_or(AppError::Crypto)?;
        let mut nonce = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce);
        let header = VaultHeader { version: 1, kdf: u.kdf, salt: u.salt, nonce };
        let plaintext = serde_json::to_vec(&u.accounts).map_err(|_| AppError::Crypto)?;
        let file = encrypt(&u.key, &header, &plaintext)?;
        storage::write_atomic(path, &file)
    }
}

fn parse_header_only(file: &[u8]) -> Result<VaultHeader> {
    let magic_len = crypto::MAGIC.len();
    if file.len() < magic_len + 4 {
        return Err(AppError::Crypto);
    }
    let header_len =
        u32::from_le_bytes(file[magic_len..magic_len + 4].try_into().map_err(|_| AppError::Crypto)?)
            as usize;
    let start = magic_len + 4;
    serde_json::from_slice(&file[start..start + header_len]).map_err(|_| AppError::Crypto)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("atotp_vault_{}.bin", uuid::Uuid::new_v4()))
    }

    #[test]
    fn create_unlock_cycle() {
        let path = tmp_path();
        let mut v = Vault::new();
        assert!(!v.is_unlocked());
        v.create(&path, b"masterpw").unwrap();
        assert!(v.is_unlocked());
        v.add(&path, Account::new("GitHub".into(), "alice".into(), "JBSWY3DPEHPK3PXP".into()))
            .unwrap();
        assert_eq!(v.accounts().unwrap().len(), 1);

        // lock, then re-open from disk
        v.lock();
        assert!(!v.is_unlocked());
        assert!(v.accounts().is_err());

        v.unlock(&path, b"masterpw").unwrap();
        assert_eq!(v.accounts().unwrap().len(), 1);
        assert_eq!(v.accounts().unwrap()[0].issuer, "GitHub");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn wrong_password_unlock_fails() {
        let path = tmp_path();
        let mut v = Vault::new();
        v.create(&path, b"masterpw").unwrap();
        v.lock();
        assert!(matches!(v.unlock(&path, b"nope"), Err(AppError::Crypto)));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn remove_persists() {
        let path = tmp_path();
        let mut v = Vault::new();
        v.create(&path, b"pw").unwrap();
        let acc = Account::new("a".into(), "b".into(), "JBSWY3DPEHPK3PXP".into());
        let id = acc.id.clone();
        v.add(&path, acc).unwrap();
        v.remove(&path, &id).unwrap();
        v.lock();
        v.unlock(&path, b"pw").unwrap();
        assert_eq!(v.accounts().unwrap().len(), 0);
        std::fs::remove_file(&path).ok();
    }
}
```

Replace the entire current contents of `vault/mod.rs` with the above (it re-declares the `pub mod` lines).

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test vault::tests`
Expected: FAIL — `Vault` not yet present (before paste) or compile error on first run.

- [ ] **Step 3: Confirm implementation present**

The code above is the implementation; ensure it compiles.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test vault::`
Expected: PASS (kdf + crypto + vault tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/vault/mod.rs
git commit -m "feat: add Vault lock/unlock state machine with CRUD"
```

---

### Task 9: otpauth:// URI parser

**Files:**
- Create: `src-tauri/src/otpauth.rs`
- Modify: `src-tauri/src/main.rs` (add `mod otpauth;`)

**Interfaces:**
- Consumes: `model::{Account, Algorithm}`, `error::{AppError, Result}`.
- Produces: `pub fn parse_otpauth(uri: &str) -> Result<Account>` — parses `otpauth://totp/Issuer:label?secret=...&issuer=...&algorithm=...&digits=...&period=...`; URL-decodes label/issuer; rejects non-totp.

- [ ] **Step 1: Write the failing test**

`src-tauri/src/otpauth.rs`:
```rust
use crate::error::{AppError, Result};
use crate::model::{Account, Algorithm};

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                if let (Some(h), Some(l)) = (hi, lo) {
                    out.push((h * 16 + l) as u8);
                    i += 3;
                    continue;
                }
                out.push(bytes[i]);
                i += 1;
            }
            b'+' => { out.push(b' '); i += 1; }
            b => { out.push(b); i += 1; }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn parse_otpauth(uri: &str) -> Result<Account> {
    let rest = uri.strip_prefix("otpauth://totp/").ok_or(AppError::InvalidSecret)?;
    let (path, query) = match rest.split_once('?') {
        Some((p, q)) => (p, q),
        None => (rest, ""),
    };

    let label_part = url_decode(path);
    let (issuer_from_label, label) = match label_part.split_once(':') {
        Some((i, l)) => (i.trim().to_string(), l.trim().to_string()),
        None => (String::new(), label_part.trim().to_string()),
    };

    let mut secret = String::new();
    let mut issuer = issuer_from_label;
    let mut algorithm = Algorithm::Sha1;
    let mut digits = 6u32;
    let mut period = 30u64;

    for pair in query.split('&').filter(|s| !s.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let v = url_decode(v);
        match k {
            "secret" => secret = v,
            "issuer" => issuer = v,
            "algorithm" => {
                algorithm = match v.to_uppercase().as_str() {
                    "SHA256" => Algorithm::Sha256,
                    "SHA512" => Algorithm::Sha512,
                    _ => Algorithm::Sha1,
                }
            }
            "digits" => digits = v.parse().unwrap_or(6),
            "period" => period = v.parse().unwrap_or(30),
            _ => {}
        }
    }

    if secret.is_empty() {
        return Err(AppError::InvalidSecret);
    }
    crate::secret::decode_secret(&secret)?; // validate

    Ok(Account {
        id: uuid::Uuid::new_v4().to_string(),
        issuer,
        label,
        secret,
        algorithm,
        digits,
        period,
        kind: "totp".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_uri() {
        let uri = "otpauth://totp/GitHub:alice%40example.com?secret=JBSWY3DPEHPK3PXP&issuer=GitHub&algorithm=SHA256&digits=8&period=60";
        let a = parse_otpauth(uri).unwrap();
        assert_eq!(a.issuer, "GitHub");
        assert_eq!(a.label, "alice@example.com");
        assert_eq!(a.secret, "JBSWY3DPEHPK3PXP");
        assert_eq!(a.algorithm, Algorithm::Sha256);
        assert_eq!(a.digits, 8);
        assert_eq!(a.period, 60);
    }

    #[test]
    fn parses_minimal_uri_with_defaults() {
        let a = parse_otpauth("otpauth://totp/alice?secret=JBSWY3DPEHPK3PXP").unwrap();
        assert_eq!(a.label, "alice");
        assert_eq!(a.digits, 6);
        assert_eq!(a.period, 30);
        assert_eq!(a.algorithm, Algorithm::Sha1);
    }

    #[test]
    fn rejects_missing_secret() {
        assert!(parse_otpauth("otpauth://totp/alice").is_err());
    }

    #[test]
    fn rejects_non_totp() {
        assert!(parse_otpauth("otpauth://hotp/alice?secret=JBSWY3DPEHPK3PXP&counter=0").is_err());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test otpauth::`
Expected: FAIL — module not declared.

- [ ] **Step 3: Wire the module**

In `src-tauri/src/main.rs` add `mod otpauth;`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test otpauth::`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/otpauth.rs src-tauri/src/main.rs
git commit -m "feat: add otpauth URI parser"
```

---

### Task 10: QR decoding (image file + screen capture)

**Files:**
- Create: `src-tauri/src/qr.rs`
- Modify: `src-tauri/src/main.rs` (add `mod qr;`)

**Interfaces:**
- Consumes: `error::{AppError, Result}`, `image`, `rqrr`, `xcap`.
- Produces:
  - `pub fn decode_image_bytes(bytes: &[u8]) -> Result<Vec<String>>` — decode all QR strings in an image buffer.
  - `pub fn decode_image_file(path: &str) -> Result<Vec<String>>`.
  - `pub fn capture_region(x: i32, y: i32, w: u32, h: u32) -> Result<Vec<String>>` — grab a screen rectangle and decode (best-effort across platforms via `xcap`).

- [ ] **Step 1: Write the failing test**

Add a tiny generated-QR test using a pre-encoded PNG fixture. Since we avoid adding a QR *encoder* dependency, the test builds a luma image by hand is impractical; instead test the decode path against a committed fixture.

`src-tauri/src/qr.rs`:
```rust
use crate::error::{AppError, Result};

pub fn decode_image_bytes(bytes: &[u8]) -> Result<Vec<String>> {
    let img = image::load_from_memory(bytes).map_err(|_| AppError::QrDecode)?;
    let luma = img.to_luma8();
    let mut prepared = rqrr::PreparedImage::prepare(luma);
    let grids = prepared.detect_grids();
    let mut out = Vec::new();
    for g in grids {
        if let Ok((_meta, content)) = g.decode() {
            out.push(content);
        }
    }
    if out.is_empty() {
        return Err(AppError::QrDecode);
    }
    Ok(out)
}

pub fn decode_image_file(path: &str) -> Result<Vec<String>> {
    let bytes = std::fs::read(path).map_err(|_| AppError::QrDecode)?;
    decode_image_bytes(&bytes)
}

pub fn capture_region(x: i32, y: i32, w: u32, h: u32) -> Result<Vec<String>> {
    use xcap::Monitor;
    let monitors = Monitor::all().map_err(|_| AppError::QrDecode)?;
    let monitor = monitors.into_iter().next().ok_or(AppError::QrDecode)?;
    let full = monitor.capture_image().map_err(|_| AppError::QrDecode)?;
    // Crop region (clamp to bounds).
    let cropped = image::imageops::crop_imm(&full, x.max(0) as u32, y.max(0) as u32, w, h).to_image();
    let dynimg = image::DynamicImage::ImageRgba8(cropped);
    let mut buf = std::io::Cursor::new(Vec::new());
    dynimg.write_to(&mut buf, image::ImageFormat::Png).map_err(|_| AppError::QrDecode)?;
    decode_image_bytes(buf.get_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_fixture_qr() {
        // Fixture: tests/fixtures/qr_otpauth.png encodes
        // otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example
        let bytes = include_bytes!("../tests/fixtures/qr_otpauth.png");
        let results = decode_image_bytes(bytes).unwrap();
        assert!(results.iter().any(|s| s.starts_with("otpauth://totp/")));
    }

    #[test]
    fn non_image_bytes_error() {
        assert!(matches!(decode_image_bytes(b"not an image"), Err(AppError::QrDecode)));
    }
}
```

- [ ] **Step 2: Create the test fixture**

Generate `src-tauri/tests/fixtures/qr_otpauth.png` once using any QR generator (e.g. an online generator or `qrencode -o qr_otpauth.png 'otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example'`). Commit the PNG as a binary fixture. If `qrencode` is available:

```bash
mkdir -p src-tauri/tests/fixtures
qrencode -o src-tauri/tests/fixtures/qr_otpauth.png 'otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example'
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd src-tauri && cargo test qr::`
Expected: FAIL — module not declared (and/or fixture missing → create it in Step 2).

- [ ] **Step 4: Wire the module**

In `src-tauri/src/main.rs` add `mod qr;`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test qr::`
Expected: PASS (2 tests). `capture_region` is not unit-tested (needs a display); it is verified manually later.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/qr.rs src-tauri/src/main.rs src-tauri/tests/fixtures/qr_otpauth.png
git commit -m "feat: add QR decoding from image and screen region"
```

---

### Task 11: Google Authenticator migration (protobuf)

**Files:**
- Create: `src-tauri/build.rs` additions OR hand-written decoder `src-tauri/src/migration.rs`
- Modify: `src-tauri/src/main.rs` (add `mod migration;`)

**Interfaces:**
- Consumes: `model::{Account, Algorithm}`, `error::{AppError, Result}`, `base32`, `prost`.
- Produces: `pub fn parse_migration(uri: &str) -> Result<Vec<Account>>` — parses `otpauth-migration://offline?data=<base64>`, base64-decodes, prost-decodes the `MigrationPayload`, maps each `OtpParameters` (TOTP only) to an `Account` (re-encoding the raw secret bytes back to Base32).

- [ ] **Step 1: Define the protobuf schema inline via prost**

Google Authenticator's export schema (well-known):
```
message MigrationPayload {
  repeated OtpParameters otp_parameters = 1;
  int32 version = 2;
  int32 batch_size = 3;
  int32 batch_index = 4;
  int32 batch_id = 5;
}
message OtpParameters {
  bytes secret = 1;
  string name = 2;
  string issuer = 3;
  Algorithm algorithm = 4; // 1=SHA1 2=SHA256 3=SHA512
  DigitCount digits = 5;    // 1=SIX 2=EIGHT
  OtpType type = 6;         // 1=HOTP 2=TOTP
  int64 counter = 7;
}
```

Use prost's derive directly (no `.proto` file / build step needed) in `src-tauri/src/migration.rs`:
```rust
use crate::error::{AppError, Result};
use crate::model::{Account, Algorithm};
use prost::Message;

#[derive(Clone, PartialEq, Message)]
pub struct OtpParameters {
    #[prost(bytes = "vec", tag = "1")]
    pub secret: Vec<u8>,
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(string, tag = "3")]
    pub issuer: String,
    #[prost(int32, tag = "4")]
    pub algorithm: i32, // 1 SHA1, 2 SHA256, 3 SHA512
    #[prost(int32, tag = "5")]
    pub digits: i32, // 1 SIX, 2 EIGHT
    #[prost(int32, tag = "6")]
    pub r#type: i32, // 1 HOTP, 2 TOTP
    #[prost(int64, tag = "7")]
    pub counter: i64,
}

#[derive(Clone, PartialEq, Message)]
pub struct MigrationPayload {
    #[prost(message, repeated, tag = "1")]
    pub otp_parameters: Vec<OtpParameters>,
    #[prost(int32, tag = "2")]
    pub version: i32,
    #[prost(int32, tag = "3")]
    pub batch_size: i32,
    #[prost(int32, tag = "4")]
    pub batch_index: i32,
    #[prost(int32, tag = "5")]
    pub batch_id: i32,
}

fn url_query_value(uri: &str, key: &str) -> Option<String> {
    let q = uri.split_once('?')?.1;
    for pair in q.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return Some(crate::otpauth_decode(v));
            }
        }
    }
    None
}

pub fn parse_migration(uri: &str) -> Result<Vec<Account>> {
    if !uri.starts_with("otpauth-migration://") {
        return Err(AppError::Migration);
    }
    let data_b64 = url_query_value(uri, "data").ok_or(AppError::Migration)?;
    let raw = base64_decode(&data_b64).ok_or(AppError::Migration)?;
    let payload = MigrationPayload::decode(&raw[..]).map_err(|_| AppError::Migration)?;

    let mut accounts = Vec::new();
    for p in payload.otp_parameters {
        if p.r#type != 2 {
            continue; // TOTP only
        }
        let secret_b32 =
            base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &p.secret);
        let algorithm = match p.algorithm {
            2 => Algorithm::Sha256,
            3 => Algorithm::Sha512,
            _ => Algorithm::Sha1,
        };
        let digits = if p.digits == 2 { 8 } else { 6 };
        accounts.push(Account {
            id: uuid::Uuid::new_v4().to_string(),
            issuer: p.issuer,
            label: p.name,
            secret: secret_b32,
            algorithm,
            digits,
            period: 30,
            kind: "totp".to_string(),
        });
    }
    if accounts.is_empty() {
        return Err(AppError::Migration);
    }
    Ok(accounts)
}

/// Minimal standard Base64 decoder (GA uses standard alphabet, may be URL-encoded first).
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut lut = [255u8; 256];
    for (i, &c) in T.iter().enumerate() {
        lut[c as usize] = i as u8;
    }
    let clean: Vec<u8> = s.bytes().filter(|&b| b != b'=' && !b.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in clean.chunks(4) {
        let mut buf = [0u8; 4];
        let mut n = 0;
        for (i, &c) in chunk.iter().enumerate() {
            let v = lut[c as usize];
            if v == 255 { return None; }
            buf[i] = v;
            n += 1;
        }
        if n >= 2 { out.push((buf[0] << 2) | (buf[1] >> 4)); }
        if n >= 3 { out.push((buf[1] << 4) | (buf[2] >> 2)); }
        if n >= 4 { out.push((buf[2] << 6) | buf[3]); }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_totp_entry() {
        // Build a payload, encode it, wrap in a migration URI, then parse back.
        let secret = vec![0x48u8, 0x65, 0x6c, 0x6c, 0x6f]; // arbitrary bytes
        let payload = MigrationPayload {
            otp_parameters: vec![OtpParameters {
                secret: secret.clone(),
                name: "alice".into(),
                issuer: "Example".into(),
                algorithm: 1,
                digits: 1,
                r#type: 2,
                counter: 0,
            }],
            version: 1, batch_size: 1, batch_index: 0, batch_id: 0,
        };
        let mut raw = Vec::new();
        payload.encode(&mut raw).unwrap();
        let b64 = base64_encode(&raw);
        let uri = format!("otpauth-migration://offline?data={}", b64);

        let accounts = parse_migration(&uri).unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].issuer, "Example");
        assert_eq!(accounts[0].label, "alice");
        let expected_b32 = base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &secret);
        assert_eq!(accounts[0].secret, expected_b32);
    }

    #[test]
    fn skips_hotp_and_errors_when_empty() {
        let payload = MigrationPayload {
            otp_parameters: vec![OtpParameters {
                secret: vec![1, 2, 3], name: "x".into(), issuer: "y".into(),
                algorithm: 1, digits: 1, r#type: 1, counter: 5, // HOTP
            }],
            version: 1, batch_size: 1, batch_index: 0, batch_id: 0,
        };
        let mut raw = Vec::new();
        payload.encode(&mut raw).unwrap();
        let uri = format!("otpauth-migration://offline?data={}", base64_encode(&raw));
        assert!(matches!(parse_migration(&uri), Err(AppError::Migration)));
    }

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
}
```

- [ ] **Step 2: Add the `otpauth_decode` helper to `main.rs`**

The migration module calls `crate::otpauth_decode`. Reuse the URL-decode logic. In `src-tauri/src/main.rs`, add a thin re-export:
```rust
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
```

- [ ] **Step 3: Wire the module**

In `src-tauri/src/main.rs` add `mod migration;`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test migration::`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/migration.rs src-tauri/src/main.rs
git commit -m "feat: add Google Authenticator migration parser"
```

---

### Task 12: Tauri command layer + shared state

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs` (managed state, vault path helper, register handlers)

**Interfaces:**
- Consumes: all prior modules.
- Produces these `#[tauri::command]`s (registered in `main.rs`):
  - `vault_exists() -> bool`
  - `create_vault(password: String) -> Result<(), AppError>`
  - `unlock(password: String) -> Result<(), AppError>`
  - `lock() -> Result<(), AppError>`
  - `is_unlocked() -> bool`
  - `list_accounts() -> Result<Vec<AccountView>, AppError>` where `AccountView { id, issuer, label }` (no secret).
  - `current_codes() -> Result<Vec<CodeView>, AppError>` where `CodeView { id, issuer, label, code, remaining }`.
  - `add_manual(issuer, label, secret) -> Result<(), AppError>`
  - `add_from_uri(uri) -> Result<(), AppError>`
  - `decode_qr_file(path) -> Result<Vec<String>, AppError>` (returns otpauth URIs found)
  - `decode_qr_region(x, y, w, h) -> Result<Vec<String>, AppError>`
  - `preview_migration(uri) -> Result<Vec<AccountView>, AppError>`
  - `import_migration(uri, selected_indices) -> Result<usize, AppError>`
  - `remove_account(id) -> Result<(), AppError>`

State: `pub struct AppState { pub vault: Mutex<Vault>, pub path: PathBuf }`.

- [ ] **Step 1: Write the failing test for the view-mapping helper**

Pure logic (the command bodies themselves need a Tauri runtime, so unit-test the mapping helpers). `src-tauri/src/commands.rs`:
```rust
use crate::error::Result;
use crate::model::Account;
use crate::totp;
use crate::vault::Vault;
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
}
```

- [ ] **Step 2: Add the command functions (after the helpers, same file)**

```rust
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
    state.vault.lock().unwrap().create(&state.path, password.as_bytes())
}

#[tauri::command]
pub fn unlock(state: tauri::State<AppState>, password: String) -> Result<()> {
    state.vault.lock().unwrap().unlock(&state.path, password.as_bytes())
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
    crate::secret::decode_secret(&secret)?; // validate
    let acc = Account::new(issuer, label, secret);
    state.vault.lock().unwrap().add(&state.path, acc)
}

#[tauri::command]
pub fn add_from_uri(state: tauri::State<AppState>, uri: String) -> Result<()> {
    let acc = crate::otpauth::parse_otpauth(&uri)?;
    state.vault.lock().unwrap().add(&state.path, acc)
}

#[tauri::command]
pub fn decode_qr_file(path: String) -> Result<Vec<String>> {
    let strings = crate::qr::decode_image_file(&path)?;
    Ok(strings.into_iter().filter(|s| s.starts_with("otpauth")).collect())
}

#[tauri::command]
pub fn decode_qr_region(x: i32, y: i32, w: u32, h: u32) -> Result<Vec<String>> {
    let strings = crate::qr::capture_region(x, y, w, h)?;
    Ok(strings.into_iter().filter(|s| s.starts_with("otpauth")).collect())
}

#[tauri::command]
pub fn preview_migration(uri: String) -> Result<Vec<AccountView>> {
    Ok(crate::migration::parse_migration(&uri)?.iter().map(to_view).collect())
}

#[tauri::command]
pub fn import_migration(state: tauri::State<AppState>, uri: String, selected_indices: Vec<usize>) -> Result<usize> {
    let accounts = crate::migration::parse_migration(&uri)?;
    let mut v = state.vault.lock().unwrap();
    let mut count = 0;
    for (i, acc) in accounts.into_iter().enumerate() {
        if selected_indices.contains(&i) {
            v.add(&state.path, acc)?;
            count += 1;
        }
    }
    Ok(count)
}

#[tauri::command]
pub fn remove_account(state: tauri::State<AppState>, id: String) -> Result<()> {
    state.vault.lock().unwrap().remove(&state.path, &id)
}
```

- [ ] **Step 3: Wire state + handlers in `main.rs`**

Replace `src-tauri/src/main.rs` body (keep the `mod` lines) with:
```rust
fn vault_path(app: &tauri::App) -> std::path::PathBuf {
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
            commands::decode_qr_region,
            commands::preview_migration,
            commands::import_migration,
            commands::remove_account,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```
Add `mod commands;` to the module list, and add `use tauri::Manager;` where needed for `app.path()`.

- [ ] **Step 4: Run tests + build**

Run: `cd src-tauri && cargo test commands:: && cargo build`
Expected: test PASS (1 test); build succeeds.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat: add Tauri command layer and app state"
```

---

### Task 13: Background tick + OS session-lock auto-lock

**Files:**
- Create: `src-tauri/src/session.rs`
- Modify: `src-tauri/src/main.rs` (spawn tick thread, init session listener)

**Interfaces:**
- Consumes: `tauri::AppHandle`, `commands::AppState`.
- Produces:
  - A background thread emitting a `"tick"` event every second (frontend pulls `current_codes` on tick, or we emit codes directly).
  - `pub fn start_session_lock_listener(app: tauri::AppHandle)` — on OS lock, lock the vault and emit `"locked"` event. Windows uses `WTS_SESSION_LOCK`; macOS/Linux best-effort; if unsupported, the function is a no-op and idle-timeout (frontend) is the fallback.

- [ ] **Step 1: Implement the tick thread (in `main.rs setup`)**

Add inside `.setup(|app| { ... })` after `app.manage(...)`:
```rust
let handle = app.handle().clone();
std::thread::spawn(move || {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let _ = tauri::Emitter::emit(&handle, "tick", ());
    }
});
session::start_session_lock_listener(app.handle().clone());
```

- [ ] **Step 2: Implement `session.rs` with platform stubs**

```rust
use tauri::{AppHandle, Emitter, Manager};

/// Lock the vault and notify the frontend.
fn do_lock(app: &AppHandle) {
    if let Some(state) = app.try_state::<crate::commands::AppState>() {
        state.vault.lock().unwrap().lock();
    }
    let _ = app.emit("locked", ());
}

#[cfg(target_os = "windows")]
pub fn start_session_lock_listener(app: AppHandle) {
    // Register for session-change notifications via a hidden message window.
    // Implementation uses windows crate APIs (WTSRegisterSessionNotification +
    // WM_WTSSESSION_CHANGE / WTS_SESSION_LOCK). See windows-rs docs.
    std::thread::spawn(move || {
        windows_session_loop(app);
    });
}

#[cfg(target_os = "windows")]
fn windows_session_loop(app: AppHandle) {
    // Pseudocode-to-implement with the `windows` crate (add as a
    // [target.'cfg(windows)'.dependencies] entry):
    //   - CreateWindowExW hidden message-only window (HWND_MESSAGE)
    //   - WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION)
    //   - GetMessageW loop; on WM_WTSSESSION_CHANGE with wparam==WTS_SESSION_LOCK,
    //     call do_lock(&app)
    let _ = &app; // replaced by real impl in Step 3
}

#[cfg(target_os = "macos")]
pub fn start_session_lock_listener(app: AppHandle) {
    // Observe "com.apple.screenIsLocked" on the distributed notification center.
    // Best-effort; if registration fails, rely on idle timeout.
    let _ = app;
}

#[cfg(target_os = "linux")]
pub fn start_session_lock_listener(app: AppHandle) {
    // Listen on D-Bus for org.freedesktop.login1 session "Lock" signal or
    // org.freedesktop.ScreenSaver ActiveChanged. Best-effort.
    let _ = app;
}

#[allow(dead_code)]
fn _ensure_used(app: &AppHandle) {
    do_lock(app);
}
```

- [ ] **Step 3: Implement the Windows listener concretely**

Add to `src-tauri/Cargo.toml`:
```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
  "Win32_Foundation",
  "Win32_System_RemoteDesktop",
  "Win32_UI_WindowsAndMessaging",
] }
```

Replace `windows_session_loop` with a real message-only window that registers `WTSRegisterSessionNotification` and calls `do_lock(&app)` on `WM_WTSSESSION_CHANGE` where `wparam == WTS_SESSION_LOCK`. Store the `AppHandle` in a thread-local or `static` accessible from the `WNDPROC`. (This is the one platform with a hard auto-lock requirement.)

- [ ] **Step 4: Wire the module + verify build on Windows**

In `src-tauri/src/main.rs` add `mod session;`.

Run: `cd src-tauri && cargo build`
Expected: builds on Windows (and on macOS/Linux with the no-op stubs).

- [ ] **Step 5: Manual verification (Windows)**

Run the app, unlock the vault, press `Win+L` to lock Windows, unlock Windows — the app must now show the unlock screen (vault re-locked). Document the result.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/session.rs src-tauri/src/main.rs src-tauri/Cargo.toml
git commit -m "feat: add 1s tick and OS session-lock auto-lock"
```

---

### Task 14: Frontend IPC wrapper + types

**Files:**
- Create: `src/lib/types.ts`
- Create: `src/lib/ipc.ts`
- Create: `src/lib/ipc.test.ts`

**Interfaces:**
- Produces: typed wrappers around `invoke` for every command in Task 12, plus `onTick`/`onLocked` event subscriptions. `CodeView`, `AccountView` types mirror the Rust views.

- [ ] **Step 1: Write the failing test (mock the Tauri API)**

`src/lib/ipc.test.ts`:
```ts
import { describe, it, expect, vi, beforeEach } from "vitest";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invokeMock(...a) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

import { unlock, currentCodes } from "./ipc";

describe("ipc", () => {
  beforeEach(() => invokeMock.mockReset());

  it("unlock forwards password", async () => {
    invokeMock.mockResolvedValue(undefined);
    await unlock("pw");
    expect(invokeMock).toHaveBeenCalledWith("unlock", { password: "pw" });
  });

  it("currentCodes returns code views", async () => {
    invokeMock.mockResolvedValue([{ id: "1", issuer: "G", label: "a", code: "123456", remaining: 12 }]);
    const codes = await currentCodes();
    expect(codes[0].code).toBe("123456");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test`
Expected: FAIL — `./ipc` not found.

- [ ] **Step 3: Implement types + ipc**

`src/lib/types.ts`:
```ts
export interface AccountView { id: string; issuer: string; label: string; }
export interface CodeView { id: string; issuer: string; label: string; code: string; remaining: number; }
```

`src/lib/ipc.ts`:
```ts
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AccountView, CodeView } from "./types";

export const vaultExists = () => invoke<boolean>("vault_exists");
export const createVault = (password: string) => invoke<void>("create_vault", { password });
export const unlock = (password: string) => invoke<void>("unlock", { password });
export const lock = () => invoke<void>("lock");
export const isUnlocked = () => invoke<boolean>("is_unlocked");
export const listAccounts = () => invoke<AccountView[]>("list_accounts");
export const currentCodes = () => invoke<CodeView[]>("current_codes");
export const addManual = (issuer: string, label: string, secret: string) =>
  invoke<void>("add_manual", { issuer, label, secret });
export const addFromUri = (uri: string) => invoke<void>("add_from_uri", { uri });
export const decodeQrFile = (path: string) => invoke<string[]>("decode_qr_file", { path });
export const decodeQrRegion = (x: number, y: number, w: number, h: number) =>
  invoke<string[]>("decode_qr_region", { x, y, w, h });
export const previewMigration = (uri: string) => invoke<AccountView[]>("preview_migration", { uri });
export const importMigration = (uri: string, selectedIndices: number[]) =>
  invoke<number>("import_migration", { uri, selectedIndices });
export const removeAccount = (id: string) => invoke<void>("remove_account", { id });

export const onTick = (cb: () => void): Promise<UnlistenFn> => listen("tick", () => cb());
export const onLocked = (cb: () => void): Promise<UnlistenFn> => listen("locked", () => cb());
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm run test`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add src/lib/
git commit -m "feat: add typed frontend IPC wrapper"
```

---

### Task 15: Unlock / create-vault screen

**Files:**
- Create: `src/routes/Unlock.svelte`
- Modify: `src/App.svelte` (route between Unlock and Main based on `is_unlocked`/`vault_exists`)

**Interfaces:**
- Consumes: `ipc.{vaultExists, createVault, unlock, isUnlocked, onLocked}`.
- Produces: emits a Svelte event `unlocked` when the vault opens. `App.svelte` holds the `unlocked` boolean and swaps views.

- [ ] **Step 1: Implement `Unlock.svelte`**

```svelte
<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { vaultExists, createVault, unlock } from "../lib/ipc";

  const dispatch = createEventDispatcher();
  let exists = false;
  let password = "";
  let confirm = "";
  let error = "";
  let busy = false;

  onMount(async () => { exists = await vaultExists(); });

  async function submit() {
    error = "";
    busy = true;
    try {
      if (exists) {
        await unlock(password);
      } else {
        if (password.length < 8) { error = "Password must be at least 8 characters"; return; }
        if (password !== confirm) { error = "Passwords do not match"; return; }
        await createVault(password);
      }
      password = ""; confirm = "";
      dispatch("unlocked");
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="unlock">
  <h1>🔐 Auth TOTP</h1>
  <p>{exists ? "Enter your master password" : "Create a master password"}</p>
  <input type="password" bind:value={password} placeholder="Master password"
         on:keydown={(e) => e.key === "Enter" && exists && submit()} />
  {#if !exists}
    <input type="password" bind:value={confirm} placeholder="Confirm password"
           on:keydown={(e) => e.key === "Enter" && submit()} />
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
  <button on:click={submit} disabled={busy}>{exists ? "Unlock" : "Create vault"}</button>
</div>

<style>
  .unlock { display: flex; flex-direction: column; gap: 12px; padding: 32px; max-width: 320px; margin: 0 auto; }
  input { padding: 10px; font-size: 14px; }
  button { padding: 10px; font-weight: 600; cursor: pointer; }
  .error { color: #c0392b; font-size: 13px; }
</style>
```

- [ ] **Step 2: Update `App.svelte` to route**

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import Unlock from "./routes/Unlock.svelte";
  import Main from "./routes/Main.svelte";
  import { isUnlocked, onLocked } from "./lib/ipc";

  let unlocked = false;
  onMount(async () => {
    unlocked = await isUnlocked();
    onLocked(() => { unlocked = false; });
  });
</script>

{#if unlocked}
  <Main on:locked={() => (unlocked = false)} />
{:else}
  <Unlock on:unlocked={() => (unlocked = true)} />
{/if}
```

- [ ] **Step 3: Create a placeholder `Main.svelte` so the app compiles**

```svelte
<script lang="ts"></script>
<main><p>Unlocked. (Main screen built in Task 16.)</p></main>
```

- [ ] **Step 4: Manual verification**

Run: `npm run tauri dev`
Expected: First run shows "Create a master password"; after creating, shows the placeholder Main. Restart → shows "Enter your master password"; wrong password shows the opaque error.

- [ ] **Step 5: Commit**

```bash
git add src/routes/Unlock.svelte src/routes/Main.svelte src/App.svelte
git commit -m "feat: add unlock / create-vault screen with routing"
```

---

### Task 16: Main screen — account list, live codes, lock icon

**Files:**
- Modify: `src/routes/Main.svelte`
- Create: `src/components/AccountCard.svelte`

**Interfaces:**
- Consumes: `ipc.{currentCodes, removeAccount, lock, onTick}`.
- Produces: live-updating list; toolbar with 🔒 lock button (calls `lock()` then dispatches `locked`); each card shows code + countdown + copy + delete; “Add” button dispatches `add` (wired in later tasks via a modal slot).

- [ ] **Step 1: Implement `AccountCard.svelte`**

```svelte
<script lang="ts">
  import type { CodeView } from "../lib/types";
  import { createEventDispatcher } from "svelte";
  export let item: CodeView;
  const dispatch = createEventDispatcher();
  let copied = false;

  async function copy() {
    await navigator.clipboard.writeText(item.code);
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }
</script>

<div class="card">
  <div class="info">
    <span class="issuer">{item.issuer || "—"}</span>
    <span class="label">{item.label}</span>
  </div>
  <button class="code" on:click={copy} title="Copy">
    {item.code.slice(0, 3)} {item.code.slice(3)}
    <span class="remaining" class:warn={item.remaining <= 5}>{item.remaining}s</span>
  </button>
  {#if copied}<span class="copied">Copied</span>{/if}
  <button class="del" on:click={() => dispatch("remove", item.id)} title="Delete">🗑</button>
</div>

<style>
  .card { display: flex; align-items: center; gap: 10px; padding: 10px 12px; border-bottom: 1px solid #eee; }
  .info { display: flex; flex-direction: column; flex: 1; min-width: 0; }
  .issuer { font-weight: 600; }
  .label { font-size: 12px; color: #777; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .code { font-family: monospace; font-size: 20px; letter-spacing: 2px; background: none; border: none; cursor: pointer; display: flex; align-items: center; gap: 8px; }
  .remaining { font-size: 12px; color: #2980b9; }
  .remaining.warn { color: #c0392b; }
  .copied { font-size: 12px; color: #27ae60; }
  .del { background: none; border: none; cursor: pointer; }
</style>
```

- [ ] **Step 2: Implement `Main.svelte`**

```svelte
<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import AccountCard from "../components/AccountCard.svelte";
  import AddMenu from "../components/AddMenu.svelte";
  import type { CodeView } from "../lib/types";
  import { currentCodes, removeAccount, lock, onTick } from "../lib/ipc";

  const dispatch = createEventDispatcher();
  let codes: CodeView[] = [];
  let unlisten: UnlistenFn | null = null;
  let showAdd = false;

  async function refresh() { codes = await currentCodes(); }

  onMount(async () => {
    await refresh();
    unlisten = await onTick(refresh);
  });
  onDestroy(() => unlisten?.());

  async function doLock() {
    await lock();
    dispatch("locked");
  }
  async function remove(id: string) {
    await removeAccount(id);
    await refresh();
  }
</script>

<header>
  <h1>Auth TOTP</h1>
  <div class="actions">
    <button on:click={() => (showAdd = true)} title="Add">＋</button>
    <button on:click={doLock} title="Lock now">🔒</button>
  </div>
</header>

<div class="list">
  {#each codes as item (item.id)}
    <AccountCard {item} on:remove={(e) => remove(e.detail)} />
  {/each}
  {#if codes.length === 0}<p class="empty">No accounts yet. Press ＋ to add one.</p>{/if}
</div>

{#if showAdd}
  <AddMenu on:close={() => { showAdd = false; refresh(); }} />
{/if}

<style>
  header { display: flex; align-items: center; justify-content: space-between; padding: 10px 14px; border-bottom: 1px solid #ddd; }
  h1 { font-size: 16px; margin: 0; }
  .actions button { font-size: 18px; background: none; border: none; cursor: pointer; margin-left: 8px; }
  .list { overflow-y: auto; }
  .empty { text-align: center; color: #999; padding: 32px; }
</style>
```

- [ ] **Step 3: Create a placeholder `AddMenu.svelte` so it compiles**

```svelte
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  const dispatch = createEventDispatcher();
</script>
<div class="overlay" on:click={() => dispatch("close")}>
  <div class="modal" on:click|stopPropagation>
    <p>Add methods built in Tasks 17–19.</p>
    <button on:click={() => dispatch("close")}>Close</button>
  </div>
</div>
<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; }
  .modal { background: #fff; padding: 20px; border-radius: 8px; min-width: 260px; }
</style>
```

- [ ] **Step 4: Manual verification**

Run: `npm run tauri dev`, unlock, confirm the list renders and the 🔒 button returns to the unlock screen. Codes refresh every second (verify a code countdown ticks).

- [ ] **Step 5: Commit**

```bash
git add src/routes/Main.svelte src/components/AccountCard.svelte src/components/AddMenu.svelte
git commit -m "feat: add main screen with live codes and lock button"
```

---

### Task 17: Add account — manual entry

**Files:**
- Modify: `src/components/AddMenu.svelte` (tabbed: Manual / QR / Import)
- Create: `src/components/AddManual.svelte`

**Interfaces:**
- Consumes: `ipc.addManual`.
- Produces: validates non-empty issuer/secret, calls `addManual`, dispatches `added` then `close` on success; shows backend error (e.g. invalid Base32) inline.

- [ ] **Step 1: Implement `AddManual.svelte`**

```svelte
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { addManual } from "../lib/ipc";
  const dispatch = createEventDispatcher();
  let issuer = "", label = "", secret = "", error = "";

  async function save() {
    error = "";
    try {
      await addManual(issuer.trim(), label.trim(), secret.trim());
      dispatch("added");
    } catch (e) { error = String(e); }
  }
</script>

<div class="form">
  <input bind:value={issuer} placeholder="Issuer (e.g. GitHub)" />
  <input bind:value={label} placeholder="Label (e.g. you@example.com)" />
  <input bind:value={secret} placeholder="Secret key (Base32)" />
  {#if error}<p class="error">{error}</p>{/if}
  <button on:click={save} disabled={!issuer || !secret}>Add</button>
</div>

<style>
  .form { display: flex; flex-direction: column; gap: 10px; }
  input { padding: 9px; }
  .error { color: #c0392b; font-size: 13px; }
</style>
```

- [ ] **Step 2: Convert `AddMenu.svelte` into a tabbed modal**

```svelte
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import AddManual from "./AddManual.svelte";
  import AddFromQr from "./AddFromQr.svelte";
  import ImportGoogle from "./ImportGoogle.svelte";
  const dispatch = createEventDispatcher();
  let tab: "manual" | "qr" | "import" = "manual";
  function done() { dispatch("close"); }
</script>

<div class="overlay" on:click={() => dispatch("close")}>
  <div class="modal" on:click|stopPropagation>
    <nav>
      <button class:active={tab==="manual"} on:click={() => tab="manual"}>Manual</button>
      <button class:active={tab==="qr"} on:click={() => tab="qr"}>QR</button>
      <button class:active={tab==="import"} on:click={() => tab="import"}>Import</button>
    </nav>
    {#if tab === "manual"}<AddManual on:added={done} />{/if}
    {#if tab === "qr"}<AddFromQr on:added={done} />{/if}
    {#if tab === "import"}<ImportGoogle on:added={done} />{/if}
    <button class="cancel" on:click={() => dispatch("close")}>Cancel</button>
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; }
  .modal { background: #fff; padding: 18px; border-radius: 8px; min-width: 300px; display: flex; flex-direction: column; gap: 12px; }
  nav { display: flex; gap: 6px; }
  nav button { flex: 1; padding: 6px; cursor: pointer; }
  nav button.active { background: #2980b9; color: #fff; }
  .cancel { margin-top: 4px; }
</style>
```

- [ ] **Step 3: Create placeholder `AddFromQr.svelte` and `ImportGoogle.svelte`**

```svelte
<!-- AddFromQr.svelte -->
<script lang="ts"></script>
<p>QR add built in Task 18.</p>
```
```svelte
<!-- ImportGoogle.svelte -->
<script lang="ts"></script>
<p>Google import built in Task 19.</p>
```

- [ ] **Step 4: Manual verification**

Run dev, open ＋ → Manual, add `issuer=Test, secret=JBSWY3DPEHPK3PXP` → appears in list with a live code. Enter an invalid secret → inline "Invalid Base32 secret".

- [ ] **Step 5: Commit**

```bash
git add src/components/AddMenu.svelte src/components/AddManual.svelte src/components/AddFromQr.svelte src/components/ImportGoogle.svelte
git commit -m "feat: add manual account entry with tabbed add modal"
```

---

### Task 18: Add account — from QR (image file + screen region)

**Files:**
- Modify: `src/components/AddFromQr.svelte`

**Interfaces:**
- Consumes: `@tauri-apps/plugin-dialog` `open`, `ipc.{decodeQrFile, decodeQrRegion, addFromUri}`.
- Produces: "Choose image…" (file picker → decode → add all found otpauth URIs) and "Capture screen region" (prompts numeric x/y/w/h or full-screen scan → decode → add). Shows count added.

- [ ] **Step 1: Implement `AddFromQr.svelte`**

```svelte
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { decodeQrFile, decodeQrRegion, addFromUri } from "../lib/ipc";
  const dispatch = createEventDispatcher();
  let error = "", status = "";

  async function addUris(uris: string[]) {
    let added = 0;
    for (const u of uris) {
      try { await addFromUri(u); added++; } catch (_) { /* skip invalid */ }
    }
    if (added === 0) { error = "No valid TOTP QR found"; return; }
    dispatch("added");
  }

  async function fromFile() {
    error = ""; status = "";
    const selected = await open({
      multiple: false,
      filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg", "bmp", "gif"] }],
    });
    if (!selected || Array.isArray(selected)) return;
    try {
      status = "Decoding…";
      const uris = await decodeQrFile(selected);
      await addUris(uris);
    } catch (e) { error = String(e); } finally { status = ""; }
  }

  async function fromScreen() {
    error = ""; status = "Scanning full screen…";
    try {
      // Full-screen scan: 0,0 with large w/h; backend clamps to monitor bounds.
      const uris = await decodeQrRegion(0, 0, 100000, 100000);
      await addUris(uris);
    } catch (e) { error = String(e); } finally { status = ""; }
  }
</script>

<div class="qr">
  <button on:click={fromFile}>Choose image…</button>
  <button on:click={fromScreen}>Scan screen for QR</button>
  {#if status}<p>{status}</p>{/if}
  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .qr { display: flex; flex-direction: column; gap: 10px; }
  button { padding: 9px; cursor: pointer; }
  .error { color: #c0392b; font-size: 13px; }
</style>
```

- [ ] **Step 2: Manual verification**

Display the Task 10 fixture QR on screen (or save it as a file). "Choose image…" → select it → account added. "Scan screen for QR" with the QR visible → account added.

- [ ] **Step 3: Commit**

```bash
git add src/components/AddFromQr.svelte
git commit -m "feat: add QR import from image file and screen scan"
```

---

### Task 19: Import from Google Authenticator (preview + select)

**Files:**
- Modify: `src/components/ImportGoogle.svelte`

**Interfaces:**
- Consumes: `@tauri-apps/plugin-dialog` `open`, `ipc.{decodeQrFile, previewMigration, importMigration}`.
- Produces: user picks a screenshot of the GA export QR (or pastes the `otpauth-migration://` URI) → preview list with checkboxes → import selected.

- [ ] **Step 1: Implement `ImportGoogle.svelte`**

```svelte
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { decodeQrFile, previewMigration, importMigration } from "../lib/ipc";
  import type { AccountView } from "../lib/types";
  const dispatch = createEventDispatcher();

  let uri = "";
  let preview: AccountView[] = [];
  let selected: boolean[] = [];
  let error = "";

  async function loadFromFile() {
    error = "";
    const file = await open({ multiple: false, filters: [{ name: "Image", extensions: ["png","jpg","jpeg"] }] });
    if (!file || Array.isArray(file)) return;
    try {
      const found = await decodeQrFile(file);
      const mig = found.find((s) => s.startsWith("otpauth-migration://"));
      if (!mig) { error = "No Google Authenticator export QR found"; return; }
      uri = mig;
      await loadPreview();
    } catch (e) { error = String(e); }
  }

  async function loadPreview() {
    error = "";
    try {
      preview = await previewMigration(uri);
      selected = preview.map(() => true);
    } catch (e) { error = String(e); preview = []; }
  }

  async function doImport() {
    const indices = selected.map((v, i) => (v ? i : -1)).filter((i) => i >= 0);
    if (indices.length === 0) return;
    try {
      await importMigration(uri, indices);
      dispatch("added");
    } catch (e) { error = String(e); }
  }
</script>

<div class="import">
  <button on:click={loadFromFile}>Choose export QR image…</button>
  <input bind:value={uri} placeholder="…or paste otpauth-migration:// URI" on:change={loadPreview} />
  {#if error}<p class="error">{error}</p>{/if}
  {#if preview.length}
    <ul>
      {#each preview as p, i}
        <li><label><input type="checkbox" bind:checked={selected[i]} /> {p.issuer || "—"} · {p.label}</label></li>
      {/each}
    </ul>
    <button on:click={doImport}>Import selected</button>
  {/if}
</div>

<style>
  .import { display: flex; flex-direction: column; gap: 10px; }
  button, input { padding: 9px; }
  ul { list-style: none; padding: 0; margin: 0; max-height: 200px; overflow-y: auto; }
  li { padding: 4px 0; }
  .error { color: #c0392b; font-size: 13px; }
</style>
```

- [ ] **Step 2: Manual verification**

In Google Authenticator, Transfer accounts → Export → screenshot the QR. "Choose export QR image…" → select it → preview lists the accounts → uncheck one → Import selected → only checked accounts appear, each with correct live codes (cross-check against the phone).

- [ ] **Step 3: Commit**

```bash
git add src/components/ImportGoogle.svelte
git commit -m "feat: add Google Authenticator import with preview/select"
```

---

### Task 20: Idle auto-lock + clipboard auto-clear (settings)

**Files:**
- Create: `src/lib/settings.ts`
- Modify: `src/routes/Main.svelte` (idle timer), `src/components/AccountCard.svelte` (clipboard clear)

**Interfaces:**
- Produces: `src/lib/settings.ts` with `idleLockMs` (default 5 min) and `clipboardClearMs` (default 20s), persisted via `localStorage`. Idle timer in `Main.svelte` resets on `mousemove`/`keydown`; on expiry calls `lock()` + dispatch `locked`. `AccountCard` clears the clipboard after `clipboardClearMs`.

- [ ] **Step 1: Implement `settings.ts`**

```ts
const KEY = "atotp.settings";
export interface Settings { idleLockMs: number; clipboardClearMs: number; }
const DEFAULTS: Settings = { idleLockMs: 5 * 60_000, clipboardClearMs: 20_000 };

export function loadSettings(): Settings {
  try { return { ...DEFAULTS, ...JSON.parse(localStorage.getItem(KEY) || "{}") }; }
  catch { return { ...DEFAULTS }; }
}
export function saveSettings(s: Settings) { localStorage.setItem(KEY, JSON.stringify(s)); }
```

- [ ] **Step 2: Add idle auto-lock to `Main.svelte`**

Add inside the `<script>` (after existing imports/state):
```ts
  import { loadSettings } from "../lib/settings";
  let idleTimer: number;
  const settings = loadSettings();

  function resetIdle() {
    clearTimeout(idleTimer);
    idleTimer = window.setTimeout(async () => { await lock(); dispatch("locked"); }, settings.idleLockMs);
  }
```
Add to `onMount` after `unlisten = await onTick(refresh);`:
```ts
    resetIdle();
    window.addEventListener("mousemove", resetIdle);
    window.addEventListener("keydown", resetIdle);
```
Add to `onDestroy`:
```ts
    clearTimeout(idleTimer);
    window.removeEventListener("mousemove", resetIdle);
    window.removeEventListener("keydown", resetIdle);
```

- [ ] **Step 3: Add clipboard auto-clear to `AccountCard.svelte`**

Replace the `copy` function:
```ts
  import { loadSettings } from "../lib/settings";
  async function copy() {
    await navigator.clipboard.writeText(item.code);
    copied = true;
    setTimeout(() => (copied = false), 1500);
    const ms = loadSettings().clipboardClearMs;
    if (ms > 0) {
      setTimeout(async () => {
        try {
          const current = await navigator.clipboard.readText();
          if (current === item.code) await navigator.clipboard.writeText("");
        } catch (_) { /* clipboard read may be blocked; ignore */ }
      }, ms);
    }
  }
```

- [ ] **Step 4: Manual verification**

Set a short idle in code (e.g. 5s) temporarily, leave the app idle → it locks. Copy a code, wait past `clipboardClearMs` → clipboard is cleared. Restore the 5-minute default.

- [ ] **Step 5: Commit**

```bash
git add src/lib/settings.ts src/routes/Main.svelte src/components/AccountCard.svelte
git commit -m "feat: add idle auto-lock and clipboard auto-clear"
```

---

### Task 21: Encrypted export / import backup

**Files:**
- Create: `src-tauri/src/backup.rs`
- Modify: `src-tauri/src/commands.rs` (2 commands), `src-tauri/src/main.rs` (register), `src/components/Settings.svelte` (UI), `src/routes/Main.svelte` (settings button)

**Interfaces:**
- Consumes: `vault` internals, `vault::crypto`, `storage`, dialog `save`/`open`.
- Produces:
  - Rust `pub fn export_encrypted(accounts: &[Account], password: &[u8], out_path: &Path) -> Result<()>` — writes a standalone encrypted file (same format as the vault, fresh salt/nonce).
  - `pub fn import_encrypted(in_path: &Path, password: &[u8]) -> Result<Vec<Account>>`.
  - Commands `export_backup(path, password)` and `import_backup(path, password) -> usize` (merges into vault).

- [ ] **Step 1: Write the failing test for `backup.rs`**

`src-tauri/src/backup.rs`:
```rust
use crate::error::{AppError, Result};
use crate::model::Account;
use crate::storage;
use crate::vault::crypto::{decrypt, encrypt, VaultHeader};
use crate::vault::kdf::{derive_key, KdfParams};
use rand::RngCore;
use std::path::Path;

pub fn export_encrypted(accounts: &[Account], password: &[u8], out_path: &Path) -> Result<()> {
    let kdf = KdfParams::default();
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce);
    let key = derive_key(password, &salt, &kdf)?;
    let header = VaultHeader { version: 1, kdf, salt, nonce };
    let plaintext = serde_json::to_vec(accounts).map_err(|_| AppError::Crypto)?;
    let file = encrypt(&key, &header, &plaintext)?;
    storage::write_atomic(out_path, &file)
}

pub fn import_encrypted(in_path: &Path, password: &[u8]) -> Result<Vec<Account>> {
    let file = storage::read_file(in_path)?;
    let plaintext = decrypt(password, &file)?;
    serde_json::from_slice(&plaintext).map_err(|_| AppError::Crypto)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_then_import_round_trips() {
        let path = std::env::temp_dir().join(format!("atotp_backup_{}.bin", uuid::Uuid::new_v4()));
        let accounts = vec![Account::new("GitHub".into(), "alice".into(), "JBSWY3DPEHPK3PXP".into())];
        export_encrypted(&accounts, b"backup-pw", &path).unwrap();
        let restored = import_encrypted(&path, b"backup-pw").unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].issuer, "GitHub");
        assert!(matches!(import_encrypted(&path, b"wrong"), Err(AppError::Crypto)));
        std::fs::remove_file(&path).ok();
    }
}
```

- [ ] **Step 2: Run test to verify it fails, then wire module**

Run: `cd src-tauri && cargo test backup::`
Expected: FAIL — module not declared. Add `mod backup;` to `main.rs`.

- [ ] **Step 3: Run test to verify it passes**

Run: `cd src-tauri && cargo test backup::`
Expected: PASS (1 test).

- [ ] **Step 4: Add commands and a method to expose accounts for export**

Add to `vault/mod.rs` `impl Vault`:
```rust
    pub fn snapshot(&self) -> Result<Vec<Account>> {
        self.state.as_ref().map(|u| u.accounts.clone()).ok_or(AppError::Crypto)
    }
```
Add to `commands.rs`:
```rust
#[tauri::command]
pub fn export_backup(state: tauri::State<AppState>, path: String, password: String) -> Result<()> {
    let accounts = state.vault.lock().unwrap().snapshot()?;
    crate::backup::export_encrypted(&accounts, password.as_bytes(), std::path::Path::new(&path))
}

#[tauri::command]
pub fn import_backup(state: tauri::State<AppState>, path: String, password: String) -> Result<usize> {
    let accounts = crate::backup::import_encrypted(std::path::Path::new(&path), password.as_bytes())?;
    let mut v = state.vault.lock().unwrap();
    let n = accounts.len();
    for a in accounts { v.add(&state.path, a)?; }
    Ok(n)
}
```
Register both in `main.rs` `generate_handler!`.

- [ ] **Step 5: Add frontend `Settings.svelte` + wire backup IPC**

Add to `src/lib/ipc.ts`:
```ts
export const exportBackup = (path: string, password: string) => invoke<void>("export_backup", { path, password });
export const importBackup = (path: string, password: string) => invoke<number>("import_backup", { path, password });
```
`src/components/Settings.svelte`:
```svelte
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import { exportBackup, importBackup } from "../lib/ipc";
  import { loadSettings, saveSettings } from "../lib/settings";
  const dispatch = createEventDispatcher();
  let pw = "", error = "", status = "";
  let s = loadSettings();

  function persist() { saveSettings(s); }

  async function doExport() {
    error = ""; if (!pw) { error = "Enter a backup password"; return; }
    const path = await save({ defaultPath: "auth-totp-backup.bin" });
    if (!path) return;
    try { await exportBackup(path, pw); status = "Exported."; } catch (e) { error = String(e); }
  }
  async function doImport() {
    error = ""; if (!pw) { error = "Enter the backup password"; return; }
    const path = await open({ multiple: false });
    if (!path || Array.isArray(path)) return;
    try { const n = await importBackup(path, pw); status = `Imported ${n}.`; dispatch("changed"); }
    catch (e) { error = String(e); }
  }
</script>

<div class="overlay" on:click={() => dispatch("close")}>
  <div class="modal" on:click|stopPropagation>
    <h2>Settings</h2>
    <label>Auto-lock (minutes)
      <input type="number" min="1" value={s.idleLockMs/60000}
             on:change={(e)=>{s.idleLockMs=Number(e.currentTarget.value)*60000;persist();}} />
    </label>
    <label>Clear clipboard after (seconds)
      <input type="number" min="0" value={s.clipboardClearMs/1000}
             on:change={(e)=>{s.clipboardClearMs=Number(e.currentTarget.value)*1000;persist();}} />
    </label>
    <hr />
    <h3>Encrypted backup</h3>
    <input type="password" bind:value={pw} placeholder="Backup password" />
    <div class="row"><button on:click={doExport}>Export…</button><button on:click={doImport}>Import…</button></div>
    {#if status}<p class="ok">{status}</p>{/if}
    {#if error}<p class="error">{error}</p>{/if}
    <button class="cancel" on:click={() => dispatch("close")}>Close</button>
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display:flex; align-items:center; justify-content:center; }
  .modal { background:#fff; padding:18px; border-radius:8px; min-width:300px; display:flex; flex-direction:column; gap:10px; }
  label { display:flex; justify-content:space-between; align-items:center; gap:8px; }
  .row { display:flex; gap:8px; } .row button { flex:1; }
  .ok { color:#27ae60; } .error { color:#c0392b; }
</style>
```

- [ ] **Step 6: Add a Settings button to `Main.svelte`**

In the `.actions` div add `<button on:click={() => (showSettings = true)} title="Settings">⚙</button>`, declare `let showSettings = false;`, and render at the bottom:
```svelte
{#if showSettings}
  <Settings on:close={() => (showSettings = false)} on:changed={refresh} />
{/if}
```
with `import Settings from "../components/Settings.svelte";`.

- [ ] **Step 7: Manual verification**

Export a backup with a password → file written. Delete an account, then Import the backup → account returns. Wrong backup password → opaque error.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/backup.rs src-tauri/src/commands.rs src-tauri/src/main.rs src-tauri/src/vault/mod.rs src/components/Settings.svelte src/routes/Main.svelte src/lib/ipc.ts
git commit -m "feat: add encrypted backup export/import and settings"
```

---

### Task 22: Cross-platform packaging + README

**Files:**
- Modify: `src-tauri/tauri.conf.json` (bundle metadata, icons)
- Create: `src-tauri/icons/` (icon set), `README.md`
- Create: `.github/workflows/build.yml` (optional CI build matrix)

**Interfaces:**
- Produces: installable bundles per OS (`.msi`/`.exe` on Windows, `.dmg`/`.app` on macOS, `.deb`/`.AppImage` on Linux) and developer docs.

- [ ] **Step 1: Generate icons**

Run: `npm run tauri icon path/to/1024x1024.png`
Expected: populates `src-tauri/icons/` and references in `tauri.conf.json`.

- [ ] **Step 2: Fill bundle metadata in `tauri.conf.json`**

Set `bundle.icon` to the generated icon list, add `bundle.category = "Utility"`, `bundle.shortDescription`, and `bundle.longDescription`.

- [ ] **Step 3: Build per-platform bundles**

Run on each OS: `npm run tauri build`
Expected: Windows → `.msi`/NSIS `.exe`; macOS → `.dmg`/`.app`; Linux → `.deb`/`.AppImage` under `src-tauri/target/release/bundle/`.

- [ ] **Step 4: Write `README.md`**

Document: what the app is (offline TOTP, KeePass-style encryption), build/run instructions (`npm install`, `npm run tauri dev`, `npm run tauri build`), security model (master password, Argon2id, AES-256-GCM, secrets never leave Rust), and the four add methods + backup.

- [ ] **Step 5: (Optional) CI matrix `.github/workflows/build.yml`**

GitHub Actions matrix over `windows-latest`, `macos-latest`, `ubuntu-latest` running `npm ci` + `npm run tauri build`, uploading artifacts. Include the Linux system deps step (`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, etc.).

- [ ] **Step 6: Full verification pass**

Run: `cd src-tauri && cargo test` (all Rust tests) and `npm run test` (all frontend tests).
Expected: all green. Then `npm run tauri build` succeeds on the current platform.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/tauri.conf.json src-tauri/icons README.md .github
git commit -m "chore: cross-platform packaging, icons, and README"
```

---

## Notes for the Implementer

- **SHA-256/512 TOTP test vectors:** the RFC 6238 seeds differ per algorithm (20/32/64 ASCII bytes). The plan's SHA-1 vectors are exact; if a SHA-256/512 assertion fails, recompute the Base32 of the documented ASCII seed — the *expected code* is fixed by the RFC, only the Base32 literal may need correcting.
- **`xcap` version:** the screen-capture crate API moves; if `capture_image()`/`Monitor::all()` signatures differ, adapt to the installed version — the contract (`capture_region` returns decoded otpauth strings) stays the same.
- **Windows session lock (Task 13):** this is the one platform with a hard auto-lock requirement; the macOS/Linux listeners are best-effort and fall back to idle auto-lock (Task 20). Do not let a missing macOS/Linux implementation block the milestone.
- **zeroize coverage:** keep `secret` strings and key bytes in types that zeroize on drop; avoid copying secrets into `String`s that outlive a single command call.
