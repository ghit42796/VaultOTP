# Account List Redesign + Change Master Password — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restyle the account list to modern bordered cards with a bottom linear countdown (replacing the SVG ring) and quieter copy feedback, and add a "Change master password" form to Settings for password-mode vaults — with labeled password fields throughout.

**Architecture:** Frontend-only. A new pure `barFraction` helper drives a linear timer; `AccountCard.svelte` is rewritten as a bordered card that owns its select/copy/timer behavior; `Main.svelte` passes select state into the card; `Settings.svelte` gains a Master-password section calling the existing `changePassword` IPC; `app.css` gets shared `.vo-label`/`.vo-group` classes applied here and in `Unlock.svelte`.

**Tech Stack:** Svelte 4 + TypeScript, existing CSS-custom-property design tokens; tests via `vitest`; no backend/crypto/IPC changes; no new dependencies.

Spec: `docs/superpowers/specs/2026-06-29-list-redesign-and-change-password-design.md`.

## Global Constraints

- **Frontend-only.** No Rust/IPC/crypto/data-format changes. `change_password`/`changePassword` already exist and are tested.
- **Tokens only.** Use existing CSS custom properties from `src/app.css` (`--surface`, `--border`, `--accent`, `--accent-weak`, `--danger`, `--success`, `--text`, `--text-muted`, `--ring-track`, `--radius`, `--radius-sm`, `--space-*`, `--font-mono`). No new palette entries; everything must work in light + dark.
- **No new dependencies. No database / no SQL.**
- **Change-password is password-mode vaults only** (`secMode === "password"`); composite/key-file vaults do not show it. Validation: new password length ≥ 8 and new === confirm (client-side); call `changePassword(currentPassword, undefined, newPassword, undefined)`.
- **No Svelte component-test harness exists** — `.svelte` changes are verified with `npx svelte-check --tsconfig ./tsconfig.json` (0 errors) + `npm run build` (succeeds) + manual run. Only pure TS (`display.ts`) gets vitest tests.
- **Frontend test command:** `npm test` (vitest run). Build: `npm run build`.
- **Commit style (reference only — execution may skip commits):** Conventional Commits; body ends with `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

---

## File Structure

- `src/lib/display.ts` — add pure `barFraction`; remove now-unused `ringCircumference`/`ringDashoffset` (Task 2).
- `src/lib/display.test.ts` — add `barFraction` test; remove the ring test (Task 2).
- `src/components/AccountCard.svelte` — full rewrite: bordered card, linear bottom timer, code-on-right, copy feedback, in-card select checkbox + `toggle` event, relocated hover delete.
- `src/routes/Main.svelte` — pass `selectMode`/`selected`/`on:toggle` to `AccountCard`; drop the sibling checkbox + its styles; polish empty-state copy.
- `src/app.css` — add shared `.vo-label` + `.vo-group`.
- `src/routes/Unlock.svelte` — wrap password/confirm/key-file fields in labeled `.vo-group`s.
- `src/components/Settings.svelte` — add the Master-password section (re-import `changePassword`, `passwordStrength`).

---

### Task 1: `barFraction` helper

**Files:**
- Modify: `src/lib/display.ts`
- Test: `src/lib/display.test.ts`

**Interfaces:**
- Produces: `barFraction(remaining: number, period: number): number` — clamped 0–1; `period <= 0` → 0.

- [ ] **Step 1: Write the failing test**

Add this `it` block inside the `describe("display helpers", …)` in `src/lib/display.test.ts`, and add `barFraction` to the import on line 2:

```ts
  it("barFraction: 1 when full, 0 when empty, clamps out-of-range", () => {
    expect(barFraction(30, 30)).toBe(1);
    expect(barFraction(0, 30)).toBe(0);
    expect(barFraction(15, 30)).toBe(0.5);
    expect(barFraction(40, 30)).toBe(1); // remaining > period → clamp to 1
    expect(barFraction(5, 0)).toBe(0);   // period 0 → empty
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/lib/display.test.ts`
Expected: FAIL — `barFraction is not a function` / import error.

- [ ] **Step 3: Write minimal implementation**

In `src/lib/display.ts`, add (e.g. just after `groupCode`):

```ts
/** Fraction of the period remaining, clamped to 0..1. period <= 0 -> 0. */
export function barFraction(remaining: number, period: number): number {
  if (period <= 0) return 0;
  return Math.max(0, Math.min(1, remaining / period));
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- src/lib/display.test.ts`
Expected: PASS (all display tests, including the new one). Do NOT remove the ring helpers yet — `AccountCard.svelte` still imports them until Task 2; removing them now breaks the build.

- [ ] **Step 5: Commit** — SKIP under no-git.

```bash
git add src/lib/display.ts src/lib/display.test.ts
git commit -m "feat(display): add barFraction helper for linear timer"
```

---

### Task 2: Rewrite `AccountCard.svelte` (bordered card + linear timer + select)

**Files:**
- Modify (full rewrite): `src/components/AccountCard.svelte`
- Modify: `src/lib/display.ts` (remove `ringCircumference`/`ringDashoffset`)
- Modify: `src/lib/display.test.ts` (remove the ring test)

**Interfaces:**
- Consumes: `barFraction` (Task 1), `initial`/`badgeColor`/`groupCode` (existing), `CodeView`, `loadSettings`.
- Produces: `AccountCard` props `item: CodeView`, `selectMode = false`, `selected = false`; events `remove` (detail: id) and `toggle` (detail: id).

- [ ] **Step 1: Replace the component file**

Overwrite `src/components/AccountCard.svelte` with:

```svelte
<script lang="ts">
  import type { CodeView } from "../lib/types";
  import { createEventDispatcher } from "svelte";
  import { loadSettings } from "../lib/settings";
  import { initial, badgeColor, groupCode, barFraction } from "../lib/display";

  export let item: CodeView;
  export let selectMode = false;
  export let selected = false;

  const dispatch = createEventDispatcher();
  const settings = loadSettings();
  let copied = false;

  const PERIOD = 30;
  $: fraction = barFraction(item.remaining, PERIOD);
  $: warn = item.remaining <= 5;

  async function activate() {
    if (selectMode) { dispatch("toggle", item.id); return; }
    await copy();
  }

  async function copy() {
    await navigator.clipboard.writeText(item.code);
    copied = true;
    setTimeout(() => (copied = false), 1200);
    const ms = settings.clipboardClearMs;
    if (ms > 0) {
      setTimeout(async () => {
        try {
          const current = await navigator.clipboard.readText();
          if (current === item.code) await navigator.clipboard.writeText("");
        } catch (_) { /* clipboard read may be blocked; ignore */ }
      }, ms);
    }
  }
</script>

<div class="card" class:selected class:copied>
  <button
    class="main"
    on:click={activate}
    aria-pressed={selectMode ? selected : undefined}
    title={selectMode ? "Toggle selection" : "Copy code"}
  >
    {#if selectMode}<span class="check" class:on={selected} aria-hidden="true"></span>{/if}
    <span class="badge" style="background:{badgeColor(item.issuer)}">{initial(item.issuer)}</span>
    <span class="mid">
      <span class="issuer">{item.issuer || "—"}</span>
      <span class="label">{item.label}</span>
    </span>
    <span class="right">
      <span class="code" class:warn>{groupCode(item.code)}</span>
      {#if copied}
        <span class="secs ok">Copied ✓</span>
      {:else}
        <span class="secs" class:warn>{item.remaining}s</span>
      {/if}
    </span>
  </button>

  {#if !selectMode}
    <button class="del" on:click|stopPropagation={() => dispatch("remove", item.id)} title="Delete" aria-label="Delete">🗑</button>
  {/if}

  <span class="bar"><i class:warn style="width:{fraction * 100}%"></i></span>
</div>

<style>
  .card {
    position: relative; background: var(--surface);
    border: 1px solid var(--border); border-radius: var(--radius);
    transition: border-color .12s, background .25s, box-shadow .12s;
  }
  .card:hover { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-weak); }
  .card.selected, .card.copied { border-color: var(--accent); background: var(--accent-weak); }

  .main {
    width: 100%; display: flex; align-items: center; gap: var(--space-3);
    padding: 11px 13px 14px; background: none; border: none; cursor: pointer;
    text-align: left; border-radius: var(--radius);
  }

  .check {
    width: 18px; height: 18px; border-radius: 6px; border: 2px solid var(--border);
    flex: 0 0 auto; position: relative;
  }
  .check.on { background: var(--accent); border-color: var(--accent); }
  .check.on::after {
    content: "✓"; color: #fff; font-size: 11px; position: absolute; inset: 0;
    display: flex; align-items: center; justify-content: center;
  }

  .badge {
    width: 34px; height: 34px; border-radius: 10px; flex: 0 0 auto;
    display: flex; align-items: center; justify-content: center;
    font-weight: 700; font-size: 15px; color: #fff;
  }
  .mid { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .issuer { font-weight: 600; color: var(--text); }
  .label { font-size: 11px; color: var(--text-muted);
           overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .right { display: flex; flex-direction: column; align-items: flex-end; gap: 1px; flex: 0 0 auto; }
  .code { font-family: var(--font-mono); font-size: 22px; font-weight: 600;
          letter-spacing: 4px; color: var(--accent); }
  .code.warn { color: var(--danger); }
  .secs { font-size: 10px; color: var(--text-muted); }
  .secs.warn { color: var(--danger); }
  .secs.ok { color: var(--success); font-weight: 600; }

  .bar { position: absolute; left: 13px; right: 13px; bottom: 6px; height: 3px;
         border-radius: 3px; background: var(--ring-track); overflow: hidden; }
  .bar > i { display: block; height: 100%; background: var(--accent); transition: width .3s linear; }
  .bar > i.warn { background: var(--danger); }

  .del { position: absolute; top: 6px; right: 8px; background: none; border: none;
         cursor: pointer; font-size: 13px; color: var(--text-muted);
         opacity: 0; transition: opacity .12s; }
  .card:hover .del { opacity: .7; }
</style>
```

- [ ] **Step 2: Remove the now-unused ring helpers**

In `src/lib/display.ts`, delete the `ringCircumference` and `ringDashoffset` functions (the block from `export function ringCircumference` through the end of `ringDashoffset`). In `src/lib/display.test.ts`, delete the `it("ringDashoffset: …")` block and remove `ringCircumference, ringDashoffset` from the import on line 2.

- [ ] **Step 3: Verify**

Run: `npm test` → all vitest pass (display tests no longer reference ring helpers).
Run: `npx svelte-check --tsconfig ./tsconfig.json` → 0 errors (no dangling imports of ring helpers; `AccountCard` compiles).
Run: `npm run build` → succeeds.
Manual (if a desktop session is available): `npm run tauri dev` — code on the right, a thin bar drains along the card bottom; click a card → it tints and shows "Copied ✓"; under 5s the code/seconds/bar turn red; hover shows the 🗑.

- [ ] **Step 4: Commit** — SKIP under no-git.

```bash
git add src/components/AccountCard.svelte src/lib/display.ts src/lib/display.test.ts
git commit -m "feat(ui): bordered-card AccountCard with linear timer + copy feedback"
```

---

### Task 3: Wire select state in `Main.svelte`

**Files:**
- Modify: `src/routes/Main.svelte`

**Interfaces:**
- Consumes: `AccountCard` props `selectMode`/`selected` + events `remove`/`toggle` (Task 2). `toggleSelect`/`selectMode`/`selected` already exist in `Main.svelte`.

- [ ] **Step 1: Replace the list block**

In `src/routes/Main.svelte`, replace the entire `<div class="list">…</div>` block (the one rendering `.card-row`) with:

```svelte
<div class="list">
  {#each codes as item (item.id)}
    <AccountCard
      {item}
      {selectMode}
      selected={selected.has(item.id)}
      on:remove={(e) => remove(e.detail)}
      on:toggle={(e) => toggleSelect(e.detail)}
    />
  {/each}
  {#if codes.length === 0}
    <div class="empty">
      <div class="empty-icon">🔐</div>
      <p class="empty-title">No accounts yet</p>
      <p class="empty-sub">Press ＋ to add your first one — scan a QR, paste a key, or import.</p>
    </div>
  {/if}
</div>
```

- [ ] **Step 2: Drop the now-unused styles, add empty-title**

In the `<style>` of `src/routes/Main.svelte`, delete the "Card row with selection checkbox" block (the `.card-row`, `.sel-check`, `.card-wrap` rules). Add an `.empty-title` rule near the `.empty` rules:

```css
  .empty-title { font-weight: 600; color: var(--text); margin: 2px 0; }
```

(Leave `.export-bar`/`.export-label`/`.export-msg`/`.export-warn` and `.empty`/`.empty-icon`/`.empty-sub` as-is.)

- [ ] **Step 3: Verify**

Run: `npx svelte-check --tsconfig ./tsconfig.json` → 0 errors.
Run: `npm run build` → succeeds.
Run: `npm test` → existing suite green.
Manual (if available): toggle select mode (☑) — the checkbox now sits inside each card; selected cards tint with an accent border; clicking a card toggles selection (no copy); export bar (QR PNGs / Text / Google) still works; empty state shows the new copy.

- [ ] **Step 4: Commit** — SKIP under no-git.

```bash
git add src/routes/Main.svelte
git commit -m "feat(ui): in-card selection + empty-state copy"
```

---

### Task 4: Shared `.vo-label`/`.vo-group` + labeled Unlock fields

**Files:**
- Modify: `src/app.css`
- Modify: `src/routes/Unlock.svelte`

**Interfaces:**
- Produces: global `.vo-label` (field caption) + `.vo-group` (label+control column) used by Task 5 and Unlock.

- [ ] **Step 1: Add the shared classes**

In `src/app.css`, append to the "shared form controls" block (after `.vo-err`):

```css
.vo-label { font-size: 11px; font-weight: 600; color: var(--text-muted); }
.vo-group { display: flex; flex-direction: column; gap: var(--space-1); width: 100%; }
```

- [ ] **Step 2: Wrap Unlock fields in labeled groups**

In `src/routes/Unlock.svelte`, replace the password input block:

```svelte
  {#if mode !== "keyfile"}
    <div class="vo-group">
      <label class="vo-label" for="vo-pw">Master password</label>
      <input id="vo-pw" class="field" type="password" bind:this={passwordInput} bind:value={password}
             placeholder="Master password"
             on:keydown={handleFirstKeydown} />
    </div>
  {/if}
```

Replace the confirm input block (keep the meter line before it):

```svelte
  {#if !exists && mode !== "keyfile"}
    <div class="meter" aria-hidden="true"><i style="width:{strength * 25}%"></i></div>
    <div class="vo-group">
      <label class="vo-label" for="vo-cpw">Confirm password</label>
      <input id="vo-cpw" class="field" type="password" bind:this={confirmInput} bind:value={confirmPassword}
             placeholder="Confirm password"
             on:keydown={(e) => e.key === "Enter" && submit()} />
    </div>
  {/if}
```

Replace the key-file picker block:

```svelte
  {#if mode !== "password"}
    <div class="vo-group">
      <label class="vo-label">Key file</label>
      <button class="vo-ghost" on:click={pickKeyfile}>{keyfilePath ? "Key file ✓" : "Select key file…"}</button>
    </div>
  {/if}
```

- [ ] **Step 3: Verify**

Run: `npx svelte-check --tsconfig ./tsconfig.json` → 0 errors.
Run: `npm run build` → succeeds.
Run: `npm test` → green.
Manual (if available): the create/unlock screen shows a small "Master password" / "Confirm password" / "Key file" caption above each control, sitting close to its input.

- [ ] **Step 4: Commit** — SKIP under no-git.

```bash
git add src/app.css src/routes/Unlock.svelte
git commit -m "feat(ui): shared field labels (.vo-label/.vo-group) + labeled Unlock fields"
```

---

### Task 5: Change-master-password section in `Settings.svelte`

**Files:**
- Modify: `src/components/Settings.svelte`

**Interfaces:**
- Consumes: `changePassword` from `src/lib/ipc.ts`, `passwordStrength` from `src/lib/display.ts`, `secMode` (already read via `refreshMode()`), `.vo-label`/`.vo-group` (Task 4).

- [ ] **Step 1: Extend imports + add handler/state**

In `src/components/Settings.svelte`, add `changePassword` to the `../lib/ipc` import (line 4), and add a new import for the strength helper:

```ts
  import { passwordStrength } from "../lib/display";
```

In the `<script>`, after the vault state block (after `doSaveAs`), add:

```ts
  let cpCurrent = "", cpNew = "", cpConfirm = "", cpStatus = "", cpError = "";
  $: cpStrength = passwordStrength(cpNew);

  async function doChangePassword() {
    cpError = ""; cpStatus = "";
    if (cpNew.length < 8) { cpError = "New password must be at least 8 characters"; return; }
    if (cpNew !== cpConfirm) { cpError = "New passwords do not match"; return; }
    try {
      await changePassword(cpCurrent, undefined, cpNew, undefined);
      cpStatus = "Password changed ✓";
      cpCurrent = ""; cpNew = ""; cpConfirm = "";
    } catch (e) { cpError = String(e); }
  }
```

- [ ] **Step 2: Add the section markup**

In the markup, insert this section after the Security `</section>` and its following `<div class="divider"></div>` (i.e., between the Security block and the Vault block). It renders only for password-mode vaults:

```svelte
    {#if secMode === "password"}
      <section class="setting">
        <h3>Master password</h3>
        <p class="hint">Re-enter your current password, then choose a new one.</p>
        <div class="vo-group">
          <label class="vo-label" for="cp-cur">Current password</label>
          <input id="cp-cur" class="vo-field" type="password" bind:value={cpCurrent} />
        </div>
        <div class="vo-group">
          <label class="vo-label" for="cp-new">New password</label>
          <input id="cp-new" class="vo-field" type="password" bind:value={cpNew} />
          <div class="meter" aria-hidden="true"><i style="width:{cpStrength * 25}%"></i></div>
        </div>
        <div class="vo-group">
          <label class="vo-label" for="cp-conf">Confirm new password</label>
          <input id="cp-conf" class="vo-field" type="password" bind:value={cpConfirm} />
        </div>
        <button class="vo-primary" on:click={doChangePassword}>Change password</button>
        <div aria-live="polite" aria-atomic="true">
          {#if cpError}<p class="vo-err">{cpError}</p>{/if}
          {#if cpStatus}<p class="ok">{cpStatus}</p>{/if}
        </div>
      </section>

      <div class="divider"></div>
    {/if}
```

- [ ] **Step 3: Add the meter style**

In the `<style>` of `src/components/Settings.svelte`, add (reusing the Unlock meter look):

```css
  .meter { width: 100%; height: 6px; border-radius: 6px; background: var(--surface-2); overflow: hidden; }
  .meter > i { display: block; height: 100%; background: var(--success); transition: width .15s; }
```

- [ ] **Step 4: Verify**

Run: `npx svelte-check --tsconfig ./tsconfig.json` → 0 errors.
Run: `npm run build` → succeeds.
Run: `npm test` → green.
Manual (if available): open Settings on a password-mode vault — a "Master password" section appears with three labeled fields, a strength meter under the new password, and a Change-password button; entering the correct current password + a valid new password (≥8, matching) shows "Password changed ✓" and the next unlock requires the new password. A mismatch / too-short shows an inline error with no IPC call. Open Settings on a composite/key-file vault — the section is absent.

- [ ] **Step 5: Commit** — SKIP under no-git.

```bash
git add src/components/Settings.svelte
git commit -m "feat(ui): change master password section (password-mode vaults)"
```

---

## Self-Review

**Spec coverage:**
- §4.1 bordered card layout / code-right / badge / hover → Task 2. ✓
- §4.1 bottom **linear timer** replacing the ring → Task 1 (`barFraction`) + Task 2 (bar, ring helpers removed). ✓
- §4.2 copy feedback (tint + "Copied ✓", no toast) → Task 2. ✓
- §4.3 in-card checkbox, selected styling, toggle-not-copy, export bar → Task 2 (card) + Task 3 (wiring/styles). ✓
- §4.4 `barFraction` + remove ring helpers + `<5s` warn → Tasks 1, 2. ✓
- §4.5 empty-state copy → Task 3. ✓
- §5 change-password (password-mode only, labeled fields, strength, validation, `changePassword`) → Task 5. ✓
- §6 `.vo-label`/`.vo-group` + Unlock labels → Task 4. ✓
- §7 theming via tokens; a11y (`aria-pressed`, `aria-label`, labels, `aria-live`) → Tasks 2–5. ✓
- §8 testing: `barFraction` unit test, ring test removed, svelte-check/build/manual for components → Tasks 1–5. ✓

**Placeholder scan:** none — every step has complete code or exact edits.

**Type/name consistency:** `barFraction(remaining, period)` defined in Task 1, consumed in Task 2 card; `AccountCard` props `item`/`selectMode`/`selected` and events `remove`/`toggle` defined in Task 2, consumed in Task 3; `.vo-label`/`.vo-group` defined in Task 4, used in Tasks 4–5; `changePassword(currentPassword, undefined, newPassword, undefined)` matches the existing IPC signature. Ordering keeps every task's build green (ring helpers removed only after the card stops importing them; `.vo-label` added before Settings uses it).
