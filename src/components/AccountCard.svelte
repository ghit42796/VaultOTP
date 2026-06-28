<script lang="ts">
  import type { CodeView } from "../lib/types";
  import { createEventDispatcher } from "svelte";
  import { loadSettings } from "../lib/settings";
  export let item: CodeView;
  const dispatch = createEventDispatcher();
  const settings = loadSettings();
  let copied = false;

  async function copy() {
    await navigator.clipboard.writeText(item.code);
    copied = true;
    setTimeout(() => (copied = false), 1500);
    const ms = settings.clipboardClearMs;
    if (ms > 0) {
      setTimeout(async () => {
        try {
          const current = await navigator.clipboard.readText();
          if (current === item.code) await navigator.clipboard.writeText("");
        } catch (_) { /* clipboard read may be blocked; ignore */ }
      }, ms);
    }
  }
</script>

<div class="card">
  <div class="info">
    <span class="issuer">{item.issuer || "—"}</span>
    <span class="label">{item.label}</span>
  </div>
  <button class="code" on:click={copy} title="Copy">
    {item.code.slice(0, 3)} {item.code.slice(3)}
    <span class="remaining" class:warn={item.remaining <= 5}>{item.remaining}s</span>
  </button>
  {#if copied}<span class="copied">Copied</span>{/if}
  <button class="del" on:click={() => dispatch("remove", item.id)} title="Delete">🗑</button>
</div>

<style>
  .card { display: flex; align-items: center; gap: 10px; padding: 10px 12px; border-bottom: 1px solid #eee; }
  .info { display: flex; flex-direction: column; flex: 1; min-width: 0; }
  .issuer { font-weight: 600; }
  .label { font-size: 12px; color: #777; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .code { font-family: monospace; font-size: 20px; letter-spacing: 2px; background: none; border: none; cursor: pointer; display: flex; align-items: center; gap: 8px; }
  .remaining { font-size: 12px; color: #2980b9; }
  .remaining.warn { color: #c0392b; }
  .copied { font-size: 12px; color: #27ae60; }
  .del { background: none; border: none; cursor: pointer; }
</style>
