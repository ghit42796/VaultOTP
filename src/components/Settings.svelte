<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import { exportBackup, importBackup } from "../lib/ipc";
  import { loadSettings, saveSettings } from "../lib/settings";
  import { loadThemePref, saveThemePref, applyTheme, type ThemePref } from "../lib/theme";
  const dispatch = createEventDispatcher();
  let pw = "", error = "", status = "";
  let s = loadSettings();
  let theme: ThemePref = loadThemePref();

  function persist() { saveSettings(s); }
  function setTheme(p: ThemePref) { theme = p; saveThemePref(p); applyTheme(p); }

  async function doExport() {
    error = ""; if (!pw) { error = "Enter a backup password"; return; }
    const path = await save({ defaultPath: "vaultotp-backup.bin" });
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

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="overlay" role="presentation" on:click={() => dispatch("close")}>
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-noninteractive-element-interactions -->
  <div
    class="sheet"
    role="dialog"
    aria-modal="true"
    aria-label="Settings"
    on:click|stopPropagation
  >
    <h2>Settings</h2>

    <section class="setting">
      <span class="k">Appearance</span>
      <span class="d">Theme follows your OS by default.</span>
      <div class="seg">
        <button class:active={theme === "system"} on:click={() => setTheme("system")}>System</button>
        <button class:active={theme === "light"} on:click={() => setTheme("light")}>Light</button>
        <button class:active={theme === "dark"} on:click={() => setTheme("dark")}>Dark</button>
      </div>
    </section>

    <div class="divider"></div>

    <section class="setting">
      <span class="k">Auto-lock after idle</span>
      <div class="stepper">
        <input
          class="box vo-field"
          type="number"
          min="1"
          aria-label="Auto-lock idle time in minutes"
          value={s.idleLockMs / 60000}
          on:change={(e) => { s.idleLockMs = Number(e.currentTarget.value) * 60000; persist(); }}
        />
        <span class="d">minutes</span>
      </div>
    </section>

    <section class="setting">
      <span class="k">Clear clipboard after copy</span>
      <div class="stepper">
        <input
          class="box vo-field"
          type="number"
          min="0"
          aria-label="Clear clipboard after seconds"
          value={s.clipboardClearMs / 1000}
          on:change={(e) => { s.clipboardClearMs = Number(e.currentTarget.value) * 1000; persist(); }}
        />
        <span class="d">seconds</span>
      </div>
    </section>

    <div class="divider"></div>

    <section class="setting">
      <span class="k">Encrypted backup</span>
      <input class="vo-field" type="password" bind:value={pw} placeholder="Backup password" />
      <div class="row"><button class="vo-ghost" on:click={doExport}>Export…</button><button class="vo-ghost" on:click={doImport}>Import…</button></div>
      <div aria-live="polite" aria-atomic="true">
        {#if status}<p class="ok">{status}</p>{/if}
        {#if error}<p class="vo-err">{error}</p>{/if}
      </div>
    </section>

    <button class="vo-ghost close-btn" on:click={() => dispatch("close")}>Close</button>
  </div>
</div>

<style>
  .overlay {
    position: fixed; inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex; align-items: flex-end;
  }
  .sheet {
    width: 100%;
    background: var(--surface);
    border-top-left-radius: 18px;
    border-top-right-radius: 18px;
    padding: 18px;
    display: flex; flex-direction: column; gap: 16px;
  }
  .sheet h2 { margin: 0; font-size: 16px; color: var(--text); }

  .setting { display: flex; flex-direction: column; gap: var(--space-2); }
  .setting .k { font-size: 13px; font-weight: 600; color: var(--text); }
  .setting .d { font-size: 12px; color: var(--text-muted); }

  .seg { display: flex; background: var(--surface-2); border-radius: 10px; padding: 3px; gap: 3px; }
  .seg button {
    flex: 1; padding: 7px; border: none; border-radius: 8px;
    background: none; color: var(--text-muted); font-size: 13px; cursor: pointer;
  }
  .seg button.active { background: var(--accent); color: var(--accent-contrast); font-weight: 600; }

  .stepper { display: flex; align-items: center; gap: 10px; }
  .stepper .box { width: 72px; padding: 7px 12px; font-size: 14px; }

  .divider { height: 1px; background: var(--border); }

  .row { display: flex; gap: 8px; }
  .row .vo-ghost { flex: 1; }

  .ok { color: var(--success); font-size: 13px; margin: 0; }

  .close-btn { width: 100%; }
</style>
