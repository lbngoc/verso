<script lang="ts">
  import type { Snippet } from "svelte";

  // One group of settings rows (spec 009, contracts/settings-window.md § Sidebar).
  let {
    title = "",
    helper = "",
    children,
  }: { title?: string; helper?: string; children?: Snippet } = $props();
</script>

<section class="card">
  {#if title}
    <header>
      <h2>{title}</h2>
      {#if helper}<p class="helper">{helper}</p>{/if}
    </header>
  {/if}
  <div class="rows">
    {@render children?.()}
  </div>
</section>

<style>
  .card {
    background: var(--surface-raised);
    border-radius: 12px;
    padding: 4px 14px;
  }

  /* High contrast and the opaque fallback keep a visible edge (accessibility contract). */
  :global([data-high-contrast="true"]) .card,
  :global([data-material="opaque"][data-theme="light"]) .card {
    box-shadow: inset 0 0 0 1px var(--stroke);
  }

  header {
    padding: 10px 0 2px;
  }

  h2 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .helper {
    margin: 2px 0 0;
    font-size: 11px;
    line-height: 1.35;
    color: var(--text-secondary);
  }

  .rows {
    display: flex;
    flex-direction: column;
  }

  /* Rows are separated by hairlines and never shorter than 36 pt. */
  .rows > :global(*) {
    min-height: 36px;
    display: flex;
    align-items: center;
    gap: 10px;
    border-top: 1px solid var(--separator);
  }

  /* A row with a head and content below it (a permission) stacks and aligns left like the rest. */
  .rows > :global([data-row="block"]) {
    flex-direction: column;
    align-items: stretch;
  }

  .rows > :global(*:first-child) {
    border-top: none;
  }
</style>
