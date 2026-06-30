<script lang="ts">
  import type { CodeView } from "../lib/types";
  import { createEventDispatcher } from "svelte";
  import { loadSettings } from "../lib/settings";
  import { initial, badgeColor, groupCode, barFraction } from "../lib/display";

  export let item: CodeView;
  export let selectMode = false;
  export let selected = false;
  export let reorderMode = false;

  const dispatch = createEventDispatcher();
  const settings = loadSettings();
  let copied = false;

  const PERIOD = 30;
  $: fraction = barFraction(item.remaining, PERIOD);
  $: warn = item.remaining <= 5;

  async function activate() {
    if (reorderMode) return;
    if (selectMode) { dispatch("toggle", item.id); return; }
    await copy();
  }

  async function copy() {
    await navigator.clipboard.writeText(item.code);
    copied = true;
    setTimeout(() => (copied = false), 1200);
    const ms = settings.clipboardClearMs;
    if (ms > 0) {
      setTimeout(async () => {
        try {
          const current = await navigator.clipboard.readText();
          if (current === item.code) await navigator.clipboard.writeText("");
        } catch (_) { /* clipboard read may be blocked; ignore */ }
      }, ms);
    }
  }
</script>

<div class="card" class:selected class:copied>
  <button
    class="main"
    on:click={activate}
    aria-pressed={selectMode ? selected : undefined}
    title={reorderMode ? "Drag to reorder" : selectMode ? "Toggle selection" : "Copy code"}
  >
    {#if reorderMode}<span class="handle" aria-hidden="true">≡</span>{/if}
    {#if selectMode}<span class="check" class:on={selected} aria-hidden="true"></span>{/if}
    <span class="badge" style="background:{badgeColor(item.issuer)}">{initial(item.issuer)}</span>
    <span class="mid">
      <span class="issuer">{item.issuer || "—"}</span>
      <span class="label">{item.label}</span>
    </span>
    <span class="right">
      <span class="code" class:warn>{groupCode(item.code)}</span>
      {#if copied}
        <span class="secs ok">Copied ✓</span>
      {:else}
        <span class="secs" class:warn>{item.remaining}s</span>
      {/if}
    </span>
  </button>

  {#if !selectMode && !reorderMode}
    <button class="del" on:click|stopPropagation={() => dispatch("remove", item.id)} title="Delete" aria-label="Delete">🗑</button>
  {/if}

  <span class="bar"><i class:warn style="width:{fraction * 100}%"></i></span>
</div>

<style>
  .card {
    position: relative; background: var(--surface);
    border: 1px solid var(--border); border-radius: var(--radius);
    transition: border-color .12s, background .25s, box-shadow .12s;
  }
  .card:hover { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-weak); }
  .card.selected, .card.copied { border-color: var(--accent); background: var(--accent-weak); }

  .main {
    width: 100%; display: flex; align-items: center; gap: var(--space-3);
    padding: 11px 13px 14px; background: none; border: none; cursor: pointer;
    text-align: left; border-radius: var(--radius);
  }

  .handle { flex: 0 0 auto; color: var(--text-muted); font-size: 16px; line-height: 1; cursor: grab; }

  .check {
    width: 18px; height: 18px; border-radius: 6px; border: 2px solid var(--border);
    flex: 0 0 auto; position: relative;
  }
  .check.on { background: var(--accent); border-color: var(--accent); }
  .check.on::after {
    content: "✓"; color: #fff; font-size: 11px; position: absolute; inset: 0;
    display: flex; align-items: center; justify-content: center;
  }

  .badge {
    width: 34px; height: 34px; border-radius: 10px; flex: 0 0 auto;
    display: flex; align-items: center; justify-content: center;
    font-weight: 700; font-size: 15px; color: #fff;
  }
  .mid { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .issuer { font-weight: 600; color: var(--text); }
  .label { font-size: 11px; color: var(--text-muted);
           overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .right { display: flex; flex-direction: column; align-items: flex-end; gap: 1px; flex: 0 0 auto; }
  .code { font-family: var(--font-mono); font-size: 22px; font-weight: 600;
          letter-spacing: 4px; color: var(--accent); }
  .code.warn { color: var(--danger); }
  .secs { font-size: 10px; color: var(--text-muted); }
  .secs.warn { color: var(--danger); }
  .secs.ok { color: var(--success); font-weight: 600; }

  .bar { position: absolute; left: 13px; right: 13px; bottom: 6px; height: 3px;
         border-radius: 3px; background: var(--ring-track); overflow: hidden; }
  .bar > i { display: block; height: 100%; background: var(--accent); transition: width .3s linear; }
  .bar > i.warn { background: var(--danger); }

  .del { position: absolute; top: 6px; right: 8px; background: none; border: none;
         cursor: pointer; font-size: 13px; color: var(--text-muted);
         opacity: 0; transition: opacity .12s; }
  .card:hover .del { opacity: .7; }
</style>
