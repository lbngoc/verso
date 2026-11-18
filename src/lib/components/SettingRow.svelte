<script lang="ts">
  import type { Snippet } from "svelte";

  // Label (and optional helper) on the left, the control on the right, centred on one line.
  let {
    label,
    helper = "",
    children,
  }: { label: string; helper?: string; children?: Snippet } = $props();
</script>

<div class="setting-row">
  <div class="text">
    <span class="label">{label}</span>
    {#if helper}<span class="helper">{helper}</span>{/if}
  </div>
  <div class="control">{@render children?.()}</div>
</div>

<style>
  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 36px;
    padding: 4px 0;
    font-size: 13px;
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .helper {
    font-size: 11px;
    line-height: 1.35;
    color: var(--text-secondary);
  }

  .control {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
  }
</style>
