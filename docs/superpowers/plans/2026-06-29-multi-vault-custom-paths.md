# Multi-Vault Custom Paths — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make VaultOTP document-based — read/save the vault from user-chosen paths, switch between multiple vaults via a picker, with a persisted recent-vaults list, plus "Save a copy as…".

**Architecture:** Keep Plan 1's `create_vault`/`unlock`/key-file commands unchanged. Replace the single fixed `AppState.path: PathBuf` with a mutable `current: Mutex<PathBuf>` plus an `AppState::current_path()` accessor; every command that used `&state.path` now uses `&state.current_path()`. A new `set_current_vault(path)` command (called by the picker before unlock/create) repoints `current` and records the path in a Rust-managed `app_config_dir()/config.json` recent-vaults list. The frontend gains a `VaultPicker` screen routed before `Unlock`.

**Tech Stack:** Rust (Tauri 2), `serde`/`serde_json`, `@tauri-apps/plugin-dialog` (`open`/`save`); Svelte 4 + TypeScript; tests via `cargo test` and `vitest`.

This is Plan 2 of 3 (spec: `docs/superpowers/specs/2026-06-29-custom-path-keyfile-export-design.md`). Plan 1 (key-file auth) is already implemented in the working tree. Plan 3 = account secret export.

## Global Constraints

