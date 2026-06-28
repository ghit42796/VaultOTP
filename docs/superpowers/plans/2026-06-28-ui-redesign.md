# VaultOTP UI Visual Overhaul — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restyle the VaultOTP frontend into a cohesive, token-based, dark/light-themeable UI (issuer badge + circular countdown ring, theme that follows the OS with a manual toggle) without changing any backend, IPC, crypto, or behavior.

**Architecture:** A single `src/app.css` defines CSS custom-property design tokens (light values on `:root`, dark via `@media (prefers-color-scheme: dark)` and `:root[data-theme="dark"]`, forced light via `:root[data-theme="light"]`). Pure presentation/theme logic lives in small, unit-tested helpers (`src/lib/display.ts`, `src/lib/theme.ts`); Svelte components consume tokens + helpers. No new dependencies.

**Tech Stack:** Svelte 4 + TypeScript + Vite; Vitest (node env, no jsdom). No Rust changes.

## Global Constraints

- **Frontend only.** No changes to `src-tauri/**`, commands, IPC, crypto, vault format, or behavior. Secrets still never reach the UI (cards render `code` only).
- **No new dependencies.** Native CSS custom properties only (no Tailwind / UI lib). Keeps the lightweight ethos.
- **No `sed`** — use Edit/Write for all changes.
- **Git is user-directed** — do NOT run git/commit autonomously; each task ends with a verification step, not a commit. (The user commits when they choose.)
- **Theme:** preference is `system | light | dark`, default `system` (follows OS), persisted in `localStorage` key `vaultotp.theme`. `system` ⇒ no `data-theme` attribute.
- **Card:** issuer first-letter badge (stable hashed color) + circular SVG countdown ring (drains over `period`, shows remaining seconds, turns `--danger` when `remaining ≤ 5`); code shown grouped in `--font-mono`, `--accent`.
- **Window:** 420×600 (matches `tauri.conf.json`); design to that width.
- **Verification gates:** every task ends green on `npx svelte-check` (0 errors) and `npm run build`; helper tasks also on `npm test` (vitest). Confirmed mockups are the visual source of truth: `docs/superpowers/specs/2026-06-28-ui-redesign-mockups/{main-screen,unlock-settings,add-modal}.html`.
- TDD for pure helpers (write failing test → run red → implement → run green). Components are gated by svelte-check + build + the mockups.

## File structure

| File | Responsibility |
|------|----------------|
| `src/app.css` | **New** — design tokens + base/reset + light/dark/system palettes |
| `src/lib/theme.ts` | **New** — theme preference: normalize/attr (pure) + load/save/apply (DOM) |
| `src/lib/theme.test.ts` | **New** — unit tests for pure theme helpers |
| `src/lib/display.ts` | **New** — pure presentation helpers (initial, badgeColor, groupCode, ring geometry, passwordStrength) |
| `src/lib/display.test.ts` | **New** — unit tests for display helpers |
| `src/main.ts` | Import `app.css`; apply theme before mount |
| `src/App.svelte` | Drop ad-hoc styles |
| `src/components/AccountCard.svelte` | Badge + ring + grouped code + copy toast + hover delete |
| `src/routes/Main.svelte` | Header + list + empty-state restyle |
| `src/routes/Unlock.svelte` | Restyle + strength meter on create |
| `src/components/AddMenu.svelte` | Tabbed modal + scrim restyle |
| `src/components/AddManual.svelte` / `AddFromQr.svelte` / `ImportGoogle.svelte` | Token fields/buttons; QR dropzone; import checkbox list |
| `src/components/Settings.svelte` | Token restyle + Appearance theme toggle |

---

### Task 1: Design tokens + global stylesheet

**Files:**
- Create: `src/app.css`
- Modify: `src/main.ts` (import the stylesheet)
- Modify: `src/App.svelte` (remove ad-hoc heading styles; let tokens drive)

**Interfaces:**
- Produces: global CSS variables (`--bg`, `--surface`, `--surface-2`, `--text`, `--text-muted`, `--accent`, `--accent-weak`, `--accent-contrast`, `--danger`, `--success`, `--border`, `--ring-track`, `--ring-fill`, `--radius`, `--radius-sm`, `--space-1..4`, `--font-sans`, `--font-mono`) consumed by all later tasks; theming via `data-theme` on `<html>`.

- [ ] **Step 1: Create `src/app.css`**

