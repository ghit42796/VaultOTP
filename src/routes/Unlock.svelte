<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { vaultExists, createVault, unlock } from "../lib/ipc";

  const dispatch = createEventDispatcher();
  let exists = false;
  let password = "";
  let confirmPassword = "";
  let error = "";
  let busy = false;
  let passwordInput: HTMLInputElement;
  let confirmInput: HTMLInputElement;

  onMount(async () => {
    try {
      exists = await vaultExists();
    } catch {
      error = "Cannot access vault storage. Check file permissions.";
    }
    passwordInput?.focus();
  });

  async function submit() {
    error = "";
    busy = true;
    try {
      if (exists) {
        await unlock(password);
      } else {
        if (password.length < 8) { error = "Password must be at least 8 characters"; return; }
        if (password !== confirmPassword) { error = "Passwords do not match"; return; }
        await createVault(password);
      }
      password = ""; confirmPassword = "";
      dispatch("unlocked");
    } catch (e) {
      error = exists
        ? String(e)                              // backend already-safe opaque message
        : "Operation failed. Please try again."; // createVault (unexpected) failure
    } finally {
      busy = false;
    }
  }

  function handleFirstKeydown(e: KeyboardEvent) {
    if (e.key !== "Enter") return;
    if (!exists && confirmPassword === "") {
      confirmInput?.focus();
    } else {
      submit();
    }
  }
</script>

<div class="unlock">
  <h1>🔐 VaultOTP</h1>
  <p>{exists ? "Enter your master password" : "Create a master password"}</p>
  <input
    type="password"
    bind:this={passwordInput}
    bind:value={password}
    placeholder="Master password"
    on:keydown={handleFirstKeydown}
  />
  {#if !exists}
    <input
      type="password"
      bind:this={confirmInput}
      bind:value={confirmPassword}
      placeholder="Confirm password"
      on:keydown={(e) => e.key === "Enter" && submit()}
    />
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
  <button on:click={submit} disabled={busy}>{exists ? "Unlock" : "Create vault"}</button>
</div>

<style>
  .unlock { display: flex; flex-direction: column; gap: 12px; padding: 32px; max-width: 320px; margin: 0 auto; }
  input { padding: 10px; font-size: 14px; }
  button { padding: 10px; font-weight: 600; cursor: pointer; }
  .error { color: #c0392b; font-size: 13px; }
</style>