- **Sans-IO core:** `src-tauri/src/vault/**`, `kdf`, `crypto` stay pure. Filesystem and config I/O live in `commands.rs` / `config.rs` / `main.rs`. Keep the recent-list manipulation logic (dedup/cap/order) in a **pure, unit-tested** function; only the file read/write is I/O.
- **No new Rust or JS dependencies.** Only crates already in `src-tauri/Cargo.toml` and packages already in `package.json` (`@tauri-apps/plugin-dialog` is already present).
- **No database / no SQL.** Persistence is the encrypted vault file + a plaintext `config.json` (non-secret paths only). Never put secrets in `config.json`.
- **Arbitrary paths need no new capability:** vault bytes are read/written in Rust via `crate::storage::{read_file, write_atomic}` (native `std::fs`, not the JS fs plugin). The dialog plugin is already allowed in `src-tauri/capabilities/default.json`.
- **Opaque crypto failures:** unlock/serialize failures stay `AppError::Crypto`. Config/path errors use `AppError::Other(String)`.
- **Recent list cap:** 10 entries, most-recent-first, deduplicated by exact path string.
- **Save As = copy only:** `save_vault_as` writes a copy to the chosen path; the active `current` vault and the recent list are unchanged.
- **Rust test command:** `cargo test --manifest-path src-tauri/Cargo.toml <filter>` — the crate is a BINARY (no lib target); do NOT pass `--lib`.
- **Frontend tests:** `npm test` (vitest). Type-check: `npx svelte-check --tsconfig ./tsconfig.json`. Build: `npm run build`. There is no Svelte component-test harness — `.svelte` changes are verified by `svelte-check` + `npm run build`, not unit tests.
- **Commit style (for reference only — execution may skip commits):** Conventional Commits; body ends with `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

---

## Current State (post-Plan-1, verified in working tree)

- `src-tauri/src/commands.rs`: `pub struct AppState { pub vault: Mutex<Vault>, pub path: PathBuf }`. Commands using `&state.path`: `vault_exists`, `vault_mode`, `create_vault`, `unlock`, `add_manual`, `add_from_uri`, `import_migration`, `remove_account`, `import_backup`, `add_keyfile`, `remove_keyfile`, `change_password`. (`export_backup`/`import_backup`/`decode_qr_file` also take their OWN `path` argument — leave those argument paths alone; only the `&state.path` occurrences change.)
- `src-tauri/src/main.rs`: `vault_path(app)` = `app_config_dir()/vault.bin`; `setup` builds `AppState { vault, path }`; `generate_handler!` lists all commands.
- `src/App.svelte`: 2-state — `{#if unlocked} <Main/> {:else} <Unlock/>`. `Main` dispatches `locked`; `Unlock` dispatches `unlocked`.
- `src/lib/ipc.ts`: existing bindings incl. `vaultExists`, `vaultMode`, `createVault`, `unlock`, `lock`, `isUnlocked`.

---

## File Structure

- `src-tauri/src/config.rs` — NEW. `RecentVaults { recent: Vec<String>, last: Option<String> }`; pure `touch(path, max)`; `load(dir)`/`save(dir, &cfg)` I/O; pure `recent_views(recent, exists_fn)`.
- `src-tauri/src/main.rs` — add `mod config;`; startup loads config + sets `current`; builds the new `AppState`; registers new commands.
- `src-tauri/src/commands.rs` — `AppState` gains `current: Mutex<PathBuf>` + `config_dir: PathBuf` (replacing `path`); `current_path()` accessor; swap `&state.path` → `&state.current_path()`; new commands `set_current_vault`, `list_recent_vaults`, `current_vault_path`, `save_vault_as`; `RecentVaultView` serialize struct.
- `src/lib/ipc.ts` + `src/lib/ipc.test.ts` — bindings `setCurrentVault`, `listRecentVaults`, `currentVaultPath`, `saveVaultAs`.
- `src/routes/VaultPicker.svelte` — NEW. Recent list + Open + Create.
- `src/App.svelte` — 3-view routing: `picker → unlock → main`.
- `src/routes/Unlock.svelte` — add a "Switch vault" affordance (dispatch `switch`).
- `src/components/Settings.svelte` — "Vault" section: show current path, "Save a copy as…", "Open a different vault…".

---

### Task 1: Recent-vaults config module

**Files:**
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/main.rs` (add `mod config;` near the other `mod` lines, line ~3-14)
- Test: `src-tauri/src/config.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Produces: `pub struct RecentVaults { pub recent: Vec<String>, pub last: Option<String> }` (serde, `Default`); `RecentVaults::touch(&mut self, path: &str, max: usize)`; `pub fn load(dir: &Path) -> RecentVaults`; `pub fn save(dir: &Path, cfg: &RecentVaults) -> Result<()>`; `pub struct RecentVaultView { pub path: String, pub exists: bool }`; `pub fn recent_views(recent: &[String], exists: impl Fn(&str) -> bool) -> Vec<RecentVaultView>`.

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/config.rs` with ONLY the tests first (so the file exists and the test compiles to a failure):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_moves_to_front_dedups_and_caps() {
        let mut r = RecentVaults::default();
        r.touch("/a", 3);
        r.touch("/b", 3);
        r.touch("/a", 3); // re-touch moves /a to front, no duplicate
        assert_eq!(r.recent, vec!["/a".to_string(), "/b".to_string()]);
        assert_eq!(r.last.as_deref(), Some("/a"));
        r.touch("/c", 3);
        r.touch("/d", 3); // cap = 3 → oldest ("/b") dropped
        assert_eq!(r.recent, vec!["/d".to_string(), "/c".to_string(), "/a".to_string()]);
    }

    #[test]
    fn recent_views_reports_existence() {
        let recent = vec!["/exists".to_string(), "/gone".to_string()];
        let views = recent_views(&recent, |p| p == "/exists");
        assert_eq!(views.len(), 2);
        assert!(views[0].exists && views[0].path == "/exists");
        assert!(!views[1].exists && views[1].path == "/gone");
    }

    #[test]
    fn load_missing_dir_returns_default() {
        let dir = std::env::temp_dir().join(format!("votp_cfg_{}", uuid::Uuid::new_v4()));
        // dir does not exist yet → default
        let cfg = load(&dir);
        assert!(cfg.recent.is_empty() && cfg.last.is_none());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = std::env::temp_dir().join(format!("votp_cfg_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut cfg = RecentVaults::default();
        cfg.touch("/vault/one.bin", 10);
        save(&dir, &cfg).unwrap();
        let back = load(&dir);
        assert_eq!(back.recent, vec!["/vault/one.bin".to_string()]);
        assert_eq!(back.last.as_deref(), Some("/vault/one.bin"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

First add `mod config;` to `src-tauri/src/main.rs` (alphabetically near `mod commands;`). Then run:
Run: `cargo test --manifest-path src-tauri/Cargo.toml config::`
Expected: FAIL to compile — `cannot find type RecentVaults` / `function load` etc.

- [ ] **Step 3: Write minimal implementation**

Prepend to `src-tauri/src/config.rs` (above the `tests` module):

```rust
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Persisted, NON-SECRET app config: the recent vault paths and the last one opened.
/// Stored as plaintext JSON in `app_config_dir()/config.json`. Never put secrets here.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecentVaults {
    #[serde(default)]
    pub recent: Vec<String>,
    #[serde(default)]
    pub last: Option<String>,
}