```css
:root {
  --radius: 14px; --radius-sm: 10px;
  --space-1: 4px; --space-2: 8px; --space-3: 12px; --space-4: 16px;
  --font-sans: "Segoe UI", system-ui, -apple-system, sans-serif;
  --font-mono: "SF Mono", "Cascadia Code", Consolas, monospace;

  /* LIGHT (also the system-light default) */
  --bg: #f4f6f9; --surface: #ffffff; --surface-2: #eef1f5;
  --text: #1a212b; --text-muted: #5b6772;
  --accent: #2563eb; --accent-weak: rgba(37,99,235,.10); --accent-contrast: #ffffff;
  --danger: #d23b3b; --success: #1a7f37; --border: #e4e9ef;
  --ring-track: #e4e9ef; --ring-fill: var(--accent);
}

/* dark palette as reusable declarations */
@media (prefers-color-scheme: dark) {
  :root:not([data-theme]) {
    --bg: #0f1419; --surface: #1a212b; --surface-2: #232c38;
    --text: #e6edf3; --text-muted: #8b98a5;
    --accent: #4c9aff; --accent-weak: rgba(76,154,255,.14); --accent-contrast: #ffffff;
    --danger: #ff6b6b; --success: #3fb950; --border: #2a3441;
    --ring-track: #2a3441; --ring-fill: var(--accent);
  }
}
:root[data-theme="dark"] {
  --bg: #0f1419; --surface: #1a212b; --surface-2: #232c38;
  --text: #e6edf3; --text-muted: #8b98a5;
  --accent: #4c9aff; --accent-weak: rgba(76,154,255,.14); --accent-contrast: #ffffff;
  --danger: #ff6b6b; --success: #3fb950; --border: #2a3441;
  --ring-track: #2a3441; --ring-fill: var(--accent);
}
/* :root[data-theme="light"] uses the :root light defaults — no override needed */

* { box-sizing: border-box; }
html, body, #app { height: 100%; }
body {
  margin: 0; background: var(--bg); color: var(--text);
  font-family: var(--font-sans); font-size: 14px;
  -webkit-font-smoothing: antialiased;
}
button { font-family: inherit; }
::selection { background: var(--accent-weak); }
```

- [ ] **Step 2: Import the stylesheet in `src/main.ts`**

Add as the first import line of `src/main.ts`:
```ts
import "./app.css";
```
(Keep the rest of `main.ts` unchanged; the `applyTheme` call is added in Task 2.)

- [ ] **Step 3: Simplify `src/App.svelte`**

`App.svelte` currently routes Unlock/Main with a small `<style>`. Remove any color/background rules there (now provided by tokens/body); keep only the routing markup/logic. If `App.svelte` has no styles beyond layout, leave its logic intact and ensure no hardcoded colors remain.

- [ ] **Step 4: Verify build + types**

Run: `cd d:/Workspace/auth-totp-app && npx svelte-check 2>&1 | tail -1 && npm run build 2>&1 | tail -2`
Expected: svelte-check 0 errors; vite build succeeds.

- [ ] **Step 5: Done** (no commit — Global Constraints)

---

### Task 2: Theme preference logic (`theme.ts`) — TDD

**Files:**
- Create: `src/lib/theme.ts`
- Create: `src/lib/theme.test.ts`
- Modify: `src/main.ts` (apply theme before mount)

**Interfaces:**
- Produces: `type ThemePref = "system" | "light" | "dark"`; `normalizePref(raw: string | null): ThemePref`; `themeAttr(pref: ThemePref): "light" | "dark" | null`; `loadThemePref(): ThemePref`; `saveThemePref(pref: ThemePref): void`; `applyTheme(pref: ThemePref): void`. Consumed by `Settings.svelte` (Task 8) and `main.ts`.

- [ ] **Step 1: Write the failing tests**

`src/lib/theme.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { normalizePref, themeAttr } from "./theme";

describe("theme helpers", () => {
  it("normalizePref accepts valid values, defaults to system", () => {
    expect(normalizePref("light")).toBe("light");
    expect(normalizePref("dark")).toBe("dark");
    expect(normalizePref("system")).toBe("system");
    expect(normalizePref(null)).toBe("system");
    expect(normalizePref("bogus")).toBe("system");
  });
  it("themeAttr maps pref to the data-theme value (null for system)", () => {
    expect(themeAttr("system")).toBeNull();
    expect(themeAttr("light")).toBe("light");
    expect(themeAttr("dark")).toBe("dark");
  });
});
```

- [ ] **Step 2: Run tests — verify they fail**

Run: `cd d:/Workspace/auth-totp-app && npx vitest run src/lib/theme.test.ts 2>&1 | tail -8`
Expected: FAIL — `./theme` not found.

- [ ] **Step 3: Implement `src/lib/theme.ts`**

```ts
export type ThemePref = "system" | "light" | "dark";

const KEY = "vaultotp.theme";

/** Pure: coerce any stored/raw value to a valid ThemePref (default "system"). */
export function normalizePref(raw: string | null): ThemePref {
  return raw === "light" || raw === "dark" || raw === "system" ? raw : "system";
}

/** Pure: the `data-theme` attribute value for a pref; null means "remove it" (system). */
export function themeAttr(pref: ThemePref): "light" | "dark" | null {
  return pref === "system" ? null : pref;
}

export function loadThemePref(): ThemePref {
  try { return normalizePref(localStorage.getItem(KEY)); } catch { return "system"; }
}

export function saveThemePref(pref: ThemePref): void {
  try { localStorage.setItem(KEY, pref); } catch { /* storage may be unavailable */ }
}

/** Apply the preference to <html data-theme>. `system` removes the attribute. */
export function applyTheme(pref: ThemePref): void {
  const attr = themeAttr(pref);
  const root = document.documentElement;
  if (attr) root.setAttribute("data-theme", attr);
  else root.removeAttribute("data-theme");
}
```

