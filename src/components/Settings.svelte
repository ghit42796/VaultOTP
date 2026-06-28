<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import { exportBackup, importBackup } from "../lib/ipc";
  import { loadSettings, saveSettings } from "../lib/settings";
  const dispatch = createEventDispatcher();
  let pw = "", error = "", status = "";
  let s = loadSettings();

  function persist() { saveSettings(s); }

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

<div class="overlay" on:click={() => dispatch("close")}>
  <div class="modal" on:click|stopPropagation>
    <h2>Settings</h2>
    <label>Auto-lock (minutes)
      <input type="number" min="1" value={s.idleLockMs/60000}
             on:change={(e)=>{s.idleLockMs=Number(e.currentTarget.value)*60000;persist();}} />
    </label>
    <label>Clear clipboard after (seconds)
      <input type="number" min="0" value={s.clipboardClearMs/1000}
             on:change={(e)=>{s.clipboardClearMs=Number(e.currentTarget.value)*1000;persist();}} />
    </label>
    <hr />
    <h3>Encrypted backup</h3>
    <input type="password" bind:value={pw} placeholder="Backup password" />
    <div class="row"><button on:click={doExport}>Export…</button><button on:click={doImport}>Import…</button></div>
    {#if status}<p class="ok">{status}</p>{/if}
    {#if error}<p class="error">{error}</p>{/if}
    <button class="cancel" on:click={() => dispatch("close")}>Close</button>
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display:flex; align-items:center; justify-content:center; }
  .modal { background:#fff; padding:18px; border-radius:8px; min-width:300px; display:flex; flex-direction:column; gap:10px; }
  label { display:flex; justify-content:space-between; align-items:center; gap:8px; }
  .row { display:flex; gap:8px; } .row button { flex:1; }
  .ok { color:#27ae60; } .error { color:#c0392b; }
</style>
