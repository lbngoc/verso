<script lang="ts">
  import type { Snippet } from "svelte";

  // One group of settings rows (spec 011 R3, contracts/settings-look.md): the heading and its helper
  // sit ABOVE the card, as in System Settings and Windows Settings; group actions go in the footer,
  // bottom right inside the card.
  let {
    title = "",
    helper = "",
    children,
    footer,
  }: { title?: string; helper?: string; children?: Snippet; footer?: Snippet } = $props();
</script>

<section class="group" class:headed={!!title}>
  {#if title}
    <header>
      <h2>{title}</h2>
      {#if helper}<p class="helper">{helper}</p>{/if}
    </header>
  {/if}
  <div class="card">
    {#if children}
      <div class="rows">
        {@render children()}
      </div>
    {/if}
    {#if footer}
      <div class="footer" class:alone={!children}>{@render footer()}</div>
    {/if}
  </div>
</section>

<style>
  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  /* A headed group stands further from the box above it, as in System Settings. */
  .group.headed:not(:first-child) {
    margin-top: var(--group-heading-space);
  }

  header {
    padding: 0 var(--card-padding-x);
  }

  /* Windows Settings aligns group headings with the card edge. */
  :global([data-os="windows"]) header {
    padding: 0 2px;
  }

  h2 {
    margin: 0;
    font-size: var(--type-group-heading-size);
    font-weight: var(--type-group-heading-weight);
    color: var(--text);
  }

  .helper {
    margin: 2px 0 0;
    font-size: var(--type-caption-size);
    font-weight: var(--type-caption-weight);
    line-height: 1.35;
    color: var(--text-secondary);
  }

  .card {
    background: var(--card-fill);
    border-radius: var(--card-radius);
    padding: 0 var(--card-padding-x);
  }

  :global([data-os="windows"]) .card {
    box-shadow: inset 0 0 0 1px var(--stroke);
  }

  /* High contrast and the opaque fallback keep a visible edge (accessibility contract). */
  :global([data-high-contrast="true"]) .card,
  :global([data-material="opaque"][data-theme="light"]) .card {
    box-shadow: inset 0 0 0 1px var(--stroke);
  }

  .rows {
    display: flex;
    flex-direction: column;
  }

  /* Rows are separated by hairlines. */
  .rows > :global(* + *) {
    border-top: 1px solid var(--separator);
  }

  /* Anything that is not a SettingRow still gets the row height and a one-line layout. */
  /* :where() keeps this default weak, so a component's own layout for its child wins. */
  .rows > :global(:where(:not(.setting-row))) {
    min-height: var(--row-min-height);
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .footer {
    display: flex;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 8px;
    padding: 4px 0 12px;
  }

  .footer.alone {
    padding-top: 12px;
  }
</style>