- [ ] **Step 4: Run tests — verify they pass**

Run: `cd d:/Workspace/auth-totp-app && npx vitest run src/lib/theme.test.ts 2>&1 | tail -5`
Expected: PASS (2 tests).

- [ ] **Step 5: Apply theme at startup in `src/main.ts`**

Make `src/main.ts` read (order matters — css import first, theme applied before mount):
```ts
import "./app.css";
import { applyTheme, loadThemePref } from "./lib/theme";
import App from "./App.svelte";

applyTheme(loadThemePref());

const app = new App({ target: document.getElementById("app")! });
export default app;
```

- [ ] **Step 6: Verify**

Run: `cd d:/Workspace/auth-totp-app && npx svelte-check 2>&1 | tail -1 && npm run build 2>&1 | tail -2 && npm test 2>&1 | grep -E "Tests"`
Expected: svelte-check 0 errors; build OK; vitest all passing (now includes theme tests).

- [ ] **Step 7: Done** (no commit)

---

### Task 3: Pure presentation helpers (`display.ts`) — TDD

**Files:**
- Create: `src/lib/display.ts`
- Create: `src/lib/display.test.ts`

**Interfaces:**
- Produces: `initial(issuer: string): string`; `badgeColor(issuer: string): string`; `groupCode(code: string): string`; `ringCircumference(radius: number): number`; `ringDashoffset(remaining: number, period: number, radius: number): number`; `passwordStrength(pw: string): number` (0–4). Consumed by `AccountCard.svelte` (Task 4) and `Unlock.svelte` (Task 6).

- [ ] **Step 1: Write the failing tests**

`src/lib/display.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { initial, badgeColor, groupCode, ringCircumference, ringDashoffset, passwordStrength } from "./display";

describe("display helpers", () => {
  it("initial returns first uppercase letter, fallback for empty", () => {
    expect(initial("GitHub")).toBe("G");
    expect(initial("  google ")).toBe("G");
    expect(initial("")).toBe("•");
  });
  it("badgeColor is deterministic and within the palette", () => {
    expect(badgeColor("GitHub")).toBe(badgeColor("GitHub"));
    expect(badgeColor("GitHub")).toMatch(/^#[0-9a-f]{6}$/i);
  });
  it("groupCode splits 6 and 8 digit codes, passes others through", () => {
    expect(groupCode("482913")).toBe("482 913");
    expect(groupCode("01522445")).toBe("0152 2445");
    expect(groupCode("12345")).toBe("12345");
  });
  it("ringDashoffset: full at remaining=period, empty at 0", () => {
    const r = 17;
    const c = ringCircumference(r);
    expect(ringDashoffset(30, 30, r)).toBeCloseTo(0, 5);
    expect(ringDashoffset(0, 30, r)).toBeCloseTo(c, 5);
    expect(ringDashoffset(15, 30, r)).toBeCloseTo(c / 2, 5);
    expect(ringDashoffset(5, 0, r)).toBeCloseTo(c, 5); // period 0 → treat as empty
  });
  it("passwordStrength grows with length and variety (0..4)", () => {
    expect(passwordStrength("")).toBe(0);
    expect(passwordStrength("short")).toBe(0);
    expect(passwordStrength("abcdefgh")).toBe(1);
    expect(passwordStrength("Abcdefghijkl")).toBe(3);
    expect(passwordStrength("Abcdef1!ghijkl")).toBe(4);
  });
});
```

- [ ] **Step 2: Run tests — verify they fail**

Run: `cd d:/Workspace/auth-totp-app && npx vitest run src/lib/display.test.ts 2>&1 | tail -8`
Expected: FAIL — `./display` not found.

- [ ] **Step 3: Implement `src/lib/display.ts`**

