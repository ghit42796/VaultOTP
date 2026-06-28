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

<div class="qr">
  <button on:click={fromFile} disabled={busy}>Choose image…</button>
  {#if status}<p>{status}</p>{/if}
  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .qr { display: flex; flex-direction: column; gap: 10px; }
  button { padding: 9px; cursor: pointer; }
  button:disabled { opacity: 0.6; cursor: not-allowed; }
  .error { color: #c0392b; font-size: 13px; }
</style>
