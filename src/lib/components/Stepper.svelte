<script lang="ts">
  // − value + [Reset]; every part shares one height and centres its glyph (spec 009 FR-040).
  let {
    value,
    canShrink,
    canEnlarge,
    onshrink,
    onenlarge,
    onreset,
    smallerLabel,
    largerLabel,
    resetLabel = "",
  }: {
    value: string;
    canShrink: boolean;
    canEnlarge: boolean;
    onshrink: () => void;
    onenlarge: () => void;
    onreset?: () => void;
    smallerLabel: string;
    largerLabel: string;
    resetLabel?: string;
  } = $props();
</script>

<div class="stepper">
  <button type="button" aria-label={smallerLabel} disabled={!canShrink} onclick={onshrink}>−</button
  >
  <span class="value">{value}</span>
  <button type="button" aria-label={largerLabel} disabled={!canEnlarge} onclick={onenlarge}
    >+</button
  >
  {#if onreset}
    <button type="button" class="reset" onclick={onreset}>{resetLabel}</button>
  {/if}
</div>

<style>
  .stepper {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  button,
  .value {
    display: grid;
    place-items: center;
    height: var(--control-height);
    line-height: 1;
    box-sizing: border-box;
  }

  button {
    min-width: var(--control-height);
    padding: 0;
    border: none;
    border-radius: 6px;
    background: var(--surface-opaque);
    box-shadow: inset 0 0 0 1px var(--stroke);
    color: var(--text);
    font-size: 15px;
  }

  button:disabled {
    opacity: 0.4;
  }

  .reset {
    padding: 0 10px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .value {
    min-width: 52px;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
</style>
