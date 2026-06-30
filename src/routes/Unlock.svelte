<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { vaultExists, vaultMode, createVault, unlock } from "../lib/ipc";
  import { open } from "@tauri-apps/plugin-dialog";
  import { passwordStrength } from "../lib/display";

  const dispatch = createEventDispatcher();
  let exists = false;
  let password = "";
  let confirmPassword = "";
  let error = "";
  let busy = false;
  let passwordInput: HTMLInputElement;
  let confirmInput: HTMLInputElement;
  let mode: "password" | "keyfile" | "composite" = "password";
  let keyfilePath = "";

  $: strength = passwordStrength(password);

  async function pickKeyfile() {
    const p = await open({ multiple: false });
    if (typeof p === "string") keyfilePath = p;
  }

  onMount(async () => {
    try {
      exists = await vaultExists();
    } catch {
      error = "Cannot access vault storage. Check file permissions.";
    }
    if (exists) {
      try { mode = await vaultMode(); } catch { /* leave default */ }
    }
    passwordInput?.focus();
  });

  async function submit() {
    error = "";
    busy = true;
    try {
      if (exists) {
        const pw = mode === "keyfile" ? undefined : password;
        const kf = mode === "password" ? undefined : (keyfilePath || undefined);
        if (mode !== "password" && !kf) { error = "Select a key file"; return; }
        await unlock(pw, kf);
      } else {
        if (mode !== "keyfile") {
          if (password.length < 8) { error = "Password must be at least 8 characters"; return; }
          if (password !== confirmPassword) { error = "Passwords do not match"; return; }
        }
        if (mode !== "password" && !keyfilePath) { error = "Select a key file"; return; }
        await createVault(
          mode,
          mode === "keyfile" ? undefined : password,
          mode === "password" ? undefined : keyfilePath,
        );
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
    if (!exists && mode !== "keyfile" && confirmPassword === "") {
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

  {#if !exists}
    <div class="modes">
      <button class:active={mode === "password"} on:click={() => (mode = "password")}>Password</button>
      <button class:active={mode === "keyfile"} on:click={() => (mode = "keyfile")}>Key file</button>
      <button class:active={mode === "composite"} on:click={() => (mode = "composite")}>Both</button>
    </div>
  {/if}

  {#if mode !== "keyfile"}
    <div class="vo-group">
      <label class="vo-label" for="vo-pw">Master password</label>
      <input id="vo-pw" class="field" type="password" bind:this={passwordInput} bind:value={password}
             placeholder="Master password"
             on:keydown={handleFirstKeydown} />
    </div>
  {/if}

  {#if !exists && mode !== "keyfile"}
    <div class="meter" aria-hidden="true"><i style="width:{strength * 25}%"></i></div>
    <div class="vo-group">
      <label class="vo-label" for="vo-cpw">Confirm password</label>
      <input id="vo-cpw" class="field" type="password" bind:this={confirmInput} bind:value={confirmPassword}
             placeholder="Confirm password"
             on:keydown={(e) => e.key === "Enter" && submit()} />
    </div>
  {/if}

  {#if mode !== "password"}
    <div class="vo-group">
      <span class="vo-label">Key file</span>
      <button class="vo-ghost" on:click={pickKeyfile}>{keyfilePath ? "Key file ✓" : "Select key file…"}</button>
    </div>
  {/if}

  {#if error}<p class="err">{error}</p>{/if}
  <button class="primary" on:click={submit} disabled={busy}>{exists ? "Unlock" : "Create vault"}</button>
  <button class="vo-ghost" on:click={() => dispatch("switch")}>← Switch vault</button>
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
  .modes { display: flex; gap: var(--space-2); width: 100%; }
  .modes button { flex: 1; padding: 8px; border-radius: var(--radius-sm);
                  border: 1px solid var(--border); background: var(--surface); color: var(--text-muted);
                  font-size: 13px; cursor: pointer; }
  .modes button.active { border-color: var(--accent); color: var(--accent); background: var(--accent-weak); }
  .vo-ghost { width: 100%; padding: 10px 14px; border-radius: var(--radius-sm);
              border: 1px dashed var(--border); background: transparent; color: var(--text-muted);
              font-size: 13px; cursor: pointer; }
  .vo-ghost:hover { border-color: var(--accent); color: var(--accent); }
</style>