```ts
const BADGE_PALETTE = [
  "#e05d44", "#d6883b", "#2f9e44", "#2563eb",
  "#7048e8", "#c2255c", "#0c8599", "#5c7cfa",
];

/** First letter of issuer, uppercased; "•" when empty. */
export function initial(issuer: string): string {
  const t = issuer.trim();
  return t ? t[0].toUpperCase() : "•";
}

/** Deterministic badge color from the issuer string. */
export function badgeColor(issuer: string): string {
  const s = issuer.trim() || "•";
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0;
  return BADGE_PALETTE[h % BADGE_PALETTE.length];
}

/** Group a TOTP code: 6 -> "NNN NNN", 8 -> "NNNN NNNN", else unchanged. */
export function groupCode(code: string): string {
  if (code.length === 6) return code.slice(0, 3) + " " + code.slice(3);
  if (code.length === 8) return code.slice(0, 4) + " " + code.slice(4);
  return code;
}

export function ringCircumference(radius: number): number {
  return 2 * Math.PI * radius;
}

/**
 * stroke-dashoffset for the progress ring: 0 when full (remaining = period),
 * full circumference when empty (remaining = 0). period <= 0 -> empty.
 */
export function ringDashoffset(remaining: number, period: number, radius: number): number {
  const c = ringCircumference(radius);
  const frac = period > 0 ? Math.max(0, Math.min(1, remaining / period)) : 0;
  return c * (1 - frac);
}

/** Heuristic password strength, 0..4 (length + character variety). */
export function passwordStrength(pw: string): number {
  let s = 0;
  if (pw.length >= 8) s++;
  if (pw.length >= 12) s++;
  if (/[a-z]/.test(pw) && /[A-Z]/.test(pw)) s++;
  if (/\d/.test(pw) && /[^A-Za-z0-9]/.test(pw)) s++;
  return Math.min(4, s);
}
```

- [ ] **Step 4: Run tests — verify they pass**

Run: `cd d:/Workspace/auth-totp-app && npx vitest run src/lib/display.test.ts 2>&1 | tail -5`
Expected: PASS (5 tests).

- [ ] **Step 5: Done** (no commit)

---

### Task 4: AccountCard — badge + countdown ring + copy toast

**Files:**
- Modify: `src/components/AccountCard.svelte` (full restyle; keep all existing logic/props/events)

**Interfaces:**
- Consumes: `display.ts` (`initial`, `badgeColor`, `groupCode`, `ringCircumference`, `ringDashoffset`); existing `CodeView` (`issuer`,`label`,`code`,`remaining`).
- **Ring period note (known limitation, frontend-only scope):** `CodeView` carries no `period`, so the ring uses `PERIOD = 30`. The **seconds number (`item.remaining`) is always exact** for any period; only the ring *arc* is normalized to 30s — a 60s account shows a full arc for its first 30s then drains. `ringDashoffset` clamps `remaining/period` to ≤1, so this degrades gracefully (no overflow). Making the arc exact for non-30s accounts would require adding a `period` field to `CodeView` (a small additive backend change) — **out of scope here**; revisit only if non-30s accounts matter. (Note: this corrects the design spec §6 wording "remaining / period" — period is not currently exposed.)
- Produces: same `remove` event (`item.id`) and copy behavior as before.

- [ ] **Step 1: Replace `src/components/AccountCard.svelte`**

```svelte
<script lang="ts">
  import type { CodeView } from "../lib/types";
  import { createEventDispatcher } from "svelte";
  import { loadSettings } from "../lib/settings";
  import { initial, badgeColor, groupCode, ringCircumference, ringDashoffset } from "../lib/display";

  export let item: CodeView;
  const dispatch = createEventDispatcher();
  const settings = loadSettings();
  let copied = false;

  const PERIOD = 30;
  const R = 17;
  const C = ringCircumference(R);
  $: offset = ringDashoffset(item.remaining, PERIOD, R);
  $: warn = item.remaining <= 5;

  async function copy() {
    await navigator.clipboard.writeText(item.code);
    copied = true;
    setTimeout(() => (copied = false), 1500);
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

<div class="card">
  <div class="badge" style="background:{badgeColor(item.issuer)}">{initial(item.issuer)}</div>

  <button class="main" on:click={copy} title="Copy code">
    <span class="issuer">{item.issuer || "—"}</span>
    <span class="label">{item.label}</span>
    <span class="code">{groupCode(item.code)}</span>
  </button>

  {#if copied}<span class="toast">Copied ✓</span>{/if}

  <div class="ring" title="{item.remaining}s remaining">
    <svg width="40" height="40" viewBox="0 0 40 40">
      <circle class="track" cx="20" cy="20" r={R} stroke-width="3.5" fill="none" />
      <circle class="fill" class:warn cx="20" cy="20" r={R} stroke-width="3.5"
        fill="none" stroke-linecap="round"
        stroke-dasharray={C} stroke-dashoffset={offset} transform="rotate(-90 20 20)" />
    </svg>
    <span class="num" class:warn>{item.remaining}</span>
  </div>

  <button class="del" on:click={() => dispatch("remove", item.id)} title="Delete" aria-label="Delete">🗑</button>
</div>

<style>
  .card {
    display: flex; align-items: center; gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: var(--surface); border: 1px solid var(--border);
    border-radius: var(--radius); position: relative;
  }
  .card:hover { border-color: var(--accent); }
  .badge {
    width: 40px; height: 40px; border-radius: 11px; flex: 0 0 auto;
    display: flex; align-items: center; justify-content: center;
    font-weight: 700; font-size: 17px; color: #fff;
  }
  .main {
    flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px;
    background: none; border: none; padding: 0; cursor: pointer; text-align: left;
  }
  .issuer { font-weight: 600; color: var(--text); }
  .label { font-size: 12px; color: var(--text-muted);
           overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .code { font-family: var(--font-mono); font-size: 22px; font-weight: 600;
          letter-spacing: 3px; color: var(--accent); margin-top: 2px; }
  .ring { position: relative; width: 40px; height: 40px; flex: 0 0 auto; }
  .track { stroke: var(--ring-track); }
  .fill { stroke: var(--ring-fill); transition: stroke-dashoffset .3s linear; }
  .fill.warn { stroke: var(--danger); }
  .num { position: absolute; inset: 0; display: flex; align-items: center;
         justify-content: center; font-size: 12px; font-weight: 600; color: var(--text-muted); }
  .num.warn { color: var(--danger); }
  .toast { position: absolute; right: 56px; top: 8px; font-size: 11px;
           color: var(--success); background: var(--accent-weak);
           padding: 2px 8px; border-radius: 20px; }
  .del { background: none; border: none; cursor: pointer; font-size: 14px;
         opacity: 0; transition: opacity .12s; color: var(--text-muted); }
  .card:hover .del { opacity: .8; }
</style>
```

