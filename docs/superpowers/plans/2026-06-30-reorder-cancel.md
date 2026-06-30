# Reorder Mode Confirm/Cancel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give reorder mode an explicit Confirm (`✓`) and Cancel (`✕`), where dragging only rearranges on-screen and nothing is persisted until Confirm; Cancel reverts to the order from when reorder mode was entered.

**Architecture:** Today every drop calls `reorderAccounts` (instant save) and the `✓` button merely exits. This plan switches to a **defer-save** model: entering reorder mode snapshots the current id order; dragging mutates only the local `codes` array; `✓` persists once; `✕` restores the snapshot. The drag math (move-element, restore-by-id-order) is extracted into a pure `src/lib/reorder.ts` module so it is unit-testable; `Main.svelte` becomes a thin orchestrator. Backend (`Vault::reorder` / `reorder_accounts`) is unchanged.

**Tech Stack:** Svelte 4 + TypeScript; native HTML5 drag-and-drop; vitest (node env) for the pure module; manual drag verification for the component wiring.

## Global Constraints

- **No new dependencies.** Pure TS + existing IPC only.
- **No backend changes.** `reorderAccounts(ids: string[])` IPC and `Vault::reorder` stay as-is.
- **Persistence is deferred to Confirm.** Drag/drop must never call `reorderAccounts`; only `confirmReorder` does.
- **Refresh must not clobber an in-progress arrangement.** While `reorderMode` is true, the periodic `tick` refresh must skip reloading `codes`.
- **Deletes stay disabled in reorder mode** (already true via the `!reorderMode` guard on the delete button), so a snapshot's ids always match the live ids.

---

### Task 1: Pure reorder helpers (`moveItem`, `restoreOrder`)

**Files:**
- Create: `src/lib/reorder.ts`
- Test: `src/lib/reorder.test.ts`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `moveItem<T>(arr: T[], from: number, to: number): T[]` — returns a **new** array with the element at `from` relocated to index `to`; input is not mutated.
  - `restoreOrder<T extends { id: string }>(items: T[], idOrder: string[]): T[]` — returns `items` reordered to follow `idOrder`; ids absent from `items` are skipped, items absent from `idOrder` are dropped.

- [ ] **Step 1: Write the failing test**

Create `src/lib/reorder.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { moveItem, restoreOrder } from "./reorder";

describe("reorder helpers", () => {
  it("moveItem moves an element later without mutating the input", () => {
    const arr = ["a", "b", "c", "d"];
    expect(moveItem(arr, 0, 2)).toEqual(["b", "c", "a", "d"]);
    expect(arr).toEqual(["a", "b", "c", "d"]); // input untouched
  });

  it("moveItem moves an element earlier", () => {
    expect(moveItem(["a", "b", "c", "d"], 3, 1)).toEqual(["a", "d", "b", "c"]);
  });

  it("moveItem is a no-op when from === to", () => {
    expect(moveItem(["a", "b", "c"], 1, 1)).toEqual(["a", "b", "c"]);
  });

  it("restoreOrder reorders items to match the id snapshot", () => {
    const items = [{ id: "2" }, { id: "3" }, { id: "1" }];
    expect(restoreOrder(items, ["1", "2", "3"])).toEqual([{ id: "1" }, { id: "2" }, { id: "3" }]);
  });

  it("restoreOrder ignores unknown ids and drops items missing from the snapshot", () => {
    const items = [{ id: "1" }, { id: "2" }];
    expect(restoreOrder(items, ["2", "9", "1"])).toEqual([{ id: "2" }, { id: "1" }]);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- reorder`
Expected: FAIL — cannot resolve `./reorder` (module not yet created).

- [ ] **Step 3: Write minimal implementation**

Create `src/lib/reorder.ts`:

```ts
/** Return a new array with the element at `from` moved to index `to`. Input is not mutated. */
export function moveItem<T>(arr: T[], from: number, to: number): T[] {
  const next = [...arr];
  const [moved] = next.splice(from, 1);
  next.splice(to, 0, moved);
  return next;
}

/**
 * Reorder `items` to follow the id sequence in `idOrder`.
 * Ids with no matching item are skipped; items whose id is absent from
 * `idOrder` are dropped. Used to restore the pre-reorder snapshot on cancel.
 */
export function restoreOrder<T extends { id: string }>(items: T[], idOrder: string[]): T[] {
  const byId = new Map(items.map((it) => [it.id, it]));
  return idOrder
    .map((id) => byId.get(id))
    .filter((it): it is T => it !== undefined);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- reorder`
Expected: PASS — 5 passing.

- [ ] **Step 5: Commit**

```bash
git add src/lib/reorder.ts src/lib/reorder.test.ts
git commit -m "feat(reorder): pure moveItem/restoreOrder helpers"
```

---

### Task 2: Wire defer-save + Confirm/Cancel into `Main.svelte`

**Files:**
- Modify: `src/routes/Main.svelte` (script: state + functions; markup: header buttons + drag-row handlers)

**Interfaces:**
- Consumes: `moveItem`, `restoreOrder` from `../lib/reorder`; `reorderAccounts` from `../lib/ipc` (already imported).
- Produces: UI behavior only — no exports.

> Manual-verification task (no Svelte component test harness exists; vitest env is `node`). The pure logic is already covered by Task 1; this task covers orchestration and is verified by `svelte-check` + a manual drag.

- [ ] **Step 1: Add the import**

In the script block of `src/routes/Main.svelte`, directly under the existing `../lib/ipc` import (line 8), add:

