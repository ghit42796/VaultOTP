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
  <h1>VaultOTP</h1>
  <div class="actions">
    <button on:click={() => (showAdd = true)} title="Add">＋</button>
    <button on:click={doLock} title="Lock now">🔒</button>
    <button on:click={() => (showSettings = true)} title="Settings">⚙</button>
  </div>
</header>

<div class="list">
  {#each codes as item (item.id)}
    <AccountCard {item} on:remove={(e) => remove(e.detail)} />
  {/each}
  {#if codes.length === 0}<p class="empty">No accounts yet. Press ＋ to add one.</p>{/if}
</div>

{#if showAdd}
  <AddMenu on:close={() => { showAdd = false; refresh(); }} />
{/if}

{#if showSettings}
  <Settings on:close={() => (showSettings = false)} on:changed={refresh} />
{/if}

<style>
  header { display: flex; align-items: center; justify-content: space-between; padding: 10px 14px; border-bottom: 1px solid #ddd; }
  h1 { font-size: 16px; margin: 0; }
  .actions button { font-size: 18px; background: none; border: none; cursor: pointer; margin-left: 8px; }
  .list { overflow-y: auto; }
  .empty { text-align: center; color: #999; padding: 32px; }
</style>
