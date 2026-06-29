<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import AccountCard from "../components/AccountCard.svelte";
  import AddMenu from "../components/AddMenu.svelte";
  import Settings from "../components/Settings.svelte";
  import type { CodeView } from "../lib/types";
  import { currentCodes, removeAccount, lock, onTick } from "../lib/ipc";
  import { loadSettings } from "../lib/settings";

  const dispatch = createEventDispatcher();
  let codes: CodeView[] = [];
  let unlisten: UnlistenFn | null = null;
  let showAdd = false;
  let showSettings = false;
  let idleTimer: number | undefined;
  const settings = loadSettings();

  async function refresh() { codes = await currentCodes(); }

  function resetIdle() {
    clearTimeout(idleTimer);
    if (settings.idleLockMs > 0) {
      idleTimer = window.setTimeout(async () => { await lock(); dispatch("locked"); }, settings.idleLockMs);
    }
  }

  onMount(async () => {
    await refresh();
    unlisten = await onTick(refresh);
    resetIdle();
    window.addEventListener("mousemove", resetIdle);
    window.addEventListener("keydown", resetIdle);
  });
  onDestroy(() => {
    unlisten?.();
    clearTimeout(idleTimer);
    window.removeEventListener("mousemove", resetIdle);
    window.removeEventListener("keydown", resetIdle);
  });

  async function doLock() {
    await lock();
    dispatch("locked");
  }
  async function remove(id: string) {
    await removeAccount(id);
    await refresh();
  }
</script>

<header>
  <div class="brand"><span class="logo">🔐</span> VaultOTP</div>
  <div class="tools">
    <button class="tool" on:click={() => (showAdd = true)} title="Add account" aria-label="Add account">＋</button>
    <button class="tool" on:click={doLock} title="Lock now" aria-label="Lock now">🔒</button>
    <button class="tool" on:click={() => (showSettings = true)} title="Settings" aria-label="Settings">⚙</button>
  </div>
</header>

<div class="list">
  {#each codes as item (item.id)}
    <AccountCard {item} on:remove={(e) => remove(e.detail)} />
  {/each}
  {#if codes.length === 0}
    <div class="empty">
      <div class="empty-icon">🔐</div>
      <p>No accounts yet.</p>
      <p class="empty-sub">Press ＋ to add your first one.</p>
    </div>
  {/if}
</div>

{#if showAdd}
  <AddMenu on:close={() => { showAdd = false; refresh(); }} />
{/if}
{#if showSettings}
  <Settings on:close={() => (showSettings = false)} on:changed={refresh} />
{/if}

<style>
  header {
    display: flex; align-items: center; justify-content: space-between;
    padding: var(--space-4) 18px; border-bottom: 1px solid var(--border);
  }
  .brand { display: flex; align-items: center; gap: var(--space-2); font-weight: 700; font-size: 16px; }
  .logo { font-size: 18px; }
  .tools { display: flex; gap: 6px; }
  .tool {
    width: 34px; height: 34px; border-radius: 9px; border: none; cursor: pointer;
    background: var(--surface-2); color: var(--text); font-size: 16px;
    display: flex; align-items: center; justify-content: center;
  }
  .tool:hover { background: var(--accent-weak); color: var(--accent); }
  .list { overflow-y: auto; padding: var(--space-2); display: flex; flex-direction: column; gap: var(--space-2); }
  .empty { text-align: center; color: var(--text-muted); padding: 48px 24px; }
  .empty-icon { font-size: 40px; opacity: .5; margin-bottom: var(--space-2); }
  .empty p { margin: 2px 0; }
  .empty-sub { font-size: 12px; }
</style>
