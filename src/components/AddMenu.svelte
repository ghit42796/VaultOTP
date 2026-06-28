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
<div class="overlay" role="presentation" on:click={() => dispatch("close")}>
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-noninteractive-element-interactions -->
  <div class="modal" role="dialog" aria-modal="true" on:click|stopPropagation>
    <nav>
      <button class:active={tab==="manual"} on:click={() => tab="manual"}>Manual</button>
      <button class:active={tab==="qr"} on:click={() => tab="qr"}>QR</button>
      <button class:active={tab==="import"} on:click={() => tab="import"}>Import</button>
    </nav>
    {#if tab === "manual"}<AddManual on:added={done} />{/if}
    {#if tab === "qr"}<AddFromQr on:added={done} />{/if}
    {#if tab === "import"}<ImportGoogle on:added={done} />{/if}
    <button class="cancel" on:click={() => dispatch("close")}>Cancel</button>
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; }
  .modal { background: #fff; padding: 18px; border-radius: 8px; min-width: 300px; display: flex; flex-direction: column; gap: 12px; }
  nav { display: flex; gap: 6px; }
  nav button { flex: 1; padding: 6px; cursor: pointer; }
  nav button.active { background: #2980b9; color: #fff; }
  .cancel { margin-top: 4px; }
</style>
