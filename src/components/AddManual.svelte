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

<div class="vo-form">
  <input class="vo-field" bind:value={issuer} placeholder="Issuer (e.g. GitHub)" />
  <input class="vo-field" bind:value={label} placeholder="Label (e.g. you@example.com)" />
  <input class="vo-field" bind:value={secret} placeholder="Secret key (Base32)" style="font-family:var(--font-mono)" />
  {#if error}<p class="vo-err">{error}</p>{/if}
  <button class="vo-primary" on:click={save} disabled={!issuer || !secret}>Add</button>
</div>