- [ ] **Step 2: Verify**

Run: `cd d:/Workspace/auth-totp-app && npx svelte-check 2>&1 | tail -1 && npm run build 2>&1 | tail -2`
Expected: svelte-check 0 errors; build OK. (Logic helpers already unit-tested in Task 3.)

- [ ] **Step 3: Manual visual check (note for executor)**

Compare against [main-screen.html](../specs/2026-06-28-ui-redesign-mockups/main-screen.html): badge color per issuer, ring drains and shows seconds, red at ≤5s, grouped code, copy toast, hover delete. Runtime requires `npm run tauri dev` — if headless, state that this is deferred to manual verification.

- [ ] **Step 4: Done** (no commit)

---

### Task 5: Main screen — header, list, empty state

**Files:**
- Modify: `src/routes/Main.svelte` (restyle only; keep all script logic, events, idle/tick wiring)

**Interfaces:**
- Consumes: existing `AccountCard`, `AddMenu`, `Settings`, ipc; no new interfaces.

- [ ] **Step 1: Replace the markup + `<style>` in `src/routes/Main.svelte`**

Keep the entire `<script>` block unchanged. Replace the markup (header/list/empty) and `<style>` with:
```svelte
<header>
  <div class="brand"><span class="logo">🔐</span> VaultOTP</div>
  <div class="tools">
    <button class="tool" on:click={() => (showAdd = true)} title="Add account" aria-label="Add account">＋</button>
    <button class="tool" on:click={doLock} title="Lock now" aria-label="Lock now">🔒</button>
    <button class="tool" on:click={() => (showSettings = true)} title="Settings" aria-label="Settings">⚙</button>
  </div>
</header>

<div class="list">
  {#each codes as item (item.id)}
    <AccountCard {item} on:remove={(e) => remove(e.detail)} />
  {/each}
  {#if codes.length === 0}
    <div class="empty">
      <div class="empty-icon">🔐</div>
      <p>No accounts yet.</p>
      <p class="empty-sub">Press ＋ to add your first one.</p>
    </div>
  {/if}
</div>

{#if showAdd}
  <AddMenu on:close={() => { showAdd = false; refresh(); }} />
{/if}
{#if showSettings}
  <Settings on:close={() => (showSettings = false)} on:changed={refresh} />
{/if}

<style>
  header {
    display: flex; align-items: center; justify-content: space-between;
    padding: var(--space-4) 18px; border-bottom: 1px solid var(--border);
  }
  .brand { display: flex; align-items: center; gap: var(--space-2); font-weight: 700; font-size: 16px; }
  .logo { font-size: 18px; }
  .tools { display: flex; gap: 6px; }
  .tool {
    width: 34px; height: 34px; border-radius: 9px; border: none; cursor: pointer;
    background: var(--surface-2); color: var(--text); font-size: 16px;
    display: flex; align-items: center; justify-content: center;
  }
  .tool:hover { background: var(--accent-weak); color: var(--accent); }
  .list { overflow-y: auto; padding: var(--space-2); display: flex; flex-direction: column; gap: var(--space-2); }
  .empty { text-align: center; color: var(--text-muted); padding: 48px 24px; }
  .empty-icon { font-size: 40px; opacity: .5; margin-bottom: var(--space-2); }
  .empty p { margin: 2px 0; }
  .empty-sub { font-size: 12px; }
</style>
```

- [ ] **Step 2: Verify**

Run: `cd d:/Workspace/auth-totp-app && npx svelte-check 2>&1 | tail -1 && npm run build 2>&1 | tail -2`
Expected: svelte-check 0 errors; build OK.

- [ ] **Step 3: Done** (no commit)

---

### Task 6: Unlock / create — restyle + password-strength meter

**Files:**
- Modify: `src/routes/Unlock.svelte` (restyle; add strength meter on the create path using `display.passwordStrength`; keep all logic/validation/events)

