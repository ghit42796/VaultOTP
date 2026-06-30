<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import { exportBackup, importBackup, vaultMode, addKeyfile, removeKeyfile, generateKeyfile, currentVaultPath, saveVaultAs, changePassword } from "../lib/ipc";
  import { passwordStrength } from "../lib/display";
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

  let secMode: "password" | "keyfile" | "composite" = "password";
  let secPw = "", secStatus = "", secError = "";

  async function refreshMode() { try { secMode = await vaultMode(); } catch {} }
  refreshMode();

  async function doGenerateKeyfile(): Promise<string | null> {
    const path = await save({ defaultPath: "vaultotp.vaultkey" });
    if (typeof path !== "string") return null;
    await generateKeyfile(path);
    return path;
  }

  async function addGenerated() {
    secError = ""; secStatus = "";
    if (!secPw) { secError = "Enter your current password"; return; }
    try {
      const path = await doGenerateKeyfile();
      if (!path) return;
      await addKeyfile(secPw, path); secStatus = "Key file added."; secPw = ""; await refreshMode();
    } catch (e) { secError = String(e); }
  }

  async function addExisting() {
    secError = ""; secStatus = "";
    if (!secPw) { secError = "Enter your current password"; return; }
    const path = await open({ multiple: false });
    if (typeof path !== "string") return;
    try { await addKeyfile(secPw, path); secStatus = "Key file added."; secPw = ""; await refreshMode(); }
    catch (e) { secError = String(e); }
  }

  async function dropKeyfile() {
    secError = ""; secStatus = "";
    if (!secPw) { secError = "Enter your current password"; return; }
    const path = await open({ multiple: false });
    if (typeof path !== "string") { secError = "Select your current key file to confirm"; return; }
    try { await removeKeyfile(secPw, path); secStatus = "Key file removed."; secPw = ""; await refreshMode(); }
    catch (e) { secError = String(e); }
  }

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

    <div class="divider"></div>

    <section class="setting">
      <h3>Security</h3>
      <p class="hint">Mode: {secMode === "composite" ? "Password + key file" : secMode === "keyfile" ? "Key file only" : "Password only"}</p>
      <input class="vo-field" type="password" bind:value={secPw} placeholder="Current password" />
      {#if secMode === "password"}
        <div class="row">
          <button class="vo-ghost" on:click={addGenerated}>Generate key file…</button>
          <button class="vo-ghost" on:click={addExisting}>Use existing file…</button>
        </div>
      {/if}
      {#if secMode === "composite"}
        <div class="row"><button class="vo-ghost" on:click={dropKeyfile}>Remove key file…</button></div>
      {/if}
      <div aria-live="polite" aria-atomic="true">
        {#if secError}<p class="err">{secError}</p>{/if}
        {#if secStatus}<p class="hint">{secStatus}</p>{/if}
      </div>
      <p class="hint">If you lose all required credentials, the vault cannot be recovered. A generated key file has stronger entropy than an existing file.</p>
    </section>

    <div class="divider"></div>

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

    <section class="setting">
      <h3>Vault</h3>
      <p class="hint" title={vaultPath}>Current: {vaultPath}</p>
      <div class="row">
        <button class="vo-ghost" on:click={doSaveAs}>Save a copy as…</button>
        <button class="vo-ghost" on:click={() => dispatch("switchVault")}>Open a different vault…</button>
      </div>
      <div aria-live="polite" aria-atomic="true">
        {#if vaultError}<p class="err">{vaultError}</p>{/if}
        {#if vaultStatus}<p class="hint">{vaultStatus}</p>{/if}
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

  .hint { font-size: 12px; color: var(--text-muted); margin: 0; }
  .err { font-size: 12px; color: var(--error, #e53e3e); margin: 0; }

  .setting h3 { margin: 0; font-size: 13px; font-weight: 600; color: var(--text); }

  .close-btn { width: 100%; }

  .meter { width: 100%; height: 6px; border-radius: 6px; background: var(--surface-2); overflow: hidden; }
  .meter > i { display: block; height: 100%; background: var(--success); transition: width .15s; }
</style>
