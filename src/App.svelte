<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import Unlock from "./routes/Unlock.svelte";
  import Main from "./routes/Main.svelte";
  import { isUnlocked, onLocked } from "./lib/ipc";
  import type { UnlistenFn } from "@tauri-apps/api/event";

  let unlocked = false;
  let unlistenLocked: UnlistenFn | undefined;

  onMount(async () => {
    unlocked = await isUnlocked();
    unlistenLocked = await onLocked(() => { unlocked = false; });
  });

  onDestroy(() => { unlistenLocked?.(); });
</script>

{#if unlocked}
  <Main on:locked={() => (unlocked = false)} />
{:else}
  <Unlock on:unlocked={() => (unlocked = true)} />
{/if}
