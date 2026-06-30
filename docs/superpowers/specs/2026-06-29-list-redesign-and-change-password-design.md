# Design Spec — Account List Redesign + Change Master Password

- **Date:** 2026-06-29
- **Status:** Approved (design via visual companion); pending implementation plan
- **Scope:** Two frontend-only pieces — (1) restyle the account list to a modern, clean "bordered-card" design, and (2) add a "Change master password" form to Settings for password-mode vaults. No backend/crypto changes (the `change_password` command + `changePassword` IPC already exist and are tested).

---

## 1. Goals

1. **Account list redesign** — modern, clean look: each account is its own rounded **bordered card** (keeps today's border treatment) laid out the airy "B" way — code on the right with a thin **linear** countdown bar along the card's bottom edge (replacing the circular SVG ring). Quieter copy feedback; cleaner select-mode and empty state. Light + dark.
2. **Change master password** — a labeled, validated form inside Settings, visible only for password-only vaults.
3. **Labeled password fields** — give every password input an explicit caption label (not placeholder-only), in the new form and in the existing Unlock/create screen, so fields stay identifiable after typing.

## 2. Non-goals

- No backend, crypto, IPC, or data-format changes. `change_password`/`changePassword` already exist.
- Change-password is **password-mode vaults only** — composite and key-file vaults do not show it (matches the existing `add_keyfile`/`remove_keyfile` scope; full credential management is out of scope).
- No new dependencies. No database / no SQL.
- Not redesigning Settings/AddMenu/VaultPicker layouts beyond adding the new section and field labels.

## 3. Current state (verified)

- `src/components/AccountCard.svelte`: a flex row — colored initial `badge`, a `.main` button (issuer/label/code, the whole thing is the copy button), a transient `Copied ✓` floating `.toast`, an SVG **ring** (`circle.track` + `circle.fill`, with a center `.num` seconds) using `ringCircumference`/`ringDashoffset` from `src/lib/display.ts`, and a hover-only delete `🗑`. Code is mono, accent, letter-spaced. Tokens from `src/app.css`.
- `src/routes/Main.svelte`: `header` (brand + tools ＋/🔒/⚙/☑), an `export-bar` shown in `selectMode`, a `.list` of `.card-row`s where (in select mode) a `.sel-check` checkbox is rendered as a **sibling before** the `AccountCard`, an `.empty` state. `toggleSelect`/`doExport`/`selectMode`/`selected` already exist.
- `src/lib/display.ts`: `initial`, `badgeColor`, `groupCode`, `ringCircumference`, `ringDashoffset`, `passwordStrength` — each unit-tested in `src/lib/display.test.ts`.
- `src/components/Settings.svelte`: sections for Appearance, Security (key file: add generated / add existing / remove, gated on `secMode`), and Vault (current path, save-a-copy, switch). It already imports `vaultMode`/`addKeyfile`/`removeKeyfile`/`generateKeyfile` and reads `secMode` via `refreshMode()`. It currently imports nothing for change-password (the unused `changePassword` import was removed in the prior cleanup).
- `src/lib/ipc.ts`: `changePassword(currentPassword, currentKeyfilePath, newPassword, newKeyfilePath)` exists and is wired to the `change_password` command.
- `src/app.css`: design tokens + shared `.vo-field`/`.vo-primary`/`.vo-ghost`/`.vo-form`/`.vo-err`. Light is `:root`; dark via `prefers-color-scheme` and `[data-theme="dark"]`.

## 4. Account list redesign

### 4.1 Card layout (`AccountCard.svelte`)

Per-account **bordered rounded card** (`background: var(--surface)`, `1px solid var(--border)`, `border-radius: var(--radius)`), `position: relative`, with a bottom-edge timer. Internal layout, left→right:

- **Badge** — unchanged: rounded square (`--radius-sm`-ish, 10px), `badgeColor(issuer)`, `initial(issuer)`, 34px.
- **Middle** (`flex:1; min-width:0`) — `issuer` (600 weight, `--text`) over `label` (11px, `--text-muted`, ellipsis).
- **Right** (column, right-aligned) — `code` (mono, ~22px, 600, letter-spacing ~4px, `var(--accent)`) over a small **seconds** caption (`{remaining}s`, 10px, `--text-muted`).
- **Bottom linear timer** — absolutely positioned bar inset from the rounded corners (`left/right: 13px; bottom: 6px; height: 3px; border-radius`), track `var(--ring-track)`, fill `var(--accent)`, width `= remaining/period`. Replaces the SVG ring entirely.
- **Delete** — a small `🗑` ghost icon, absolutely positioned top-right of the card, `opacity:0` → fades in on `.card:hover` (same affordance as today, relocated so it doesn't disturb the code/seconds column).
- **Hover** — border → `var(--accent)` plus a soft `box-shadow: 0 0 0 3px var(--accent-weak)` (matches the focus ring elsewhere).

The whole card remains the **copy button** (click → copy code), except in select mode (§4.3).

### 4.2 Copy feedback (replaces the floating toast)

On copy: the card briefly (~1.2s) tints — `background: var(--accent-weak)`, `border-color: var(--accent)` — and the **seconds caption is replaced by `Copied ✓`** (in `var(--success)`). No floating `.toast`. Clipboard auto-clear behavior (`settings.clipboardClearMs`) is unchanged.

### 4.3 Select mode (`Main.svelte` + card)

- The checkbox moves **inside** the card as the leading element (slides in when `selectMode`), styled as a rounded box that fills accent with a `✓` when checked — instead of today's sibling-before-card raw `<input>`.
- **Selected** cards get `border-color: var(--accent)` + `background: var(--accent-weak)` tint.
- In select mode, clicking a card **toggles selection** (no copy). The export bar (QR PNGs / Text / Google + count) sits under the header, lightly restyled to the token system; the existing plaintext-secret warning copy is unchanged.
- A native checkbox `<input type="checkbox">` stays underneath (visually replaced) for accessibility, with the existing `aria-label`.

### 4.4 Timer helper + display.ts

- Add a pure `barFraction(remaining: number, period: number): number` (clamped 0–1) to `src/lib/display.ts`, unit-tested in `display.test.ts`. The card computes the fill width from it.
- Remove the now-unused `ringCircumference`/`ringDashoffset` and their tests (the card was their only consumer). Keep `initial`/`badgeColor`/`groupCode`/`passwordStrength`.
- `<5s` warning: when `remaining <= 5`, the code, seconds caption, and bar fill switch to `var(--danger)` (same threshold/behavior as today's ring `.warn`).

### 4.5 Empty state

Keep the centered treatment, polish copy: a large muted 🔐, **"No accounts yet"** (600, `--text`), and a hint line "Press ＋ to add your first one — scan a QR, paste a key, or import." Tokens only.

## 5. Change master password (`Settings.svelte`)

### 5.1 Behavior

- A new **"Master password"** section, rendered **only when `secMode === "password"`** (read from the existing `vaultMode()` via `refreshMode()`).
- Three **labeled** fields (each a small bold caption above its input): **Current password**, **New password**, **Confirm new password**. The New-password field shows the existing `passwordStrength` meter + a "Weak/Fair/Good/Strong" caption beneath it.
- A **Change password** primary button; inline success ("Password changed ✓") / error.
- **Validation** mirrors create-vault: new password length ≥ 8 and new === confirm (checked client-side before the call); on success clear the fields. Calls `changePassword(currentPassword, undefined, newPassword, undefined)` (password-mode → no key-file paths). A wrong current password surfaces the backend's opaque `AppError::Crypto` message.
- Re-add the `changePassword` import to `Settings.svelte` (removed in the prior cleanup; now actually used).

### 5.2 Failure handling

- Mismatch / too-short → inline `.vo-err`, no IPC call.
- Backend error (wrong current password, etc.) → show the returned message in the error line; fields retained so the user can retry.

## 6. Labeled password fields (consistency)

- Introduce a shared `.vo-label` class in `src/app.css` (small, 600, `--text-muted`, block, 4px bottom margin) for field captions.
- Apply labels to the new Master-password fields (§5) and to the existing **Unlock/create** screen (`src/routes/Unlock.svelte`) password/confirm/key-file fields, which are placeholder-only today. This is a small markup change; logic unchanged.

## 7. Theming & accessibility

- Everything uses existing CSS custom properties → light/dark/system continue to work with no new palette entries.
- Keep `aria-label`s on icon-only controls (delete, tools, select checkbox). Labeled fields improve form a11y. The linear bar is decorative; the seconds caption conveys the same info as text.

## 8. Testing

- **Pure logic:** `barFraction` unit test in `display.test.ts` (0/period→0, full→1, clamp beyond range); remove `ringCircumference`/`ringDashoffset` tests.
- **Components** (no Svelte test harness): verify via `npx svelte-check` (0 errors) + `npm run build` + manual run — copy feedback, `<5s` warn, select-mode toggle/export, empty state, and the Master-password section appearing only for password-mode vaults with working validation.
- Existing `vitest` suite stays green.

## 9. Files touched

- `src/components/AccountCard.svelte` — card layout, linear timer, copy feedback, hover, relocated delete.
- `src/routes/Main.svelte` — in-card checkbox, selected-card styling, export-bar polish, empty-state copy.
- `src/lib/display.ts` + `src/lib/display.test.ts` — add `barFraction`, remove ring helpers + tests.
- `src/components/Settings.svelte` — Master-password section (+ re-add `changePassword` import).
- `src/routes/Unlock.svelte` — labeled fields.
- `src/app.css` — `.vo-label` shared class.

## 10. Open risks

- The bottom linear bar must inset within the card's border radius so it doesn't clip the rounded corners (use left/right/bottom insets, not full-bleed).
- Delete-on-hover top-right must not overlap the code/seconds column at narrow widths — give the right column a little right padding when hovered, or place the delete above the bar.
