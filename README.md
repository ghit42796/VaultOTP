# 🔐 VaultOTP

> Fully offline TOTP authenticator for desktop — your 2FA secrets stay encrypted on your machine, never in the cloud.

VaultOTP is a cross-platform (Windows / macOS / Linux) desktop replacement for Google
Authenticator. It generates RFC 6238 time-based one-time passwords from a single
**master-password-encrypted local vault**. Nothing is ever synced, uploaded, or phoned
home — all key derivation, decryption, and code generation happen in the Rust backend and
**secrets never cross the IPC boundary into the UI**.

Built with [Tauri 2](https://tauri.app) (Rust core + WebView) and Svelte + TypeScript.

---

## 📦 Download / Install

Grab the latest installer for your platform from the
[**Releases**](../../releases) page:

| Platform | File | Install |
|----------|------|---------|
| Windows  | `.msi` or `.exe` | Run the installer |
| macOS    | `.dmg` (universal — Intel + Apple Silicon) | Open the `.dmg`, drag VaultOTP to Applications |
| Linux    | `.AppImage` | `chmod +x VaultOTP_*.AppImage && ./VaultOTP_*.AppImage` |
| Linux    | `.deb` | `sudo dpkg -i VaultOTP_*.deb` |

> ⚠️ **The binaries are unsigned.** On first launch your OS may warn that the developer is
> unverified. To proceed:
> - **Windows:** SmartScreen → *More info* → *Run anyway*.
> - **macOS:** right-click the app → *Open* (or System Settings → Privacy & Security →
>   *Open Anyway*). Gatekeeper blocks double-click launch the first time.
>
> Prefer to build it yourself? See [Getting Started](#-getting-started) below. Maintainers:
> see [RELEASING.md](RELEASING.md) for how releases are cut.

---

## ✨ Features

- **100% offline.** No network code, no telemetry, no cloud sync.
- **KeePass-style encryption.** Composite key → Argon2id KDF → AES-256-GCM, with the file
  header bound in as AEAD associated data (tamper-evident).
- **Three ways to add accounts:**
  1. Manual entry (paste a Base32 secret + issuer/label).
  2. Scan a QR code from a **QR image file** (PNG/JPG).
  3. **Import from Google Authenticator** (`otpauth-migration://` export QR, batch).
- **Live codes.** 6/8-digit codes with a per-account countdown; click to copy.
- **Auto-lock.** Manual lock button, idle timeout (default 5 min), and automatic lock on
  **OS session/screen lock** (Windows `WTS_SESSION_LOCK`; best-effort on macOS/Linux).
- **Encrypted export / backup** with the same or a separate password.

> **Non-goals (YAGNI):** HOTP, cloud sync, live camera scanning, and full KDBX file
> interoperability are intentionally out of scope for now.

---

## 🏗️ Architecture

**Core principle: a secret never leaves the Rust backend.** The frontend only renders the
current code and remaining seconds; the master key, decrypted secrets, and TOTP math all
stay in Rust memory.

```
┌─────────────────────────────────────────────┐
│  Frontend (WebView): Svelte + TypeScript      │
│  · unlock screen, account list, add/import UI │
│  · shows code + countdown only — never secret │
└───────────────┬─────────────────────────────┘
                │ Tauri IPC (commands / events)
┌───────────────┴─────────────────────────────┐
│  Backend (Rust core)                          │
│  · vault    Argon2id + AES-256-GCM            │
│  · session  OS session-lock listener          │
│  · totp     RFC 6238 + background 1 s tick     │
│  · qr       image QR decode                     │
│  · migration  Google Authenticator protobuf    │
│  · storage  atomic encrypted-vault file I/O    │
└──────────────────────────────────────────────┘
```

### Vault file format

A single encrypted file in the OS config dir
(`app_config_dir()/vault.bin`, e.g. `%APPDATA%\com.vaultotp.app\` on Windows):

```
[ Header (plaintext, but authenticated as AEAD AAD) ]
  magic        "ATOTP1\0"
  version      u16
  kdf          Argon2id  (salt + memory / iterations / parallelism)
  cipher       AES-256-GCM  (12-byte nonce, regenerated on every save)
[ Ciphertext + GCM tag ]  →  decrypts to JSON: array of accounts
```

A wrong password or any tampering fails with one opaque error —
`"Incorrect password or corrupted vault"` — leaking nothing about which.

---

## 🛠️ Tech Stack

| Layer    | Choice |
|----------|--------|
| Shell    | Tauri 2 (Rust + WebView) |
| Frontend | Svelte 4 + TypeScript + Vite |
| Backend  | Rust (edition 2021) |
| Crypto   | `argon2`, `aes-gcm`, `hmac`, `sha1`, `sha2`, `zeroize` |
| QR decode | `rqrr`, `image` |
| Migration | `prost` (protobuf), `base32` |

---

## 🚀 Getting Started

All platforms need two common toolchains, then a small set of OS-specific build
dependencies (the [Tauri 2 prerequisites](https://tauri.app/start/prerequisites/)):

- [Rust](https://rustup.rs) — stable toolchain (`rustup` installs `cargo`/`rustc`).
- [Node.js](https://nodejs.org) 18+ and npm (CI uses Node 20).

Then, once per platform (see below), install the native deps and run **the same dev
command everywhere**:

```bash
npm install            # install frontend deps (once, after cloning)
npm run tauri dev      # build + launch the app with hot-reload
```

> `npm run tauri dev` rebuilds the Rust backend and serves the Svelte frontend with
> hot-reload; the first run compiles the full crate graph and is slow, subsequent runs are
> fast. Use `npm test` / `cd src-tauri && cargo test` for the test suites (see
> [Testing](#-testing)), and `npm run build` for a frontend-only production build.

### 🪟 Windows

```powershell
# 1. Rust (MSVC toolchain) — https://rustup.rs  (or: winget install Rustlang.Rustup)
# 2. Microsoft C++ Build Tools (provides the MSVC linker `link.exe`):
winget install --id Microsoft.VisualStudio.2022.BuildTools -e `
  --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
# 3. WebView2 runtime — preinstalled on Windows 10/11; otherwise install "Evergreen WebView2".
# 4. Node.js 18+  (winget install OpenJS.NodeJS)

npm install
npm run tauri dev
```

### 🍎 macOS

```bash
# 1. Xcode Command Line Tools (clang, headers, codesign):
xcode-select --install
# 2. Rust (rustup) and Node 18+  — e.g. via Homebrew:  brew install node
#    (WebKit/WKWebView ships with macOS — no extra runtime needed)

npm install
npm run tauri dev
```

### 🐧 Linux (Debian / Ubuntu)

```bash
# 1. WebKitGTK + GTK build dependencies (same set CI uses):
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libxdo-dev \
  libssl-dev \
  pkg-config
# 2. Rust (rustup) and Node.js 18+

npm install
npm run tauri dev
```

> Other distros: install the equivalent of the packages above (WebKitGTK 4.1, GTK 3,
> libsoup3, librsvg, libxdo, OpenSSL, pkg-config). See the
> [Tauri Linux prerequisites](https://tauri.app/start/prerequisites/#linux) for Fedora,
> Arch, openSUSE, etc.

### Build a release bundle

```bash
npm run tauri build    # produces installers/binaries under src-tauri/target/release/bundle
```

Bundles are emitted per host OS: Windows → `.msi` / NSIS `.exe`; macOS → `.dmg` / `.app`;
Linux → `.deb` / `.AppImage`. Build on each target OS (cross-compiling desktop bundles is
not supported here).

---

## 🧪 Testing

The project is built test-first (TDD). Run both suites:

```bash
# Rust backend (TOTP vectors, crypto round-trips, tamper detection, migration, ...)
cd src-tauri && cargo test

# Frontend (Vitest)
npm test
```

The Rust suite includes the official RFC 6238 test vectors (SHA1/256/512, 6/8 digits) and
verifies that flipping a single byte of the vault makes decryption fail.

---

## 📁 Project Structure

```
.
├── index.html                 # Vite entry
├── src/                       # Svelte + TypeScript frontend
│   ├── routes/                #   Unlock / Main screens
│   ├── components/            #   AccountCard, AddManual, AddFromQr, AddMenu
│   └── lib/                   #   typed IPC wrappers + types
├── src-tauri/                 # Rust backend
│   ├── src/
│   │   ├── vault/             #   kdf · crypto · lock/unlock state machine
│   │   ├── totp.rs            #   RFC 6238
│   │   ├── qr.rs              #   image QR decode
│   │   ├── migration.rs       #   Google Authenticator protobuf
│   │   ├── session.rs         #   OS session-lock → auto-lock
│   │   ├── storage.rs         #   atomic vault file I/O
│   │   └── commands.rs        #   Tauri command surface
│   └── tauri.conf.json
└── docs/superpowers/          # design spec + implementation plan
```

---

## 🤖 Developed with Claude Code

This project was designed and implemented with
[Claude Code](https://claude.com/claude-code) using the **Superpowers** spec-driven
workflow. The full paper trail lives in the repo, so the design intent is reproducible:

- **Design spec:** [`docs/superpowers/specs/`](docs/superpowers/specs/) — goals, scope,
  threat model, crypto format, and module responsibilities.
- **Implementation plan:** [`docs/superpowers/plans/`](docs/superpowers/plans/) — the
  task-by-task TDD plan (failing test → minimal impl → passing test → commit) that the
  backend and frontend were built against.

If you're working in this repo with Claude Code, start from those documents — they are the
source of truth for *why* the code is shaped the way it is. Keep the invariants intact when
changing things:

- Secrets must never be returned across a Tauri command/event to the frontend.
- All crypto failures collapse to the single opaque error string.
- All vault writes stay atomic (write `*.tmp` → fsync → rename).
- Sensitive byte buffers are zeroized on drop.

---

## 🔒 Security Notes

- The vault's security rests entirely on your **master password** — choose a strong one;
  there is no recovery if you forget it.
- VaultOTP is a personal-use authenticator and has **not** undergone a third-party security
  audit. Review the crypto in [`src-tauri/src/vault/`](src-tauri/src/vault/) before relying
  on it for high-value secrets.
- Found a vulnerability? Please open a private security advisory rather than a public issue.