**Interfaces:**
- Consumes: `display.passwordStrength`; existing `vaultExists`/`createVault`/`unlock` ipc and existing component state (`exists`, `password`, `confirmPassword`, `error`).

- [ ] **Step 1: Add the import + reactive strength in the `<script>`**

In `src/routes/Unlock.svelte`, add to the imports:
```ts
  import { passwordStrength } from "../lib/display";
```
And a reactive value (place after the existing `let` declarations):
```ts
  $: strength = passwordStrength(password);
```
(Do not change any existing logic — `submit()`, validation, dispatch, Enter handling stay as-is.)

- [ ] **Step 2: Restyle markup + add the meter (create path only)**

Replace the template markup and `<style>` (keep the `<script>` logic) with:
```svelte
<div class="unlock">
  <div class="lock">🔐</div>
  <h2>{exists ? "Unlock VaultOTP" : "Create a master password"}</h2>
  <p class="sub">{exists ? "Enter your master password" : "This encrypts your vault. There is no recovery if you forget it."}</p>

  <input class="field" type="password" bind:this={passwordInput} bind:value={password}
         placeholder="Master password"
         on:keydown={(e) => e.key === "Enter" && handleFirstKeydown(e)} />

  {#if !exists}
    <div class="meter" aria-hidden="true"><i style="width:{strength * 25}%"></i></div>
    <input class="field" type="password" bind:value={confirmPassword}
           placeholder="Confirm password"
           on:keydown={(e) => e.key === "Enter" && submit()} />
  {/if}

  {#if error}<p class="err">{error}</p>{/if}
  <button class="primary" on:click={submit} disabled={busy}>{exists ? "Unlock" : "Create vault"}</button>
</div>

<style>
  .unlock { display: flex; flex-direction: column; align-items: center; justify-content: center;
            gap: var(--space-3); height: 100%; padding: 40px 36px; }
  .lock { font-size: 42px; }
  h2 { margin: 0; font-size: 20px; color: var(--text); }
  .sub { margin: 0; color: var(--text-muted); font-size: 13px; text-align: center; }
  .field { width: 100%; padding: 12px 14px; border-radius: var(--radius-sm);
           border: 1px solid var(--border); background: var(--surface); color: var(--text); font-size: 14px; }
  .field::placeholder { color: var(--text-muted); }
  .field:focus { outline: none; border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-weak); }
  .meter { width: 100%; height: 6px; border-radius: 6px; background: var(--surface-2); overflow: hidden; }
  .meter > i { display: block; height: 100%; background: var(--success); transition: width .15s; }
  .err { color: var(--danger); font-size: 13px; margin: 0; }
  .primary { width: 100%; padding: 12px; border: none; border-radius: var(--radius-sm);
             background: var(--accent); color: var(--accent-contrast); font-weight: 600; font-size: 14px; cursor: pointer; }
  .primary:disabled { opacity: .6; cursor: default; }
</style>
```
> Note: this assumes the existing `<script>` already has `passwordInput`, `busy`, `handleFirstKeydown`, `submit` (from the current Unlock implementation). If the helper name differs, keep the existing handler — only the classes/markup and the meter are new.

- [ ] **Step 3: Verify**

Run: `cd d:/Workspace/auth-totp-app && npx svelte-check 2>&1 | tail -1 && npm run build 2>&1 | tail -2`
Expected: svelte-check 0 errors; build OK.

- [ ] **Step 4: Done** (no commit)

---

### Task 7: Add / Import modal — tabbed restyle

**Files:**
- Modify: `src/components/AddMenu.svelte` (tabs + scrim)
- Modify: `src/components/AddManual.svelte` (token fields/buttons)
- Modify: `src/components/AddFromQr.svelte` (dropzone + buttons)
- Modify: `src/components/ImportGoogle.svelte` (token fields + checkbox preview list)

**Interfaces:**
- Consumes: existing component logic/events (unchanged). Only markup classes + `<style>` change.

- [ ] **Step 1: Restyle `AddMenu.svelte` (keep its `<script>` logic)**

Replace markup + `<style>`:
```svelte
<div class="scrim" role="presentation" on:click={() => dispatch("close")}>
  <div class="modal" role="dialog" aria-modal="true" aria-label="Add account" on:click|stopPropagation>
    <div class="tabs">
      <button class:active={tab === "manual"} on:click={() => (tab = "manual")}>Manual</button>
      <button class:active={tab === "qr"} on:click={() => (tab = "qr")}>QR</button>
      <button class:active={tab === "import"} on:click={() => (tab = "import")}>Import</button>
    </div>
    {#if tab === "manual"}<AddManual on:added={done} />{/if}
    {#if tab === "qr"}<AddFromQr on:added={done} />{/if}
    {#if tab === "import"}<ImportGoogle on:added={done} />{/if}
    <button class="cancel" on:click={() => dispatch("close")}>Cancel</button>
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; background: rgba(0,0,0,.5);
           display: flex; align-items: center; justify-content: center; }
  .modal { width: 340px; background: var(--surface); border: 1px solid var(--border);
           border-radius: var(--radius); padding: var(--space-4);
           display: flex; flex-direction: column; gap: var(--space-3);
           box-shadow: 0 24px 60px rgba(0,0,0,.4); }
  .tabs { display: flex; background: var(--surface-2); border-radius: 10px; padding: 3px; gap: 3px; }
  .tabs button { flex: 1; padding: 8px; border: none; border-radius: 8px; background: none;
                 color: var(--text-muted); font-size: 13px; cursor: pointer; }
  .tabs button.active { background: var(--accent); color: var(--accent-contrast); font-weight: 600; }
  .cancel { background: none; border: none; color: var(--text-muted); font-size: 13px; cursor: pointer; }
</style>
```

