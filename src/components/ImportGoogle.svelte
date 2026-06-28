<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { decodeQrFile, previewMigration, importMigration } from "../lib/ipc";
  import type { AccountView } from "../lib/types";
  const dispatch = createEventDispatcher();

  let uri = "";
  let preview: AccountView[] = [];
  let selected: boolean[] = [];
  let error = "";

  async function loadFromFile() {
    error = "";
    const file = await open({ multiple: false, filters: [{ name: "Image", extensions: ["png","jpg","jpeg"] }] });
    if (!file || Array.isArray(file)) return;
    try {
      const found = await decodeQrFile(file);
      const mig = found.find((s) => s.startsWith("otpauth-migration://"));
      if (!mig) { error = "No Google Authenticator export QR found"; return; }
      uri = mig;
      await loadPreview();
    } catch (e) { error = String(e); }
  }

  async function loadPreview() {
    error = "";
    try {
      preview = await previewMigration(uri);
      selected = preview.map(() => true);
    } catch (e) { error = String(e); preview = []; }
  }

  async function doImport() {
    const indices = selected.map((v, i) => (v ? i : -1)).filter((i) => i >= 0);
    if (indices.length === 0) return;
    try {
      await importMigration(uri, indices);
      dispatch("added");
    } catch (e) { error = String(e); }
  }
</script>

<div class="vo-form">
  <button class="vo-ghost" on:click={loadFromFile}>Choose export QR image…</button>
  <input class="vo-field" bind:value={uri} placeholder="…or paste otpauth-migration:// URI" on:change={loadPreview} />
  {#if error}<p class="vo-err">{error}</p>{/if}
  {#if preview.length}
    <div class="preview">
      {#each preview as p, i}
        <label class="prow">
          <input type="checkbox" bind:checked={selected[i]} />
          <span class="pi">{p.issuer || "—"}</span>
          <span class="pl">{p.label}</span>
        </label>
      {/each}
    </div>
    <button class="vo-primary" on:click={doImport}>Import selected ({selected.filter(Boolean).length})</button>
  {/if}
</div>

<style>
  .preview { display: flex; flex-direction: column; gap: 6px; max-height: 220px; overflow: auto; }
  .prow { display: flex; align-items: center; gap: 10px; padding: 9px 10px;
    background: var(--bg); border: 1px solid var(--border); border-radius: 10px; cursor: pointer; }
  .pi { font-size: 13px; font-weight: 600; color: var(--text); }
  .pl { font-size: 11px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
