<script lang="ts">
  import { onMount, createEventDispatcher } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { listRecentVaults, setCurrentVault, type RecentVaultView } from "../lib/ipc";

  const dispatch = createEventDispatcher();
  let recents: RecentVaultView[] = [];
  let error = "";

  onMount(async () => {
    try { recents = await listRecentVaults(); } catch (e) { error = String(e); }
  });

  async function choose(path: string) {
    error = "";
    try { await setCurrentVault(path); dispatch("selected"); }
    catch (e) { error = String(e); }
  }

  async function openExisting() {
    const p = await open({ multiple: false });
    if (typeof p === "string") await choose(p);
  }

  async function createNew() {
    const p = await save({ defaultPath: "vault.bin" });
    if (typeof p === "string") await choose(p); // Unlock will show "create" since the file does not exist yet
  }
</script>

<div class="picker">
  <div class="logo">🔐</div>
  <h2>VaultOTP</h2>
  <p class="sub">Open a vault or create a new one</p>

  {#if recents.length}
    <div class="recents">
      {#each recents as r}
        <button class="recent" class:missing={!r.exists} on:click={() => choose(r.path)} title={r.path}>
          <span class="name">{r.path.split("/").pop()}</span>
          <span class="path">{r.path}</span>
          {#if !r.exists}<span class="badge">missing</span>{/if}
        </button>
      {/each}
    </div>
  {/if}

  {#if error}<p class="err">{error}</p>{/if}

  <div class="actions">
    <button class="primary" on:click={openExisting}>Open vault…</button>
    <button class="vo-ghost" on:click={createNew}>Create new vault…</button>
  </div>
</div>

<style>
  .picker { display: flex; flex-direction: column; align-items: center; gap: var(--space-3); height: 100%; padding: 36px; }
  .logo { font-size: 42px; }
  h2 { margin: 0; font-size: 20px; color: var(--text); }
  .sub { margin: 0; color: var(--text-muted); font-size: 13px; }
  .recents { width: 100%; display: flex; flex-direction: column; gap: 8px; max-height: 240px; overflow: auto; }
  .recent { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; padding: 10px 12px;
            border: 1px solid var(--border); border-radius: var(--radius-sm); background: var(--surface);
            color: var(--text); cursor: pointer; text-align: left; }
  .recent:hover { border-color: var(--accent); }
  .recent.missing { opacity: .55; }
  .recent .name { font-weight: 600; font-size: 14px; }
  .recent .path { font-size: 11px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 100%; }
  .recent .badge { font-size: 10px; color: var(--danger); }
  .actions { display: flex; gap: 8px; width: 100%; }
  .actions .primary { flex: 1; padding: 12px; border: none; border-radius: var(--radius-sm);
            background: var(--accent); color: var(--accent-contrast); font-weight: 600; cursor: pointer; }
  .err { color: var(--danger); font-size: 13px; margin: 0; }
</style>
