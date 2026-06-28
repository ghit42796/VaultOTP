<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { addManual } from "../lib/ipc";
  const dispatch = createEventDispatcher();
  let issuer = "", label = "", secret = "", error = "";

  $: if (issuer || label || secret) error = "";

  async function save() {
    error = "";
    try {
      await addManual(issuer.trim(), label.trim(), secret.trim());
      dispatch("added");
    } catch (e) { error = String(e); }
  }
</script>

<div class="form">
  <input bind:value={issuer} placeholder="Issuer (e.g. GitHub)" />
  <input bind:value={label} placeholder="Label (e.g. you@example.com)" />
  <input bind:value={secret} placeholder="Secret key (Base32)" />
  {#if error}<p class="error">{error}</p>{/if}
  <button on:click={save} disabled={!issuer || !secret}>Add</button>
</div>

<style>
  .form { display: flex; flex-direction: column; gap: 10px; }
  input { padding: 9px; }
  .error { color: #c0392b; font-size: 13px; }
</style>