```ts
  import { moveItem, restoreOrder } from "../lib/reorder";
```

- [ ] **Step 2: Replace the reorder state**

Find (lines ~26-27):

```ts
  let dragIndex: number | null = null;
  let dragging = false;
```

Replace with:

```ts
  let dragIndex: number | null = null;
  let orderSnapshot: string[] = [];
```

- [ ] **Step 3: Gate `refresh` on reorder mode**

Find (line ~29):

```ts
  async function refresh() { if (dragging) return; codes = await currentCodes(); }
```

Replace with:

```ts
  async function refresh() { if (reorderMode) return; codes = await currentCodes(); }
```

- [ ] **Step 4: Replace `toggleReorderMode` with enter/confirm/cancel + simplify drag handlers**

Find the block (lines ~71-97):

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

Replace with:

```ts
  function enterReorderMode() {
    reorderMode = true;
    selectMode = false;
    selected = new Set();
    dragIndex = null;
    orderSnapshot = codes.map((c) => c.id);
  }

  function onDragStart(i: number) { dragIndex = i; }

  function onDragOver(e: DragEvent, i: number) {
    e.preventDefault();
    if (dragIndex === null || dragIndex === i) return;
    codes = moveItem(codes, dragIndex, i);
    dragIndex = i;
  }

  function endDrag() { dragIndex = null; }

  async function confirmReorder() {
    // Persist while still in reorder mode so a tick-refresh can't reload
    // the stale order between exit and save.
    try { await reorderAccounts(codes.map((c) => c.id)); }
    catch (_) { /* ignore; refresh below restores the persisted order */ }
    reorderMode = false;
    dragIndex = null;
    await refresh();
  }

  async function cancelReorder() {
    // Nothing was persisted; restore the on-screen order, then let refresh
    // reload live codes (backend still holds the original order).
    codes = restoreOrder(codes, orderSnapshot);
    reorderMode = false;
    dragIndex = null;
    await refresh();
  }
```

- [ ] **Step 5: Replace the header reorder button with Confirm + Cancel**

Find (lines ~143-147):

```svelte
    {#if !selectMode}
      <button class="tool" class:tool-active={reorderMode} on:click={toggleReorderMode} title={reorderMode ? "Done reordering" : "Reorder accounts"} aria-label={reorderMode ? "Done reordering" : "Reorder accounts"}>
        {reorderMode ? "✓" : "⇅"}
      </button>
    {/if}
```

Replace with:

```svelte
    {#if reorderMode}
      <button class="tool tool-active" on:click={confirmReorder} title="Save order" aria-label="Save order">✓</button>
      <button class="tool" on:click={cancelReorder} title="Cancel reordering" aria-label="Cancel reordering">✕</button>
    {:else if !selectMode}
      <button class="tool" on:click={enterReorderMode} title="Reorder accounts" aria-label="Reorder accounts">⇅</button>
    {/if}
```

- [ ] **Step 6: Point the drag-row handlers at `endDrag`**

Find (lines ~176-177):

```svelte
        on:drop={persistOrder}
        on:dragend={persistOrder}
```

Replace with:

```svelte
        on:drop={endDrag}
        on:dragend={endDrag}
```

- [ ] **Step 7: Type-check the component**

Run: `npx svelte-check --tsconfig ./tsconfig.json`
Expected: 0 errors in `src/routes/Main.svelte` (no reference to the removed `dragging`/`toggleReorderMode`/`persistOrder`).

- [ ] **Step 8: Manual verification (launch + drag)**

Run: `npm run tauri dev` (single instance; ensure no other dev server is holding port 1420).
Verify all four:
1. Enter reorder mode (`⇅`) → header shows `✓` and `✕`.
2. Drag a row to a new position → it moves on screen.
3. Click `✕` → list returns to the original order; relaunch app → original order persisted (no save happened).
4. Re-enter, drag, click `✓` → new order shown; relaunch app → new order persisted.

- [ ] **Step 9: Commit**

```bash
git add src/routes/Main.svelte
git commit -m "feat(reorder): confirm/cancel with deferred save"
```

---

## Notes / Out of Scope

- **Pending bug-fix commits (separate from this feature):** the working tree already contains two unrelated fixes — `watch.ignored` in `vite.config.ts` (EBUSY watcher crash) and `dragDropEnabled: false` in `src-tauri/tauri.conf.json` (HTML5 drag on Windows). Commit those on their own before/after this feature so each diff stays isolated. The `dragDropEnabled` fix is a prerequisite for Step 8's drag to actually work on Windows.
- No backend or IPC changes; `reorder_accounts` and `Vault::reorder` are untouched and already covered by `cargo test`.

## Self-Review

- **Spec coverage:** Confirm (`✓`) persists once (Task 2 Step 4 `confirmReorder`, Step 5 button); Cancel (`✕`) reverts via snapshot (Task 2 Step 4 `cancelReorder` + Task 1 `restoreOrder`, Step 5 button); deferred save (drag handlers no longer persist — Step 4/6); refresh no longer clobbers (Step 3). All covered.
- **Placeholder scan:** none — every step has concrete code/commands.
- **Type consistency:** `moveItem`/`restoreOrder` signatures in Task 1 match their call sites in Task 2 (`moveItem(codes, dragIndex, i)`, `restoreOrder(codes, orderSnapshot)`). Removed symbols (`dragging`, `toggleReorderMode`, `persistOrder`) have no remaining references after Steps 2-6.
