<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { decodeQrFile, addFromUri } from "../lib/ipc";
  const dispatch = createEventDispatcher();
  let error = "", status = "", busy = false;

  async function addUris(uris: string[]) {
    error = "";
    let added = 0;
    for (const u of uris) {
      try { await addFromUri(u); added++; } catch (_) { /* skip invalid */ }
    }
    if (added === 0) { error = "No valid TOTP QR found"; return; }
    dispatch("added");
  }

  async function fromFile() {
    if (busy) return;
    error = ""; status = "";
    const selected = await open({
      multiple: false,
      filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg", "bmp", "gif"] }],
    });
    if (!selected || Array.isArray(selected)) return;
    busy = true;
    try {
      status = "Decoding…";
      const uris = await decodeQrFile(selected as string);
      await addUris(uris);
    } catch (e) { error = String(e); } finally { status = ""; busy = false; }
  }

</script>

<div class="vo-form">
  <div class="dropzone">🖼️ Choose a QR image file<br /><span>PNG / JPG containing an otpauth:// code</span></div>
  <button class="vo-ghost" on:click={fromFile} disabled={busy}>Choose image…</button>
  {#if status}<p class="status">{status}</p>{/if}
  {#if error}<p class="vo-err">{error}</p>{/if}
</div>

<style>
  .dropzone { border: 1.5px dashed var(--border); border-radius: var(--radius-sm);
    padding: 22px; text-align: center; color: var(--text-muted); font-size: 13px; }
  .dropzone span { font-size: 11px; }
  .status { color: var(--text-muted); font-size: 13px; margin: 0; }
</style>
