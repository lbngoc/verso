<script lang="ts">
  import type { Snippet } from "svelte";

  // One settings row (spec 011 R3, contracts/settings-look.md): label and, under it, a description
  // on the left; the control on the right, centred on the label's FIRST line, never on a wrapped
  // block. `block` rows (a permission) put more content under the head; the control stays top right.
  let {
    label,
    description = "",
    kind = "inline",
    children,
    content,
  }: {
    label: string;
    description?: string;
    kind?: "inline" | "block";
    /** The control: switch, menu, stepper, recorder, value text or button(s). */
    children?: Snippet;
    /** Only for `block`: what sits under the head (status, steps, a guide). */
    content?: Snippet;
  } = $props();
</script>

<div class="setting-row" class:block={kind === "block"} data-row={kind}>
  <div class="label-column">
    <span class="label">{label}</span>
    {#if description}<span class="description">{description}</span>{/if}
  </div>
  {#if children}
    <div class="control">{@render children()}</div>
  {/if}
  {#if kind === "block" && content}
    <div class="content">{@render content()}</div>
  {/if}
</div>

<style>
  .setting-row {
    /* The label's line box; the control column has this height and lets taller controls overflow
       evenly above and below, so they centre on the first line. */
    --row-line: calc(var(--type-row-label-size) * 1.4);
    --row-pad: calc((var(--row-min-height) - var(--row-line)) / 2);
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: 16px;
    align-items: start;
    min-height: var(--row-min-height);
    padding-block: var(--row-pad);
  }

  /* A row with a push button is taller, so the button keeps clear of the separators (44 pt on macOS). */
  .setting-row:has(.control :global(.button:not(.link))) {
    padding-block: calc(var(--row-pad) + 4px);
  }

  .label-column {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .label {
    font-size: var(--type-row-label-size);
    font-weight: var(--type-row-label-weight);
    line-height: var(--row-line);
    color: var(--text);
    overflow-wrap: anywhere;
  }

  .description {
    font-size: var(--type-row-description-size);
    font-weight: var(--type-row-description-weight);
    line-height: 1.45;
    color: var(--text-secondary);
    overflow-wrap: anywhere;
  }

  .control {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    height: var(--row-line);
    font-size: var(--type-row-value-size);
    font-weight: var(--type-row-value-weight);
    color: var(--text);
    white-space: nowrap;
  }

  .content {
    grid-column: 1 / -1;
    min-width: 0;
    padding-top: 8px;
  }
</style>