impl RecentVaults {
    /// Pure: move `path` to the front of `recent` (dedup by exact string), set it as
    /// `last`, and cap the list to `max` entries (oldest dropped).
    pub fn touch(&mut self, path: &str, max: usize) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_string());
        self.recent.truncate(max);
        self.last = Some(path.to_string());
    }
}

/// A recent-vault entry plus whether the file currently exists on disk.
#[derive(Debug, Clone, Serialize)]
pub struct RecentVaultView {
    pub path: String,
    pub exists: bool,
}

/// Pure: pair each recent path with its existence, preserving order.
pub fn recent_views(recent: &[String], exists: impl Fn(&str) -> bool) -> Vec<RecentVaultView> {
    recent.iter().map(|p| RecentVaultView { path: p.clone(), exists: exists(p) }).collect()
}

/// Read `dir/config.json`. Missing or unparseable → default (empty) config.
pub fn load(dir: &Path) -> RecentVaults {
    let p = dir.join("config.json");
    std::fs::read(&p)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Atomically write `dir/config.json`.
pub fn save(dir: &Path, cfg: &RecentVaults) -> Result<()> {
    let p = dir.join("config.json");
    let bytes = serde_json::to_vec_pretty(cfg).map_err(|e| AppError::Other(format!("config serialize: {e}")))?;
    crate::storage::write_atomic(&p, &bytes)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config::`
Expected: PASS (4 tests). Then `cargo build --manifest-path src-tauri/Cargo.toml` → builds (a `dead_code` warning for items not yet called from commands is expected; they're consumed in Tasks 2-4).

- [ ] **Step 5: Commit** — SKIP if executing under a no-git constraint.

```bash
git add src-tauri/src/config.rs src-tauri/src/main.rs
git commit -m "feat(config): recent-vaults config module (touch/load/save)"
```

---

### Task 2: AppState current-path refactor

**Files:**
- Modify: `src-tauri/src/commands.rs` (`AppState` struct + add `current_path()`; replace `&state.path` occurrences)
- Modify: `src-tauri/src/main.rs` (`setup` startup builds the new `AppState`)
- Test: `src-tauri/src/commands.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: `config::{load, RecentVaults}` (Task 1).
- Produces: `pub struct AppState { pub vault: Mutex<Vault>, pub current: Mutex<PathBuf>, pub config_dir: PathBuf }`; `impl AppState { pub fn current_path(&self) -> PathBuf }`.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src-tauri/src/commands.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml app_state_current_path_clones_current`
Expected: FAIL — `AppState` has no field `current`/`config_dir`, no method `current_path`.

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/commands.rs`, replace the `AppState` struct (currently `pub struct AppState { pub vault: Mutex<Vault>, pub path: PathBuf }`) with:

```rust
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
```

Now replace every `&state.path` with `&state.current_path()` in `commands.rs`. Use the Edit tool with replace-all on the exact token `&state.path` → `&state.current_path()`. (Do NOT touch occurrences of a local `path` argument such as in `export_backup`/`import_backup`/`decode_qr_file` — those are `std::path::Path::new(&path)`, a different token.) After replacing, confirm by reading the file that no `state.path` remains.

In `src-tauri/src/main.rs`, change `setup` so it loads config and seeds `current`. Replace the body that builds `AppState` (the `let path = vault_path(app); app.manage(commands::AppState { vault: ..., path });` block) with:

```rust
            use tauri::Manager;
            let config_dir = app.path().app_config_dir().expect("config dir");
            std::fs::create_dir_all(&config_dir).ok();
            let cfg = config::load(&config_dir);
            let current = cfg
                .last
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| config_dir.join("vault.bin"));
            app.manage(commands::AppState {
                vault: std::sync::Mutex::new(vault::Vault::new()),
                current: std::sync::Mutex::new(current),
                config_dir,
            });
```

The existing `vault_path(app)` helper is now unused — delete the `fn vault_path` definition (lines ~35-40) to avoid a dead-code warning. (The default location is now inlined as `config_dir.join("vault.bin")`.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml` (whole suite — the path swap touches many commands)
Expected: PASS (all prior tests + the new one). Then `cargo build --manifest-path src-tauri/Cargo.toml` → clean (Task-1 config `dead_code` warnings shrink as `load` is now used; `touch`/`save`/`recent_views` still pending Task 3).

- [ ] **Step 5: Commit** — SKIP under no-git.

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "refactor(commands): AppState.current path + current_path() accessor"
```

---

### Task 3: Vault-selection commands

**Files:**
- Modify: `src-tauri/src/commands.rs` (add `set_current_vault`, `list_recent_vaults`, `current_vault_path`)
- Modify: `src-tauri/src/main.rs` (register the three commands)
- Test: `src-tauri/src/commands.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: `config::{load, save, recent_views, RecentVaultView}` (Task 1); `AppState::current_path` (Task 2).
- Produces (Tauri commands): `set_current_vault(state, path: String) -> Result<()>`, `list_recent_vaults(state) -> Result<Vec<config::RecentVaultView>>`, `current_vault_path(state) -> String`.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src-tauri/src/commands.rs`:

```rust
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
```

(The test calls a pure-ish inner helper `set_current_vault_inner(&AppState, String)` so it does not need a Tauri `State` wrapper. The `#[tauri::command]` wrapper delegates to it.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml set_current_vault_updates_state_and_recent`
Expected: FAIL — `cannot find function set_current_vault_inner`.

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/commands.rs`, add (near the other commands):

```rust
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
```

In `src-tauri/src/main.rs`, add to `generate_handler!`: `commands::set_current_vault,`, `commands::list_recent_vaults,`, `commands::current_vault_path,`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml` (whole suite)
Expected: PASS. Then `cargo build --manifest-path src-tauri/Cargo.toml` → clean (config helpers now all consumed).

- [ ] **Step 5: Commit** — SKIP under no-git.

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(commands): set_current_vault / list_recent_vaults / current_vault_path"
```

---

### Task 4: `save_vault_as` (Save a copy)

**Files:**
- Modify: `src-tauri/src/commands.rs` (add `save_vault_as`)
- Modify: `src-tauri/src/main.rs` (register it)
- Test: `src-tauri/src/commands.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: `Vault::serialize`, `random_nonce` (existing), `AppState`.
- Produces (Tauri command): `save_vault_as(state, path: String) -> Result<()>` — writes a copy of the current (unlocked) vault to `path`; `current`/recent unchanged.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src-tauri/src/commands.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml save_vault_as`
Expected: FAIL — `cannot find function save_vault_as_inner`.

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/commands.rs`, add:

```rust
/// Shared logic for `save_vault_as`, testable without a Tauri `State` wrapper.
fn save_vault_as_inner(state: &AppState, path: String) -> Result<()> {
    let g = state.vault.lock().unwrap();
    let bytes = g.serialize(random_nonce())?; // Err(Crypto) if locked
    crate::storage::write_atomic(std::path::Path::new(&path), &bytes)
}

#[tauri::command]
pub fn save_vault_as(state: tauri::State<AppState>, path: String) -> Result<()> {
    save_vault_as_inner(&state, path)
}
```

In `src-tauri/src/main.rs`, add `commands::save_vault_as,` to `generate_handler!`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml` (whole suite)
Expected: PASS. Then `cargo build --manifest-path src-tauri/Cargo.toml` → clean.

- [ ] **Step 5: Commit** — SKIP under no-git.

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(commands): save_vault_as (copy current vault to a new path)"
```

---

### Task 5: IPC bindings + tests

**Files:**
- Modify: `src/lib/ipc.ts`
- Modify: `src/lib/ipc.test.ts`

**Interfaces:**
- Consumes (backend): Task 3 & 4 commands.
- Produces (TS): `setCurrentVault(path)`, `listRecentVaults()`, `currentVaultPath()`, `saveVaultAs(path)`, and a `RecentVaultView` type.

- [ ] **Step 1: Write the failing test**

Add to `src/lib/ipc.test.ts` (and update the import line to include the new names):

```ts
  it("setCurrentVault forwards path", async () => {
    invokeMock.mockResolvedValue(undefined);
    await setCurrentVault("/vaults/a.bin");
    expect(invokeMock).toHaveBeenCalledWith("set_current_vault", { path: "/vaults/a.bin" });
  });

  it("listRecentVaults returns entries", async () => {
    invokeMock.mockResolvedValue([{ path: "/a.bin", exists: true }]);
    const r = await listRecentVaults();
    expect(r[0].path).toBe("/a.bin");
    expect(r[0].exists).toBe(true);
  });

  it("saveVaultAs forwards path", async () => {
    invokeMock.mockResolvedValue(undefined);
    await saveVaultAs("/copy.bin");
    expect(invokeMock).toHaveBeenCalledWith("save_vault_as", { path: "/copy.bin" });
  });
```

Update the test's import to: `import { unlock, currentCodes, createVault, setCurrentVault, listRecentVaults, saveVaultAs } from "./ipc";`

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/lib/ipc.test.ts`
Expected: FAIL — the new functions are not exported.

- [ ] **Step 3: Write minimal implementation**

Add to `src/lib/ipc.ts`:

```ts
export interface RecentVaultView { path: string; exists: boolean }
export const setCurrentVault = (path: string) => invoke<void>("set_current_vault", { path });
export const listRecentVaults = () => invoke<RecentVaultView[]>("list_recent_vaults");
export const currentVaultPath = () => invoke<string>("current_vault_path");
export const saveVaultAs = (path: string) => invoke<void>("save_vault_as", { path });
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test -- src/lib/ipc.test.ts` → PASS.
Run: `npx svelte-check --tsconfig ./tsconfig.json` → no new errors.

- [ ] **Step 5: Commit** — SKIP under no-git.

```bash
git add src/lib/ipc.ts src/lib/ipc.test.ts
git commit -m "feat(ipc): bindings for vault selection + save-as"
```

---

### Task 6: VaultPicker screen + App routing

**Files:**
- Create: `src/routes/VaultPicker.svelte`
- Modify: `src/App.svelte` (3-view routing)
- Modify: `src/routes/Unlock.svelte` (add "Switch vault" dispatch)

**Interfaces:**
- Consumes (TS): `listRecentVaults`, `setCurrentVault`, `currentVaultPath` from `src/lib/ipc.ts`; `open`/`save` from `@tauri-apps/plugin-dialog`.

- [ ] **Step 1: Create `VaultPicker.svelte`**

Create `src/routes/VaultPicker.svelte`:

```svelte
<script lang="ts">
  import { onMount, createEventDispatcher } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { listRecentVaults, setCurrentVault, type RecentVaultView } from "../lib/ipc";

  const dispatch = createEventDispatcher();
  let recents: RecentVaultView[] = [];
  let error = "";

  onMount(async () => {
    try { recents = await listRecentVaults(); } catch (e) { error = String(e); }
  });

  async function choose(path: string) {
    error = "";
    try { await setCurrentVault(path); dispatch("selected"); }
    catch (e) { error = String(e); }
  }

  async function openExisting() {
    const p = await open({ multiple: false });
    if (typeof p === "string") await choose(p);
  }

  async function createNew() {
    const p = await save({ defaultPath: "vault.bin" });
    if (typeof p === "string") await choose(p); // Unlock will show "create" since the file does not exist yet
  }
</script>

<div class="picker">
  <div class="logo">🔐</div>
  <h2>VaultOTP</h2>
  <p class="sub">Open a vault or create a new one</p>

  {#if recents.length}
    <div class="recents">
      {#each recents as r}
        <button class="recent" class:missing={!r.exists} on:click={() => choose(r.path)} title={r.path}>
          <span class="name">{r.path.split("/").pop()}</span>
          <span class="path">{r.path}</span>
          {#if !r.exists}<span class="badge">missing</span>{/if}
        </button>
      {/each}
    </div>
  {/if}

  {#if error}<p class="err">{error}</p>{/if}

  <div class="actions">
    <button class="primary" on:click={openExisting}>Open vault…</button>
    <button class="vo-ghost" on:click={createNew}>Create new vault…</button>
  </div>
</div>

<style>
  .picker { display: flex; flex-direction: column; align-items: center; gap: var(--space-3); height: 100%; padding: 36px; }
  .logo { font-size: 42px; }
  h2 { margin: 0; font-size: 20px; color: var(--text); }
  .sub { margin: 0; color: var(--text-muted); font-size: 13px; }
  .recents { width: 100%; display: flex; flex-direction: column; gap: 8px; max-height: 240px; overflow: auto; }
  .recent { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; padding: 10px 12px;
            border: 1px solid var(--border); border-radius: var(--radius-sm); background: var(--surface);
            color: var(--text); cursor: pointer; text-align: left; }
  .recent:hover { border-color: var(--accent); }
  .recent.missing { opacity: .55; }
  .recent .name { font-weight: 600; font-size: 14px; }
  .recent .path { font-size: 11px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 100%; }
  .recent .badge { font-size: 10px; color: var(--danger); }
  .actions { display: flex; gap: 8px; width: 100%; }
  .actions .primary { flex: 1; padding: 12px; border: none; border-radius: var(--radius-sm);
            background: var(--accent); color: var(--accent-contrast); font-weight: 600; cursor: pointer; }
  .err { color: var(--danger); font-size: 13px; margin: 0; }
</style>
```

- [ ] **Step 2: Wire routing in `App.svelte`**

Replace the contents of `src/App.svelte` with a 3-view machine (preserving the lock listener behaviour):

```svelte
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import VaultPicker from "./routes/VaultPicker.svelte";
  import Unlock from "./routes/Unlock.svelte";
  import Main from "./routes/Main.svelte";
  import { isUnlocked, onLocked } from "./lib/ipc";
  import type { UnlistenFn } from "@tauri-apps/api/event";

  type View = "picker" | "unlock" | "main";
  let view: View = "picker";
  let unlistenLocked: UnlistenFn | undefined;

  onMount(async () => {
    if (await isUnlocked()) view = "main";
    unlistenLocked = await onLocked(() => { view = "unlock"; });
  });
  onDestroy(() => { unlistenLocked?.(); });
</script>

{#if view === "main"}
  <Main on:locked={() => (view = "unlock")} on:switchVault={() => (view = "picker")} />
{:else if view === "unlock"}
  <Unlock on:unlocked={() => (view = "main")} on:switch={() => (view = "picker")} />
{:else}
  <VaultPicker on:selected={() => (view = "unlock")} />
{/if}
```

- [ ] **Step 3: Add "Switch vault" to `Unlock.svelte`**

In `src/routes/Unlock.svelte`, the `<script>` already has `createEventDispatcher` (`dispatch`). Add a button below the primary action in the markup:

```svelte
  <button class="vo-ghost" on:click={() => dispatch("switch")}>← Switch vault</button>
```

(No new imports needed; `dispatch` already exists.)

- [ ] **Step 4: Verify**

Run: `npx svelte-check --tsconfig ./tsconfig.json` → no new errors (note `Main` now also emits `switchVault`, wired in Task 7; until then App just listens — harmless).
Run: `npm run build` → succeeds.
Run: `npm test` → existing suite still green.
Manual (if a desktop session is available): `npm run tauri dev` — the picker lists recent vaults, "Create new vault…" opens a save dialog then the create screen; "Open vault…" opens an existing vault to the unlock screen; "← Switch vault" returns to the picker.

- [ ] **Step 5: Commit** — SKIP under no-git.

```bash
git add src/routes/VaultPicker.svelte src/App.svelte src/routes/Unlock.svelte
git commit -m "feat(ui): VaultPicker screen + 3-view routing + switch-vault"
```

---

### Task 7: Settings — current vault, Save-a-copy, Open-another

**Files:**
- Modify: `src/components/Settings.svelte`

**Interfaces:**
- Consumes (TS): `currentVaultPath`, `saveVaultAs` from `src/lib/ipc.ts`; `save` from `@tauri-apps/plugin-dialog` (already imported in Settings).

- [ ] **Step 1: Add a "Vault" section**

In `src/components/Settings.svelte` `<script>`, extend the ipc import to also include `currentVaultPath, saveVaultAs`, add `createEventDispatcher` if not present (Settings already dispatches `close`/`changed`, so `dispatch` exists), and add:

```ts
  import { currentVaultPath, saveVaultAs } from "../lib/ipc";
  let vaultPath = "";
  let vaultStatus = "", vaultError = "";
  (async () => { try { vaultPath = await currentVaultPath(); } catch {} })();

  async function doSaveAs() {
    vaultError = ""; vaultStatus = "";
    const p = await save({ defaultPath: "vault-copy.bin" });
    if (typeof p !== "string") return;
    try { await saveVaultAs(p); vaultStatus = "Copy saved."; }
    catch (e) { vaultError = String(e); }
  }
```

(Adjust the existing `import { ... } from "../lib/ipc"` line to include `currentVaultPath, saveVaultAs` rather than adding a duplicate import. `save` is already imported from `@tauri-apps/plugin-dialog`.)

In the markup, add a `<section class="setting">` (place it after the Security section from Plan 1):

```svelte
    <section class="setting">
      <h3>Vault</h3>
      <p class="hint" title={vaultPath}>Current: {vaultPath}</p>
      <div class="row">
        <button class="vo-ghost" on:click={doSaveAs}>Save a copy as…</button>
        <button class="vo-ghost" on:click={() => dispatch("switchVault")}>Open a different vault…</button>
      </div>
      {#if vaultError}<p class="err">{vaultError}</p>{/if}
      {#if vaultStatus}<p class="hint">{vaultStatus}</p>{/if}
    </section>
```

- [ ] **Step 2: Forward `switchVault` through `Main.svelte`**

`Settings` is rendered by `Main.svelte`. In `src/routes/Main.svelte`, find where `<Settings ... />` is rendered and add a handler that re-dispatches to App. The `Settings` open block looks like `<Settings on:close={...} on:changed={...} />`; add `on:switchVault`:

```svelte
  <Settings on:close={() => (showSettings = false)} on:changed={refresh} on:switchVault={() => dispatch("switchVault")} />
```

`Main` already has `const dispatch = createEventDispatcher();` (it dispatches `locked`). App (Task 6) already listens for `on:switchVault` on `<Main>`. Read the actual `Settings` invocation in `Main.svelte` first and match its existing handlers.

- [ ] **Step 3: Verify**

Run: `npx svelte-check --tsconfig ./tsconfig.json` → no new errors.
Run: `npm run build` → succeeds.
Run: `npm test` → existing suite green.
Manual (if available): in Settings, "Save a copy as…" writes a working copy; "Open a different vault…" returns to the picker.

- [ ] **Step 4: Commit** — SKIP under no-git.

```bash
git add src/components/Settings.svelte src/routes/Main.svelte
git commit -m "feat(ui): Settings vault section — current path, save-as, switch"
```

---

## Self-Review

**Spec coverage (multi-vault portions of the spec §4):**
- Custom path read/save + multiple vaults — Tasks 2-6. ✓
- Recent list + last-opened in Rust `config.json` — Tasks 1-3; startup seeds `current` from `config.last` (Task 2). ✓
- Arbitrary paths, no new capability — vault/config I/O via `storage`/`config` in Rust; dialog already allowed. ✓ (Global Constraints)
- Create-new via save dialog; open via open dialog — Task 6 `VaultPicker`. ✓
- Save As = copy, current unchanged — Task 4 (`save_vault_as` doesn't touch `current`/recent) + Task 7 UI. ✓
- Picker before unlock; switch vaults — Task 6 routing + Task 7 "Open a different vault…". ✓

**Out of scope here (Plan 3):** account secret export.

**Placeholder scan:** none — every step has complete code. The one "read the actual `Settings`/`Main` invocation first" notes (Tasks 6-7) are evidence-grounding instructions, not placeholders; the exact handler strings are given.

**Type/name consistency:** `RecentVaults`, `touch`, `recent_views`, `RecentVaultView`, `set_current_vault_inner`/`save_vault_as_inner`, `current_path`, and the IPC names (`set_current_vault`↔`setCurrentVault`, `list_recent_vaults`↔`listRecentVaults`, `current_vault_path`↔`currentVaultPath`, `save_vault_as`↔`saveVaultAs`) are used identically across tasks. Event names: `Unlock` emits `switch`; `Main`/`Settings` emit `switchVault`; `App` listens for both — consistent across Tasks 6-7.

**Known carry-over (not introduced here):** `Vault::mode()` dead-code from Plan 1 — `save_vault_as`/selection commands don't consume it; it remains for a later use or the final review.
