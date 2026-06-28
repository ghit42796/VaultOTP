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

<div class="import">
  <button on:click={loadFromFile}>Choose export QR image…</button>
  <input bind:value={uri} placeholder="…or paste otpauth-migration:// URI" on:change={loadPreview} />
  {#if error}<p class="error">{error}</p>{/if}
  {#if preview.length}
    <ul>
      {#each preview as p, i}
        <li><label><input type="checkbox" bind:checked={selected[i]} /> {p.issuer || "—"} · {p.label}</label></li>
      {/each}
    </ul>
    <button on:click={doImport}>Import selected</button>
  {/if}
</div>

<style>
  .import { display: flex; flex-direction: column; gap: 10px; }
  button, input { padding: 9px; }
  ul { list-style: none; padding: 0; margin: 0; max-height: 200px; overflow-y: auto; }
  li { padding: 4px 0; }
  .error { color: #c0392b; font-size: 13px; }
</style>
