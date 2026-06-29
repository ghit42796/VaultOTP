<script lang="ts">
  import type { CodeView } from "../lib/types";
  import { createEventDispatcher } from "svelte";
  import { loadSettings } from "../lib/settings";
  import { initial, badgeColor, groupCode, ringCircumference, ringDashoffset } from "../lib/display";

  export let item: CodeView;
  const dispatch = createEventDispatcher();
  const settings = loadSettings();
  let copied = false;

  const PERIOD = 30;
  const R = 17;
  const C = ringCircumference(R);
  $: offset = ringDashoffset(item.remaining, PERIOD, R);
  $: warn = item.remaining <= 5;

  async function copy() {
    await navigator.clipboard.writeText(item.code);
    copied = true;
    setTimeout(() => (copied = false), 1500);
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

<div class="card">
  <div class="badge" style="background:{badgeColor(item.issuer)}">{initial(item.issuer)}</div>

  <button class="main" on:click={copy} title="Copy code">
    <span class="issuer">{item.issuer || "—"}</span>
    <span class="label">{item.label}</span>
    <span class="code">{groupCode(item.code)}</span>
  </button>

  {#if copied}<span class="toast">Copied ✓</span>{/if}

  <div class="ring" title="{item.remaining}s remaining">
    <svg width="40" height="40" viewBox="0 0 40 40">
      <circle class="track" cx="20" cy="20" r={R} stroke-width="3.5" fill="none" />
      <circle class="fill" class:warn cx="20" cy="20" r={R} stroke-width="3.5"
        fill="none" stroke-linecap="round"
        stroke-dasharray={C} stroke-dashoffset={offset} transform="rotate(-90 20 20)" />
    </svg>
    <span class="num" class:warn>{item.remaining}</span>
  </div>

  <button class="del" on:click={() => dispatch("remove", item.id)} title="Delete" aria-label="Delete">🗑</button>
</div>

<style>
  .card {
    display: flex; align-items: center; gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: var(--surface); border: 1px solid var(--border);
    border-radius: var(--radius); position: relative;
  }
  .card:hover { border-color: var(--accent); }
  .badge {
    width: 40px; height: 40px; border-radius: 11px; flex: 0 0 auto;
    display: flex; align-items: center; justify-content: center;
    font-weight: 700; font-size: 17px; color: #fff;
  }
  .main {
    flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px;
    background: none; border: none; padding: 0; cursor: pointer; text-align: left;
  }
  .issuer { font-weight: 600; color: var(--text); }
  .label { font-size: 12px; color: var(--text-muted);
           overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .code { font-family: var(--font-mono); font-size: 22px; font-weight: 600;
          letter-spacing: 3px; color: var(--accent); margin-top: 2px; }
  .ring { position: relative; width: 40px; height: 40px; flex: 0 0 auto; }
  .track { stroke: var(--ring-track); }
  .fill { stroke: var(--ring-fill); transition: stroke-dashoffset .3s linear; }
  .fill.warn { stroke: var(--danger); }
  .num { position: absolute; inset: 0; display: flex; align-items: center;
         justify-content: center; font-size: 12px; font-weight: 600; color: var(--text-muted); }
  .num.warn { color: var(--danger); }
  .toast { position: absolute; right: 56px; top: 8px; font-size: 11px;
           color: var(--success); background: var(--accent-weak);
           padding: 2px 8px; border-radius: 20px; }
  .del { background: none; border: none; cursor: pointer; font-size: 14px;
         opacity: 0; transition: opacity .12s; color: var(--text-muted); }
  .card:hover .del { opacity: .8; }
</style>
