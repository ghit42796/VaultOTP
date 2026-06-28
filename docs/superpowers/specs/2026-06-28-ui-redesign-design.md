# VaultOTP UI Visual Overhaul — Design Spec

- **Date:** 2026-06-28
- **Status:** Approved design, pending implementation plan
- **Scope:** Frontend visual layer only (Svelte/TS). **No backend, IPC, crypto, or behavior changes.**

## 1. Goal & scope

Raise VaultOTP's visual quality from the current bare, light-only, inline-styled UI to a
cohesive, themeable design — without changing any functionality. Every screen is restyled
through a shared design-token system; a circular countdown ring + issuer badge replace the
plain "30s" text; a dark/light theme that follows the OS (with manual override) is added.

### In scope
- A single global stylesheet of **CSS custom-property design tokens** (color/type/spacing/radius).
- **Dual theme** (dark + light), default **follows OS** (`prefers-color-scheme`), with a manual
  **System / Light / Dark** toggle in Settings, persisted locally.
- Restyle of all screens: app shell, Unlock/create, Main + **AccountCard (badge + countdown ring)**,
  Add/Import modal (tabbed), Settings sheet, empty state, copy feedback.

### Out of scope (YAGNI — not in this work)
- New features: account search/filter, drag-reorder, account editing, icons-by-domain fetching.
- Animations beyond simple CSS transitions (hover, ring tick, toast fade).
- Internationalization, font bundling (use system font stacks).
- Any change to commands, vault, crypto, or the sans-IO architecture.

## 2. Confirmed decisions (from visual brainstorming)

| Decision | Choice |
|----------|--------|
| Optimization scope | Full visual overhaul |
| Theme | Dual (dark+light), default follows OS, manual System/Light/Dark toggle |
| Styling approach | **Native CSS custom properties** (no Tailwind / no new deps — keeps the lightweight ethos) |
| Account card | **Issuer first-letter badge + circular countdown ring** (Authy/2FAS-style) |

**Confirmed visual mockups (high-fidelity HTML, open in a browser):**
- [main-screen.html](2026-06-28-ui-redesign-mockups/main-screen.html) — Main screen, dark + light
- [unlock-settings.html](2026-06-28-ui-redesign-mockups/unlock-settings.html) — Unlock, create-vault, Settings + theme toggle
- [add-modal.html](2026-06-28-ui-redesign-mockups/add-modal.html) — Add/Import modal (Manual / QR / Import tabs)

## 3. Theming architecture

**New file `src/app.css`** (imported once in `src/main.ts`) defines all tokens and base styles.
Components stop hardcoding colors and use `var(--…)`.

```css
:root {                 /* shared, non-color tokens */
  --radius:14px; --radius-sm:10px;
  --space-1:4px; --space-2:8px; --space-3:12px; --space-4:16px;
  --font-sans:"Segoe UI",system-ui,-apple-system,sans-serif;
  --font-mono:"SF Mono","Cascadia Code",Consolas,monospace;
}
```

Theme is selected by a `data-theme` attribute on `<html>`:
- **`system`** (default): no `data-theme` attribute → a `@media (prefers-color-scheme: dark)`
  block supplies dark values, otherwise the `:root` light values apply.
- **`light`** / **`dark`**: `:root[data-theme="light"]` / `:root[data-theme="dark"]` force the
  palette regardless of OS.

```css
:root, :root[data-theme="light"] {            /* LIGHT (also the system-light default) */
  --bg:#f4f6f9; --surface:#ffffff; --surface-2:#eef1f5;
  --text:#1a212b; --text-muted:#5b6772;
  --accent:#2563eb; --accent-weak:rgba(37,99,235,.10); --accent-contrast:#ffffff;
  --danger:#d23b3b; --success:#1a7f37; --border:#e4e9ef;
  --ring-track:#e4e9ef; --ring-fill:var(--accent);
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme]) {                   /* system + OS dark */
    --bg:#0f1419; --surface:#1a212b; --surface-2:#232c38;
    --text:#e6edf3; --text-muted:#8b98a5;
    --accent:#4c9aff; --accent-weak:rgba(76,154,255,.14); --accent-contrast:#ffffff;
    --danger:#ff6b6b; --success:#3fb950; --border:#2a3441;
    --ring-track:#2a3441; --ring-fill:var(--accent);
  }
}
:root[data-theme="dark"] { /* same dark values as above (forced) */ }
```

**New file `src/lib/theme.ts`:**
- `type ThemePref = "system" | "light" | "dark"`.
- `loadThemePref()` / `saveThemePref(p)` — persisted in `localStorage` (key `vaultotp.theme`).
- `applyTheme(p)` — `system` → remove the `data-theme` attribute; otherwise set it.
- Called once at startup in `main.ts` (before mounting), and on change from Settings.

> The existing `src/lib/settings.ts` (idle/clipboard) stays as-is; theme uses its own key to
> avoid coupling. No backend involvement — theme is a pure frontend preference.

## 4. Per-screen design

### 4.1 App shell + header
Window is 420×600 (matches `tauri.conf.json`). Body uses `--bg`/`--text`/`--font-sans`.
Header: `🔐 VaultOTP` left; right tool buttons `＋` (add) `🔒` (lock) `⚙` (settings) as 34×34
rounded buttons (`--surface-2`, hover → `--accent-weak`/`--accent`).