- [ ] **Step 2: Add a shared field/button style to `app.css`** (DRY — used by the three add panels)

Append to `src/app.css`:
```css
/* shared form controls for modal panels */
.vo-field { width: 100%; padding: 11px 13px; border-radius: var(--radius-sm);
  border: 1px solid var(--border); background: var(--bg); color: var(--text); font-size: 14px; }
.vo-field::placeholder { color: var(--text-muted); }
.vo-field:focus { outline: none; border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-weak); }
.vo-primary { padding: 11px; border: none; border-radius: var(--radius-sm);
  background: var(--accent); color: var(--accent-contrast); font-weight: 600; font-size: 14px; cursor: pointer; }
.vo-primary:disabled { opacity: .6; cursor: default; }
.vo-ghost { padding: 11px; border: 1px solid var(--border); background: none; color: var(--text);
  border-radius: var(--radius-sm); cursor: pointer; font-size: 13px; }
.vo-form { display: flex; flex-direction: column; gap: var(--space-3); }
.vo-err { color: var(--danger); font-size: 13px; margin: 0; }
```
> These are global utility classes (intentional, small, prefixed `vo-`) so the three add panels share one definition. Svelte scoped styles can't cross components, so global is the DRY choice here.

- [ ] **Step 3: Restyle `AddManual.svelte`** (keep `<script>`; apply classes)

Set the form wrapper to `class="vo-form"`, inputs to `class="vo-field"` (secret input also `style="font-family:var(--font-mono)"`), the Add button to `class="vo-primary"`, and any error `<p>` to `class="vo-err"`. Remove the old local `<style>` color rules (keep only layout not covered by the utilities).

- [ ] **Step 4: Restyle `AddFromQr.svelte`** (keep `<script>`)

Wrap in `class="vo-form"`; "Choose image…" button `class="vo-ghost"`; add a dropzone element above it:
```svelte
<div class="dropzone">🖼️ Choose a QR image file<br /><span>PNG / JPG containing an otpauth:// code</span></div>
```
with a local style:
```css
.dropzone { border: 1.5px dashed var(--border); border-radius: var(--radius-sm);
  padding: 22px; text-align: center; color: var(--text-muted); font-size: 13px; }
.dropzone span { font-size: 11px; }
```
Status/error text uses `class="vo-err"` (or a muted `<p>`). Keep all existing button handlers.

- [ ] **Step 5: Restyle `ImportGoogle.svelte`** (keep `<script>`)

Wrap in `class="vo-form"`; "Choose export QR image…" `class="vo-ghost"`; the URI input `class="vo-field"`; the import button `class="vo-primary"`. Render the preview list with checkbox rows:
```svelte
<div class="preview">
  {#each preview as p, i}
    <label class="prow">
      <input type="checkbox" bind:checked={selected[i]} />
      <span class="pi">{p.issuer || "—"}</span>
      <span class="pl">{p.label}</span>
    </label>
  {/each}
</div>
```
Local style:
```css
.preview { display: flex; flex-direction: column; gap: 6px; max-height: 220px; overflow: auto; }
.prow { display: flex; align-items: center; gap: 10px; padding: 9px 10px;
  background: var(--bg); border: 1px solid var(--border); border-radius: 10px; cursor: pointer; }
.pi { font-size: 13px; font-weight: 600; color: var(--text); }
.pl { font-size: 11px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
```
Keep all existing import logic/handlers (`loadFromFile`, `loadPreview`, `doImport`, `selected`, `preview`, `uri`).

- [ ] **Step 6: Verify**

Run: `cd d:/Workspace/auth-totp-app && npx svelte-check 2>&1 | tail -1 && npm run build 2>&1 | tail -2`
Expected: svelte-check 0 errors; build OK. Cross-check against [add-modal.html](../specs/2026-06-28-ui-redesign-mockups/add-modal.html).

- [ ] **Step 7: Done** (no commit)

---

### Task 8: Settings — restyle + Appearance theme toggle

**Files:**
- Modify: `src/components/Settings.svelte` (token restyle; add Appearance segmented control wired to `theme.ts`)

**Interfaces:**
- Consumes: `theme.ts` (`loadThemePref`, `saveThemePref`, `applyTheme`, `ThemePref`); existing settings logic (`s`, `persist`, export/import handlers) unchanged.

