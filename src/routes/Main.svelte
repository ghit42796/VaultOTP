<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import AccountCard from "../components/AccountCard.svelte";
  import AddMenu from "../components/AddMenu.svelte";
  import Settings from "../components/Settings.svelte";
  import type { CodeView } from "../lib/types";
  import { currentCodes, removeAccount, lock, onTick, exportSecrets, reorderAccounts, type ExportFormat } from "../lib/ipc";
  import { loadSettings } from "../lib/settings";
  import { open, save } from "@tauri-apps/plugin-dialog";

  const dispatch = createEventDispatcher();
  let codes: CodeView[] = [];
  let unlisten: UnlistenFn | null = null;
  let showAdd = false;
  let showSettings = false;
  let idleTimer: number | undefined;
  const settings = loadSettings();

  let selectMode = false;
  let selected = new Set<string>();
  let exportStatus = "";
  let exportError = "";

  let reorderMode = false;
  let dragIndex: number | null = null;
  let dragging = false;

  async function refresh() { if (dragging) return; codes = await currentCodes(); }

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

  function toggleSelectMode() {
    selectMode = !selectMode;
    if (selectMode) { reorderMode = false; }
    if (!selectMode) {
      selected = new Set();
      exportStatus = "";
      exportError = "";
    }
  }

  function toggleReorderMode() {
    reorderMode = !reorderMode;
    if (reorderMode) { selectMode = false; selected = new Set(); }
    dragIndex = null;
    dragging = false;
  }

  function onDragStart(i: number) { dragIndex = i; dragging = true; }

  function onDragOver(e: DragEvent, i: number) {
    e.preventDefault();
    if (dragIndex === null || dragIndex === i) return;
    const arr = [...codes];
    const [moved] = arr.splice(dragIndex, 1);
    arr.splice(i, 0, moved);
    codes = arr;
    dragIndex = i;
  }

  async function persistOrder() {
    if (!dragging) return;
    dragging = false;
    dragIndex = null;
    try { await reorderAccounts(codes.map((c) => c.id)); }
    catch (_) { /* ignore; refresh restores the persisted order */ }
    await refresh();
  }

  function toggleSelect(id: string) {
    if (selected.has(id)) { selected.delete(id); } else { selected.add(id); }
    selected = selected; // reassign for Svelte reactivity
  }

  async function doExport(format: ExportFormat) {
    exportError = "";
    exportStatus = "";
    const ids = [...selected];
    if (!ids.length) { exportError = "Select at least one account."; return; }
    let path: string | null = null;
    if (format === "otpauth_qr") {
      const d = await open({ directory: true });
      path = typeof d === "string" ? d : null;
    } else {
      const def = format === "otpauth_text" ? "vaultotp-secrets.txt" : "vaultotp-migration.png";
      const f = await save({ defaultPath: def });
      path = typeof f === "string" ? f : null;
    }
    if (!path) return;
    try {
      const n = await exportSecrets(ids, path, format);
      exportStatus = `Exported ${n} account(s). WARNING: file contains PLAINTEXT secrets — keep it safe and delete it when done.`;
      selectMode = false;
      selected = new Set();
    } catch (e) {
      exportError = String(e);
    }
  }
</script>

<header>
  <div class="brand"><span class="logo">🔐</span> VaultOTP</div>
  <div class="tools">
    {#if !selectMode && !reorderMode}
      <button class="tool" on:click={() => (showAdd = true)} title="Add account" aria-label="Add account">＋</button>
      <button class="tool" on:click={doLock} title="Lock now" aria-label="Lock now">🔒</button>
      <button class="tool" on:click={() => (showSettings = true)} title="Settings" aria-label="Settings">⚙</button>
    {/if}
    {#if !reorderMode}
      <button class="tool" class:tool-active={selectMode} on:click={toggleSelectMode} title={selectMode ? "Cancel selection" : "Select accounts to export"} aria-label={selectMode ? "Cancel selection" : "Select accounts"}>
        {selectMode ? "✕" : "☑"}
      </button>
    {/if}
    {#if !selectMode}
      <button class="tool" class:tool-active={reorderMode} on:click={toggleReorderMode} title={reorderMode ? "Done reordering" : "Reorder accounts"} aria-label={reorderMode ? "Done reordering" : "Reorder accounts"}>
        {reorderMode ? "✓" : "⇅"}
      </button>
    {/if}
  </div>
</header>

{#if selectMode}
  <div class="export-bar">
    <span class="export-label">{selected.size} selected</span>
    <button class="vo-ghost" on:click={() => doExport("otpauth_qr")} title="Export one QR PNG per account to a folder">QR PNGs</button>
    <button class="vo-ghost" on:click={() => doExport("otpauth_text")} title="Export otpauth URIs as a text file">Text</button>
    <button class="vo-ghost" on:click={() => doExport("google_migration")} title="Export as Google Authenticator migration QR">Google</button>
  </div>
  {#if exportError}
    <div class="err export-msg">{exportError}</div>
  {/if}
{/if}

{#if exportStatus}
  <div class="hint export-msg export-warn">{exportStatus}</div>
{/if}

<div class="list">
  {#each codes as item, i (item.id)}
    {#if reorderMode}
<!-- svelte-ignore a11y-no-static-element-interactions -->
      <div
        class="drag-row"
        draggable="true"
        on:dragstart={() => onDragStart(i)}
        on:dragover={(e) => onDragOver(e, i)}
        on:drop={persistOrder}
        on:dragend={persistOrder}
      >
        <AccountCard {item} reorderMode={true} />
      </div>
    {:else}
      <AccountCard
        {item}
        {selectMode}
        selected={selected.has(item.id)}
        on:remove={(e) => remove(e.detail)}
        on:toggle={(e) => toggleSelect(e.detail)}
      />
    {/if}
  {/each}
  {#if codes.length === 0}
    <div class="empty">
      <div class="empty-icon">🔐</div>
      <p class="empty-title">No accounts yet</p>
      <p class="empty-sub">Press ＋ to add your first one — scan a QR, paste a key, or import.</p>
    </div>
  {/if}
</div>

{#if showAdd}
  <AddMenu on:close={() => { showAdd = false; refresh(); }} />
{/if}
{#if showSettings}
  <Settings on:close={() => (showSettings = false)} on:changed={refresh} on:switchVault={() => dispatch("switchVault")} />
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
  .tool-active { background: var(--accent-weak); color: var(--accent); }
  .list { overflow-y: auto; padding: var(--space-2); display: flex; flex-direction: column; gap: var(--space-2); }
  .empty { text-align: center; color: var(--text-muted); padding: 48px 24px; }
  .empty-icon { font-size: 40px; opacity: .5; margin-bottom: var(--space-2); }
  .empty p { margin: 2px 0; }
  .empty-title { font-weight: 600; color: var(--text); margin: 2px 0; }
  .empty-sub { font-size: 12px; }

  .drag-row { cursor: grab; }
  .drag-row:active { cursor: grabbing; }

  /* Export bar */
  .export-bar {
    display: flex; align-items: center; gap: var(--space-2);
    padding: var(--space-2) 18px; border-bottom: 1px solid var(--border);
    background: var(--surface-2);
  }
  .export-label { font-size: 12px; color: var(--text-muted); flex: 1; }
  .export-msg { margin: var(--space-1) 18px; font-size: 12px; }
  .export-warn { color: var(--text-muted); }

  /* Status / error messages */
  .hint { font-size: 12px; color: var(--text-muted); margin: 0; }
  .err { font-size: 12px; color: var(--error, #e53e3e); margin: 0; }
</style>
