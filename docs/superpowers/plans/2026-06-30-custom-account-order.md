# Custom Account Display Order — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users drag-and-drop accounts into a custom order in a dedicated Reorder mode, persisted in the encrypted vault.

**Architecture:** A pure `Vault::reorder(ids)` rearranges the in-memory `accounts` Vec to a validated permutation; a `reorder_accounts` command persists it (serialize + atomic write). The frontend adds a Reorder-mode toggle (mutually exclusive with Select mode) that makes cards draggable via native HTML5 drag events; dropping persists the new id order. No backend ordering metadata, no new dependencies.

**Tech Stack:** Rust (Tauri 2) sans-IO core; Svelte 4 + TypeScript; native HTML5 drag-and-drop; tests via `cargo test` + `vitest`.

Spec: `docs/superpowers/specs/2026-06-30-custom-account-order-design.md`.

## Global Constraints

- **Sans-IO core:** `Vault::reorder` is pure (no I/O/rand/clock). Persistence (serialize + `write_atomic`) happens only at the `commands.rs` edge, following the existing `*_inner` pattern.
- **Order = the `accounts` Vec order in the encrypted vault.** No new field, no DB, no SQL.
- **No new dependencies.** Drag-and-drop uses native HTML5 drag events.
- **Errors:** locked vault → `AppError::Crypto` (consistent with `snapshot`/`accounts`); a non-permutation `ids` (missing/extra/duplicate/wrong-length) → `AppError::Other("invalid account order")`. The vault is left unchanged on a non-permutation (validate before mutating).
- **Reorder mode and Select mode are mutually exclusive** — entering one cancels the other; while either is active, the ＋/🔒/⚙ tools are hidden.
- **Rust test command:** `cargo test --manifest-path src-tauri/Cargo.toml <filter>` — BINARY crate; never pass `--lib`.
- **Frontend:** `npm test` (vitest); `npx svelte-check --tsconfig ./tsconfig.json` (keep it at **0 errors and 0 warnings** — the codebase standard; use `<!-- svelte-ignore <rule> -->` for unavoidable a11y warnings, as `Settings.svelte` already does); `npm run build`.
- **Commit style (reference only — execution may skip commits):** Conventional Commits; body ends with `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`.

---

## File Structure

- `src-tauri/src/vault/mod.rs` — add pure `reorder` + tests.
- `src-tauri/src/commands.rs` — add `reorder_accounts` (+ `reorder_accounts_inner`) + tests.
- `src-tauri/src/main.rs` — register `reorder_accounts`.
- `src/lib/ipc.ts` + `src/lib/ipc.test.ts` — `reorderAccounts` binding + test.
- `src/components/AccountCard.svelte` — `reorderMode` prop (drag handle, disable copy, hide delete).
- `src/routes/Main.svelte` — Reorder-mode toggle, draggable list, mutual exclusion, drag-gated refresh.

---

### Task 1: `Vault::reorder`

**Files:**
- Modify: `src-tauri/src/vault/mod.rs` (add method to `impl Vault`, after `remove`)
- Test: `src-tauri/src/vault/mod.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Produces: `pub fn reorder(&mut self, ids: &[String]) -> Result<()>`.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src-tauri/src/vault/mod.rs` (the existing `acc(id)` and `fast_kdf()` helpers create accounts/params):

```rust
    #[test]
    fn reorder_to_permutation() {
        let mut v = Vault::create_unlocked(Credential::Password(b"pw"), [9u8; 16], fast_kdf()).unwrap();
        v.add(acc("a")).unwrap(); v.add(acc("b")).unwrap(); v.add(acc("c")).unwrap();
        v.reorder(&["c".to_string(), "a".to_string(), "b".to_string()]).unwrap();
        let ids: Vec<&str> = v.accounts().unwrap().iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, vec!["c", "a", "b"]);
    }

    #[test]
    fn reorder_rejects_non_permutation_and_leaves_unchanged() {
        let mut v = Vault::create_unlocked(Credential::Password(b"pw"), [9u8; 16], fast_kdf()).unwrap();
        v.add(acc("a")).unwrap(); v.add(acc("b")).unwrap();
        assert!(matches!(v.reorder(&["a".to_string()]), Err(AppError::Other(_))));                          // missing
        assert!(matches!(v.reorder(&["a".to_string(), "b".to_string(), "x".to_string()]), Err(AppError::Other(_)))); // extra
        assert!(matches!(v.reorder(&["a".to_string(), "a".to_string()]), Err(AppError::Other(_))));         // duplicate
        let ids: Vec<&str> = v.accounts().unwrap().iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b"]); // unchanged
    }

    #[test]
    fn reorder_locked_errors() {
        let mut v = Vault::new();
        assert!(matches!(v.reorder(&["a".to_string()]), Err(AppError::Crypto)));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml vault::tests::reorder_to_permutation`