### 4.2 Main screen + AccountCard  → [main-screen.html](2026-06-28-ui-redesign-mockups/main-screen.html)
```
┌──────────────────────────────────────────────┐
│ 🔐 VaultOTP                      ＋  🔒  ⚙     │
├──────────────────────────────────────────────┤
│ ╭───╮  GitHub                         ╭────╮  │
│ │ G │  alice@example.com              │ 23 │  │  ← ring drains over 30s
│ ╰───╯  482 913                        ╰────╯  │     (accent), shows seconds
│ ╭───╮  Google                         ╭────╮  │
│ │ G │  you@gmail.com    [Copied ✓]    │ 12 │  │  ← tap card = copy → toast
│ ╰───╯  730 118                        ╰────╯  │
│ ╭───╮  AWS                            ╭────╮  │
│ │ A │  root · 1234-5678-9012          │  4 │  │  ← ≤5s: ring + number RED
│ ╰───╯  015 224                        ╰────╯  │
└──────────────────────────────────────────────┘
```
- **Card** = `--surface` panel, `--border`, `--radius`; hover → border `--accent`.
- **Badge** = 40×40 rounded square, issuer initial (uppercase), background = a stable color
  derived from the issuer string (hash → fixed palette); text white. Empty issuer → "•".
- **Code** = `--font-mono`, grouped `NNN NNN` (or `NNNN NNNN` for 8-digit), `--accent`, ~24px.
- **Countdown ring** = 40×40 inline SVG: track circle `--ring-track`, progress circle
  `--ring-fill` using `stroke-dasharray`/`stroke-dashoffset` = `remaining / period`; number
  centered. At `remaining ≤ 5`, ring + number switch to `--danger`.
- **Copy**: clicking the card copies the code and shows a transient `Copied ✓` pill
  (`--success`); clipboard auto-clear behavior unchanged. **Delete** revealed on hover.
- **Empty state**: centered muted text + a hint to press `＋`.

### 4.3 Unlock / create-vault  → [unlock-settings.html](2026-06-28-ui-redesign-mockups/unlock-settings.html)
Centered column: 🔐, title, helper text, password field(s), primary button.
- **Unlock**: one password field + "Unlock"; the opaque error renders in `--danger`.
- **Create (first run)**: password + **strength meter** (`--success` fill) + confirm field +
  "Create vault"; helper line warns there is no recovery.
- Inputs use `--surface`/`--border`; focus → `--accent` border + `--accent-weak` glow ring.

### 4.4 Add / Import modal  → [add-modal.html](2026-06-28-ui-redesign-mockups/add-modal.html)
Centered modal over a dimmed scrim; segmented **tab bar** `Manual · QR · Import` (active tab
= `--accent`).
- **Manual**: issuer / label / Base32-secret (mono) fields → "Add" + "Cancel".
- **QR**: dashed dropzone + "Choose image…" (file picker → decode).
- **Import**: "Choose export QR image…" or paste `otpauth-migration://` → checkbox preview
  list → "Import selected (N)".

### 4.5 Settings sheet (incl. theme toggle)  → [unlock-settings.html](2026-06-28-ui-redesign-mockups/unlock-settings.html)
Bottom sheet (`--surface`, rounded top). Sections:
1. **Appearance** — `System / Light / Dark` segmented control (the new theme toggle).
2. **Auto-lock after idle** — minutes stepper (existing setting).
3. **Clear clipboard after copy** — seconds stepper (existing setting).
4. **Encrypted backup** — password field + Export… / Import… (existing).

## 5. Component / file map

| File | Change |
|------|--------|
| `src/app.css` | **New** — tokens + base/reset + light/dark/system palettes |
| `src/lib/theme.ts` | **New** — ThemePref load/save/apply |
| `src/main.ts` | Import `app.css`; `applyTheme(loadThemePref())` before mount |
| `src/App.svelte` | Drop ad-hoc styles; rely on tokens/body |
| `src/routes/Main.svelte` | Restyle header + list + empty state with tokens |
| `src/components/AccountCard.svelte` | Badge + SVG ring + grouped code + copy toast + hover delete |
| `src/routes/Unlock.svelte` | Restyle; add password-strength meter on create |
| `src/components/AddMenu.svelte` | Restyle tabbed modal + scrim |
| `src/components/AddManual.svelte` / `AddFromQr.svelte` / `ImportGoogle.svelte` | Token-based fields/buttons; QR dropzone; import checkbox list |
| `src/components/Settings.svelte` | Token restyle + **Appearance** theme segmented control |

A small shared style surface (tokens in `app.css`) means most component `<style>` blocks shrink
to layout-only rules referencing `var(--…)`.

## 6. Behavior preservation (must not regress)
- **No backend/IPC/crypto/logic change.** Same Tauri commands, same events, same vault format.
- Secrets still never reach the UI (cards render `code` only; no `secret`).
- All existing flows intact: unlock/create, add (manual/QR/import), copy + clipboard auto-clear,
  manual/idle/OS auto-lock, encrypted backup, opaque error string.
- The countdown ring is driven by the existing `CodeView.remaining` / `period`; no new IPC.

## 7. Testing & verification
- `npx svelte-check` → 0 errors (a11y: give interactive elements roles/labels; keep warnings at 0
  where practical).
- `npm run build` (vite) succeeds; `npm test` (vitest) stays green (existing `ipc` test untouched).
- Manual visual check via `npm run tauri dev`: dark/light correctness, ring depletion + red ≤5s,
  copy toast, theme toggle (System/Light/Dark) persists across restart, focus states.
- No Rust changes → `cargo test` unaffected (still 43/0).

## 8. Accessibility notes
- Color is not the only countdown signal (number + ring shape both convey time).
- Contrast: token pairs chosen for ≥ WCAG AA body text on `--bg`/`--surface`.
- Interactive non-button elements (clickable card, scrim) get `role`/`tabindex`/key handlers to
  keep `svelte-check` a11y warnings at 0 (as already done for existing modals).
