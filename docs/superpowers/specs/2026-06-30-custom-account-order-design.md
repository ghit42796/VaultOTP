# Design Spec — Custom Account Display Order

- **Date:** 2026-06-30
- **Status:** Approved (design); pending implementation plan
- **Scope:** Let users manually reorder accounts in the list via drag-and-drop in a dedicated "Reorder mode"; the order is stored in the encrypted vault and persists.

---

## 1. Goal

Users can set a custom display order for their accounts by dragging cards in a dedicated Reorder mode. The order is persisted with the vault (encrypted) and is the order shown everywhere the account list appears.

## 2. Non-goals

- No sort presets (A–Z by issuer, by date added, etc.) — manual order only (YAGNI).
- No per-vault preferences beyond the order itself; no backend ordering metadata beyond the existing accounts sequence.
- No new dependencies — drag-and-drop uses native HTML5 drag events.
- No database / no SQL.

## 3. Current state (verified)

- The vault stores accounts as `Vec<Account>` (`Unlocked.accounts` in `src-tauri/src/vault/mod.rs`); `Account` has an `id: String`. The vault exposes `add`/`remove`/`snapshot`/`accounts` — **no reorder**.
- `list_accounts`/`current_codes` (`src-tauri/src/commands.rs`) map the accounts **in Vec order**; `add_*` push to the end. So today's display order is insertion order.
- Mutating commands follow the pattern: lock vault → mutate → `serialize(random_nonce())` → `storage::write_atomic(&state.current_path(), …)`.
- `src/routes/Main.svelte` renders `{#each codes …}` `<AccountCard …>` and already has a `selectMode` toggle (☑) in the header that hides ＋/🔒/⚙ while active; `AccountCard.svelte` has props `item`/`selectMode`/`selected` and events `remove`/`toggle`.

## 4. Data model & persistence

- The custom order **is** the `accounts` Vec order in the encrypted vault. No new field.
- A new pure `Vault::reorder(ids: &[String])` rearranges `accounts` to match `ids`. `ids` must be exactly the current id set (a permutation): same length, every current id present once. A locked vault → `Err(AppError::Crypto)` (consistent with `snapshot`/`accounts`); a non-permutation (missing/extra/duplicate/wrong-length) → `Err(AppError::Other("invalid account order"))`. In both cases the vault is left unchanged.
- A `reorder_accounts(ids)` command persists the new order (serialize + atomic write to the current vault). Order travels with the vault file and is per-vault.
- Sans-IO preserved: `reorder` is pure (no I/O/rand/clock); persistence happens at the commands edge.

## 5. Backend

- `src-tauri/src/vault/mod.rs`: `pub fn reorder(&mut self, ids: &[String]) -> Result<()>`. Implementation: require the vault unlocked; verify `ids` is a permutation of the current account ids (length match + set match, no dupes); rebuild `accounts` in `ids` order (e.g. drain into a `HashMap<String, Account>` then re-collect by `ids`). On a non-permutation, return `Err(AppError::Other("invalid account order"))` without mutating; when locked, `Err(AppError::Crypto)`.
- `src-tauri/src/commands.rs`: `reorder_accounts(state, ids: Vec<String>) -> Result<()>` (with a testable `reorder_accounts_inner(&AppState, Vec<String>)` like the existing `*_inner` helpers): lock vault → `reorder(&ids)?` → `serialize(random_nonce())?` → `write_atomic(&state.current_path(), …)`.
- `src-tauri/src/main.rs`: register `reorder_accounts` in `generate_handler!`.

## 6. Frontend

- `src/lib/ipc.ts`: `reorderAccounts(ids: string[]) => invoke<void>("reorder_accounts", { ids })`.
- `src/routes/Main.svelte`:
  - Add `reorderMode: boolean`. A header toggle (⇅ "Reorder") that is **mutually exclusive** with `selectMode` (entering one cancels the other). While `reorderMode`, hide ＋/🔒/⚙ (same as select mode) and disable copy.
  - Render each item in a draggable row when in reorder mode: a wrapper `<div draggable={reorderMode}>` with native handlers — `dragstart` records the dragged index, `dragover` (`preventDefault`) computes the hover target and reorders the local `codes` array live for feedback, `drop`/`dragend` finalizes. On finalize, call `reorderAccounts(codes.map(c => c.id))` then `refresh()`.
  - Codes keep ticking; since the order is persisted, re-fetches return the same order. (To avoid a tick clobbering an in-progress drag, only apply the per-tick `codes` reassignment when not currently dragging — track a `dragging` flag.)
- `src/components/AccountCard.svelte`: add `reorderMode = false`. When true: show a leading `≡` drag-handle affordance, make the card non-copying (the main click is a no-op / not a button action), and hide the delete button. (Mutually exclusive with `selectMode`'s checkbox.)

## 7. Behavior & edge cases

- 0 or 1 account: nothing to drag (reorder mode still toggles, just no effect).
- A successful drop persists immediately; if `reorderAccounts` fails, `refresh()` restores the persisted order (optimistic update reverts).
- Entering reorder mode cancels select mode (and clears any selection); entering select mode cancels reorder mode.
- `reorder` rejecting a non-permutation id list surfaces `AppError::Other("invalid account order")`; the UI just refreshes to the persisted order.
- Locking / OS auto-lock during reorder: the existing lock flow routes away from Main; no special handling needed.

## 8. Testing

- **Pure (vault):** `reorder` reorders to the given permutation; rejects a list with a missing id, an extra id, a duplicate, or wrong length (vault unchanged); a single/zero-account vault round-trips.
- **Command:** `reorder_accounts_inner` persists the new order (build an unlocked vault with 3 accounts, reorder, re-read the written bytes, confirm order); errors when locked.
- **IPC:** `reorderAccounts` arg-shape test (`{ ids }`).
- **Components:** no Svelte test harness — verify reorder mode (drag to reorder, persisted across lock/unlock; copy disabled in reorder mode; select/reorder mutual exclusion) via `svelte-check` + `npm run build` + manual.

## 9. Files touched

- `src-tauri/src/vault/mod.rs` — `reorder` + tests.
- `src-tauri/src/commands.rs` — `reorder_accounts` (+ `_inner`) + tests.
- `src-tauri/src/main.rs` — register command.
- `src/lib/ipc.ts` + `src/lib/ipc.test.ts` — `reorderAccounts` + test.
- `src/routes/Main.svelte` — reorder mode toggle, drag-and-drop list, mutual exclusion.
- `src/components/AccountCard.svelte` — `reorderMode` prop (handle, no-copy, no-delete).

## 10. Open risks

- Native HTML5 DnD quirks (the `dragover` `preventDefault` is required to allow a drop; `dragend` must fire even on an invalid drop). The plan must handle a drag that ends outside the list (no reorder) gracefully.
- The per-tick `codes` refresh vs an in-progress drag — gate the reassignment on a `dragging` flag (§6) so a tick can't reset the list mid-drag.