Expected: FAIL — `no method named reorder`.

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/vault/mod.rs`, add to `impl Vault` (right after the `remove` method):

```rust
    /// Pure: rearrange `accounts` to match `ids`, which must be a permutation of the
    /// current account ids. Locked -> Err(Crypto). Non-permutation -> Err(Other), unchanged.
    pub fn reorder(&mut self, ids: &[String]) -> Result<()> {
        let u = self.state.as_mut().ok_or(AppError::Crypto)?;
        let is_permutation = {
            let current: std::collections::HashSet<&str> = u.accounts.iter().map(|a| a.id.as_str()).collect();
            let requested: std::collections::HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();
            ids.len() == u.accounts.len() && requested.len() == ids.len() && requested == current
        };
        if !is_permutation {
            return Err(AppError::Other("invalid account order".into()));
        }
        let mut map: std::collections::HashMap<String, Account> =
            u.accounts.drain(..).map(|a| (a.id.clone(), a)).collect();
        u.accounts = ids.iter().map(|id| map.remove(id).expect("validated permutation")).collect();
        Ok(())
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml vault`
Expected: PASS (the 3 new tests + existing vault tests). Then `cargo build --manifest-path src-tauri/Cargo.toml` → clean (a `dead_code` warning for `reorder` until Task 2 is expected).

- [ ] **Step 5: Commit** — SKIP under no-git.

```bash
git add src-tauri/src/vault/mod.rs
git commit -m "feat(vault): reorder accounts to a validated permutation"
```

---

### Task 2: `reorder_accounts` command

**Files:**
- Modify: `src-tauri/src/commands.rs` (add command + `reorder_accounts_inner`)
- Modify: `src-tauri/src/main.rs` (register)
- Test: `src-tauri/src/commands.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: `Vault::reorder` (Task 1); `AppState`/`current_path`/`random_nonce`/`storage::write_atomic` (existing).
- Produces (Tauri command): `reorder_accounts(state, ids: Vec<String>) -> Result<()>`.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src-tauri/src/commands.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml reorder_accounts_inner_errors_when_locked`
Expected: FAIL — `cannot find function reorder_accounts_inner`.

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/commands.rs`, add (near the other `*_inner` helpers / commands):

```rust
/// Shared logic for `reorder_accounts`, testable without a Tauri `State` wrapper.
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
```

In `src-tauri/src/main.rs`, add `commands::reorder_accounts,` to `generate_handler!`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml` (whole suite)
Expected: PASS. Then `cargo build --manifest-path src-tauri/Cargo.toml` → clean (the Task-1 `reorder` dead_code warning clears now).

- [ ] **Step 5: Commit** — SKIP under no-git.

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(commands): reorder_accounts persists custom order"
```

---

### Task 3: IPC `reorderAccounts` binding

**Files:**
- Modify: `src/lib/ipc.ts`, `src/lib/ipc.test.ts`

**Interfaces:**
- Produces (TS): `reorderAccounts(ids: string[]): Promise<void>`.

- [ ] **Step 1: Write the failing test**

Add to `src/lib/ipc.test.ts` (extend the import line to include `reorderAccounts`):

```ts
  it("reorderAccounts forwards ids", async () => {
    invokeMock.mockResolvedValue(undefined);
    await reorderAccounts(["a", "b", "c"]);
    expect(invokeMock).toHaveBeenCalledWith("reorder_accounts", { ids: ["a", "b", "c"] });
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/lib/ipc.test.ts`
Expected: FAIL — `reorderAccounts` is not exported.

- [ ] **Step 3: Write minimal implementation**

Add to `src/lib/ipc.ts`:

```ts
export const reorderAccounts = (ids: string[]) => invoke<void>("reorder_accounts", { ids });
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test -- src/lib/ipc.test.ts` → PASS. Then `npx svelte-check --tsconfig ./tsconfig.json` → no new errors.

- [ ] **Step 5: Commit** — SKIP under no-git.

```bash
git add src/lib/ipc.ts src/lib/ipc.test.ts
git commit -m "feat(ipc): reorderAccounts binding"
```

---

### Task 4: `AccountCard` reorder-mode prop

**Files:**
- Modify: `src/components/AccountCard.svelte`

**Interfaces:**
- Produces: `AccountCard` prop `reorderMode = false` — when true, shows a leading `≡` drag handle, disables copy (click is a no-op), and hides the delete button.

- [ ] **Step 1: Edit the component**

In `src/components/AccountCard.svelte`:

Add the prop (after `export let selected = false;`):

```ts
  export let reorderMode = false;
```

Replace `activate()` so it does nothing in reorder mode:

```ts
  async function activate() {
    if (reorderMode) return;
    if (selectMode) { dispatch("toggle", item.id); return; }
    await copy();
  }
```

In the `.main` button, update the `title` and add the handle before the badge. Replace the button's opening tag + the `{#if selectMode}` check line with:

```svelte
  <button
    class="main"
    on:click={activate}
    aria-pressed={selectMode ? selected : undefined}
    title={reorderMode ? "Drag to reorder" : selectMode ? "Toggle selection" : "Copy code"}
  >
    {#if reorderMode}<span class="handle" aria-hidden="true">≡</span>{/if}
    {#if selectMode}<span class="check" class:on={selected} aria-hidden="true"></span>{/if}
```

Change the delete-button guard from `{#if !selectMode}` to:

```svelte
  {#if !selectMode && !reorderMode}
    <button class="del" on:click|stopPropagation={() => dispatch("remove", item.id)} title="Delete" aria-label="Delete">🗑</button>
  {/if}
```

Add the handle style to the `<style>` block (next to `.check`):

```css
  .handle { flex: 0 0 auto; color: var(--text-muted); font-size: 16px; line-height: 1; cursor: grab; }
```

- [ ] **Step 2: Verify**

Run: `npx svelte-check --tsconfig ./tsconfig.json` → 0 errors/warnings (the new prop has a default, so existing `<AccountCard {item} …>` call sites are unaffected).
Run: `npm run build` → succeeds.
Run: `npm test` → green.

- [ ] **Step 3: Commit** — SKIP under no-git.

```bash
git add src/components/AccountCard.svelte
git commit -m "feat(ui): AccountCard reorder-mode (drag handle, no copy/delete)"
```

---

### Task 5: Reorder mode + drag-and-drop in `Main.svelte`

**Files:**
- Modify: `src/routes/Main.svelte`

**Interfaces:**
- Consumes: `reorderAccounts` (Task 3); `AccountCard` `reorderMode` prop (Task 4); existing `codes`/`selectMode`/`selected`/`toggleSelectMode`/`refresh`.

- [ ] **Step 1: Add reorder state, handlers, and a drag-gated refresh**

In `src/routes/Main.svelte` `<script>`:

Add `reorderAccounts` to the `../lib/ipc` import (the line that imports `currentCodes, removeAccount, lock, onTick, exportSecrets, type ExportFormat`).

Add state after `let exportError = "";`:

```ts
  let reorderMode = false;
  let dragIndex: number | null = null;
  let dragging = false;
```

Replace `refresh` so a tick can't clobber an in-progress drag:

```ts
  async function refresh() { if (dragging) return; codes = await currentCodes(); }
```

Replace `toggleSelectMode` to cancel reorder mode when entering select mode:

```ts
  function toggleSelectMode() {
    selectMode = !selectMode;
    if (selectMode) { reorderMode = false; }
    if (!selectMode) {
      selected = new Set();
      exportStatus = "";
      exportError = "";
    }
  }
```

Add the reorder handlers (after `toggleSelectMode`):

```ts
  function toggleReorderMode() {
    reorderMode = !reorderMode;
    if (reorderMode) { selectMode = false; selected = new Set(); }
    dragIndex = null;
    dragging = false;
  }

  function onDragStart(i: number) { dragIndex = i; dragging = true; }

  function onDragOver(e: DragEvent, i: number) {
    e.preventDefault();
    if (dragIndex === null || dragIndex === i) return;
    const arr = [...codes];
    const [moved] = arr.splice(dragIndex, 1);
    arr.splice(i, 0, moved);
    codes = arr;
    dragIndex = i;
  }

  async function persistOrder() {
    if (!dragging) return;
    dragging = false;
    dragIndex = null;
    try { await reorderAccounts(codes.map((c) => c.id)); }
    catch (_) { /* ignore; refresh restores the persisted order */ }
    await refresh();
  }
```

- [ ] **Step 2: Add the header toggle**

In the `.tools` block of the header, replace the existing tools markup with (this hides ＋/🔒/⚙ in either mode, shows the Select toggle only when not reordering, and the Reorder toggle only when not selecting):

```svelte
  <div class="tools">
    {#if !selectMode && !reorderMode}
      <button class="tool" on:click={() => (showAdd = true)} title="Add account" aria-label="Add account">＋</button>
      <button class="tool" on:click={doLock} title="Lock now" aria-label="Lock now">🔒</button>
      <button class="tool" on:click={() => (showSettings = true)} title="Settings" aria-label="Settings">⚙</button>
    {/if}
    {#if !reorderMode}
      <button class="tool" class:tool-active={selectMode} on:click={toggleSelectMode} title={selectMode ? "Cancel selection" : "Select accounts to export"} aria-label={selectMode ? "Cancel selection" : "Select accounts"}>
        {selectMode ? "✕" : "☑"}
      </button>
    {/if}
    {#if !selectMode}
      <button class="tool" class:tool-active={reorderMode} on:click={toggleReorderMode} title={reorderMode ? "Done reordering" : "Reorder accounts"} aria-label={reorderMode ? "Done reordering" : "Reorder accounts"}>
        {reorderMode ? "✓" : "⇅"}
      </button>
    {/if}
  </div>
```

- [ ] **Step 3: Make the list draggable in reorder mode**

Replace the `{#each codes …}` body inside `<div class="list">` so reorder mode wraps each card in a draggable row:

```svelte
  {#each codes as item, i (item.id)}
    {#if reorderMode}
      <div
        class="drag-row"
        draggable="true"
        on:dragstart={() => onDragStart(i)}
        on:dragover={(e) => onDragOver(e, i)}
        on:drop={persistOrder}
        on:dragend={persistOrder}
      >
        <AccountCard {item} reorderMode={true} />
      </div>
    {:else}
      <AccountCard
        {item}
        {selectMode}
        selected={selected.has(item.id)}
        on:remove={(e) => remove(e.detail)}
        on:toggle={(e) => toggleSelect(e.detail)}
      />
    {/if}
  {/each}
```

Add a `.drag-row` style near the other list styles:

```css
  .drag-row { cursor: grab; }
  .drag-row:active { cursor: grabbing; }
```

- [ ] **Step 4: Verify (and silence any a11y warning the drag-row triggers)**

Run: `npx svelte-check --tsconfig ./tsconfig.json`.
If it reports an a11y warning on the `.drag-row` `<div>` (e.g. `a11y-no-static-element-interactions` for the drag handlers), add the exact `<!-- svelte-ignore <rule-name> -->` comment immediately above the `<div class="drag-row" …>` (mirroring how `src/components/Settings.svelte` silences its overlay warnings). Re-run until it reports **0 errors and 0 warnings**.
Run: `npm run build` → succeeds.
Run: `npm test` → green.
Manual (if a desktop session is available): toggle ⇅ Reorder mode — ＋/🔒/⚙ and the Select toggle hide; each card shows a ≡ handle and is draggable; dragging reorders live and dropping persists (the order survives lock → unlock and reopening the vault); entering Select mode (when available) exits Reorder mode and vice-versa; clicking a card in reorder mode does not copy.

- [ ] **Step 5: Commit** — SKIP under no-git.

```bash
git add src/routes/Main.svelte
git commit -m "feat(ui): drag-and-drop reorder mode with persisted order"
```

---

## Self-Review

**Spec coverage:**
- §4 order = vault Vec; pure `reorder` validates permutation; persist via command — Tasks 1, 2. ✓
- §5 backend `reorder` (locked→Crypto, non-perm→Other, unchanged) + `reorder_accounts`(+`_inner`) + register — Tasks 1, 2. ✓
- §6 `reorderAccounts` IPC; Main reorder toggle (mutually exclusive with select, hides tools), native HTML5 DnD, persist-on-drop, `dragging`-gated refresh; AccountCard `reorderMode` — Tasks 3, 4, 5. ✓
- §7 edge cases (0/1 account no-op; failed persist reverts via refresh; mutual exclusion; opaque-ish errors) — Tasks 1, 5. ✓
- §8 tests: vault reorder (permute + reject + locked), command inner (persist + locked), ipc arg-shape, components via svelte-check/build/manual — Tasks 1–5. ✓

**Placeholder scan:** none — every step has complete code or exact edits. The Task-5 Step-4 a11y instruction names the concrete action (add the exact `<!-- svelte-ignore <rule> -->` from the warning), matching the existing `Settings.svelte` pattern.

**Type/name consistency:** `reorder(&mut self, ids: &[String])` (Task 1) ↔ `reorder_accounts_inner`/`reorder_accounts(ids: Vec<String>)` (Task 2) ↔ `reorderAccounts(ids: string[])` / invoke arg `{ ids }` (Task 3) ↔ `AccountCard reorderMode` (Task 4) consumed in Main (Task 5). Errors: locked→`Crypto`, non-permutation→`Other` used consistently. Task ordering keeps every build green (`reorder` dead_code clears in Task 2; `reorderMode` prop has a default so Task 4 doesn't break existing call sites before Task 5 wires it).
