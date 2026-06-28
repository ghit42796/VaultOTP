<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { vaultExists, createVault, unlock } from "../lib/ipc";
  import { passwordStrength } from "../lib/display";

  const dispatch = createEventDispatcher();
  let exists = false;
  let password = "";
  let confirmPassword = "";
  let error = "";
  let busy = false;
  let passwordInput: HTMLInputElement;
  let confirmInput: HTMLInputElement;

  $: strength = passwordStrength(password);

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
  <div class="lock">🔐</div>
  <h2>{exists ? "Unlock VaultOTP" : "Create a master password"}</h2>
  <p class="sub">{exists ? "Enter your master password" : "This encrypts your vault. There is no recovery if you forget it."}</p>

  <input class="field" type="password" bind:this={passwordInput} bind:value={password}
         placeholder="Master password"
         on:keydown={handleFirstKeydown} />

  {#if !exists}
    <div class="meter" aria-hidden="true"><i style="width:{strength * 25}%"></i></div>
    <input class="field" type="password" bind:this={confirmInput} bind:value={confirmPassword}
           placeholder="Confirm password"
           on:keydown={(e) => e.key === "Enter" && submit()} />
  {/if}

  {#if error}<p class="err">{error}</p>{/if}
  <button class="primary" on:click={submit} disabled={busy}>{exists ? "Unlock" : "Create vault"}</button>
</div>

<style>
  .unlock { display: flex; flex-direction: column; align-items: center; justify-content: center;
            gap: var(--space-3); height: 100%; padding: 40px 36px; }
  .lock { font-size: 42px; }
  h2 { margin: 0; font-size: 20px; color: var(--text); }
  .sub { margin: 0; color: var(--text-muted); font-size: 13px; text-align: center; }
  .field { width: 100%; padding: 12px 14px; border-radius: var(--radius-sm);
           border: 1px solid var(--border); background: var(--surface); color: var(--text); font-size: 14px; }
  .field::placeholder { color: var(--text-muted); }
  .field:focus { outline: none; border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-weak); }
  .meter { width: 100%; height: 6px; border-radius: 6px; background: var(--surface-2); overflow: hidden; }
  .meter > i { display: block; height: 100%; background: var(--success); transition: width .15s; }
  .err { color: var(--danger); font-size: 13px; margin: 0; }
  .primary { width: 100%; padding: 12px; border: none; border-radius: var(--radius-sm);
             background: var(--accent); color: var(--accent-contrast); font-weight: 600; font-size: 14px; cursor: pointer; }
  .primary:disabled { opacity: .6; cursor: default; }
</style>
