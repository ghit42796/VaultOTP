<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import VaultPicker from "./routes/VaultPicker.svelte";
  import Unlock from "./routes/Unlock.svelte";
  import Main from "./routes/Main.svelte";
  import { isUnlocked, onLocked } from "./lib/ipc";
  import type { UnlistenFn } from "@tauri-apps/api/event";

  type View = "picker" | "unlock" | "main";
  let view: View = "picker";
  let unlistenLocked: UnlistenFn | undefined;

  onMount(async () => {
    if (await isUnlocked()) view = "main";
    unlistenLocked = await onLocked(() => { view = "unlock"; });
  });
  onDestroy(() => { unlistenLocked?.(); });
</script>

{#if view === "main"}
  <Main on:locked={() => (view = "unlock")} on:switchVault={() => (view = "picker")} />
{:else if view === "unlock"}
  <Unlock on:unlocked={() => (view = "main")} on:switch={() => (view = "picker")} />
{:else}
  <VaultPicker on:selected={() => (view = "unlock")} />
{/if}