- [ ] **Step 1: Add theme state to the `<script>`**

In `src/components/Settings.svelte` add:
```ts
  import { loadThemePref, saveThemePref, applyTheme, type ThemePref } from "../lib/theme";
  let theme: ThemePref = loadThemePref();
  function setTheme(p: ThemePref) { theme = p; saveThemePref(p); applyTheme(p); }
```
Keep all existing settings/backup logic.

- [ ] **Step 2: Add the Appearance section + restyle (keep other sections’ logic)**

Add as the first section inside the settings sheet, and restyle the sheet with tokens:
```svelte
  <section class="setting">
    <span class="k">Appearance</span>
    <span class="d">Theme follows your OS by default.</span>
    <div class="seg">
      <button class:active={theme === "system"} on:click={() => setTheme("system")}>System</button>
      <button class:active={theme === "light"} on:click={() => setTheme("light")}>Light</button>
      <button class:active={theme === "dark"} on:click={() => setTheme("dark")}>Dark</button>
    </div>
  </section>
```
Sheet/section styles:
```css
  .seg { display: flex; background: var(--surface-2); border-radius: 10px; padding: 3px; gap: 3px; }
  .seg button { flex: 1; padding: 7px; border: none; border-radius: 8px; background: none;
                color: var(--text-muted); font-size: 13px; cursor: pointer; }
  .seg button.active { background: var(--accent); color: var(--accent-contrast); font-weight: 600; }
  .setting { display: flex; flex-direction: column; gap: var(--space-2); }
  .setting .k { font-size: 13px; font-weight: 600; color: var(--text); }
  .setting .d { font-size: 12px; color: var(--text-muted); }
```
Restyle the surrounding overlay/modal and existing inputs/buttons to tokens (reuse `vo-field`/`vo-ghost`/`vo-primary` where applicable), keeping all existing handlers and the existing auto-lock/clipboard/backup controls.

- [ ] **Step 3: Verify**

Run: `cd d:/Workspace/auth-totp-app && npx svelte-check 2>&1 | tail -1 && npm run build 2>&1 | tail -2`
Expected: svelte-check 0 errors; build OK. Cross-check against [unlock-settings.html](../specs/2026-06-28-ui-redesign-mockups/unlock-settings.html).

- [ ] **Step 4: Manual check (note for executor)**

Via `npm run tauri dev`: toggling System/Light/Dark changes the theme live and persists across restart (localStorage `vaultotp.theme`).

- [ ] **Step 5: Done** (no commit)

---

### Task 9: Final verification gate

**Files:** none modified (verification only).

- [ ] **Step 1: Frontend suites + build**

Run: `cd d:/Workspace/auth-totp-app && npm test 2>&1 | grep -E "Test Files|Tests" && npx svelte-check 2>&1 | tail -1 && npm run build 2>&1 | tail -2`
Expected: vitest all pass (includes the new `theme.test.ts` + `display.test.ts`); svelte-check 0 errors; vite build succeeds.

- [ ] **Step 2: Confirm no backend touched**

Run: `cd d:/Workspace/auth-totp-app && git status -s src-tauri/ ; echo "exit=$?"`
Expected: no changes under `src-tauri/` (UI work is frontend-only). (Informational; no commit.)

- [ ] **Step 3: Confirm no hardcoded colors leaked into components**

Run: `grep -rnE "#[0-9a-fA-F]{6}|rgba?\(" src/components src/routes src/App.svelte | grep -v "var(--" || echo "none — all colors via tokens"`
Expected: only the badge palette is allowed to be literal — and that lives in `src/lib/display.ts` (not in the grep paths). Any other literal color in components/routes should be moved to a token. (The brand badge colors are data, not theme, so they correctly stay in `display.ts`.)

- [ ] **Step 4: Manual visual pass (note for executor)**

`npm run tauri dev`: verify against the three mockups — dark/light correctness, badge+ring, red ≤5s, copy toast, theme toggle persistence, focus glows, modal tabs. Headless executors record this as deferred-to-user manual verification.

- [ ] **Step 5: Done** (no commit)

---

## Notes for the implementer
- **Frontend-only, no deps, no Rust.** If any task tempts a `src-tauri/` change, stop — it's out of scope.
- **Tokens are the contract.** Components must reference `var(--…)`; the only literal colors allowed are the badge palette in `display.ts` (issuer-derived data, not theme).
- **Keep all existing `<script>` logic** in restyled components — these tasks change markup classes + `<style>` (+ the two new helper imports), never IPC/handlers.
- **Pure helpers are the TDD surface** (`theme.ts` normalize/attr, `display.ts` all functions). DOM/visual is verified by svelte-check + build + the committed mockups (manual at runtime).
- **a11y:** interactive non-buttons (scrim, clickable card) carry `role`/`aria`/keyboard affordances to keep svelte-check warnings at 0, matching the existing pattern.
