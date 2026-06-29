<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import AddManual from "./AddManual.svelte";
  import AddFromQr from "./AddFromQr.svelte";
  import ImportGoogle from "./ImportGoogle.svelte";
  const dispatch = createEventDispatcher();
  let tab: "manual" | "qr" | "import" = "manual";
  function done() { dispatch("close"); }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="scrim" role="presentation" on:click={() => dispatch("close")}>
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-noninteractive-element-interactions -->
  <div class="modal" role="dialog" aria-modal="true" aria-label="Add account" on:click|stopPropagation>
    <div class="tabs">
      <button class:active={tab === "manual"} on:click={() => (tab = "manual")}>Manual</button>
      <button class:active={tab === "qr"} on:click={() => (tab = "qr")}>QR</button>
      <button class:active={tab === "import"} on:click={() => (tab = "import")}>Import</button>
    </div>
    {#if tab === "manual"}<AddManual on:added={done} />{/if}
    {#if tab === "qr"}<AddFromQr on:added={done} />{/if}
    {#if tab === "import"}<ImportGoogle on:added={done} />{/if}
    <button class="cancel" on:click={() => dispatch("close")}>Cancel</button>
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; background: rgba(0,0,0,.5);
           display: flex; align-items: center; justify-content: center; }
  .modal { width: 340px; background: var(--surface); border: 1px solid var(--border);
           border-radius: var(--radius); padding: var(--space-4);
           display: flex; flex-direction: column; gap: var(--space-3);
           box-shadow: 0 24px 60px rgba(0,0,0,.4); }
  .tabs { display: flex; background: var(--surface-2); border-radius: 10px; padding: 3px; gap: 3px; }
  .tabs button { flex: 1; padding: 8px; border: none; border-radius: 8px; background: none;
                 color: var(--text-muted); font-size: 13px; cursor: pointer; }
  .tabs button.active { background: var(--accent); color: var(--accent-contrast); font-weight: 600; }
  .cancel { background: none; border: none; color: var(--text-muted); font-size: 13px; cursor: pointer; }
</style>
