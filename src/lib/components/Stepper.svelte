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
    height: var(--button-height);
    line-height: 1;
    box-sizing: border-box;
  }

  button {
    min-width: var(--button-height);
    padding: 0;
    border: none;
    border-radius: var(--button-radius);
    background: var(--button-fill);
    box-shadow: inset 0 0 0 0.5px var(--button-stroke);
    color: var(--text);
    /* The − and + glyphs read a step larger than the row text. */
    font-size: calc(var(--type-row-label-size) + 2px);
  }

  button:hover:not(:disabled) {
    background: var(--button-fill-hover);
  }

  button:disabled {
    opacity: 0.4;
    background: var(--button-fill);
  }

  .reset {
    padding: 0 10px;
    font-size: var(--type-row-description-size);
    color: var(--text-secondary);
  }

  .value {
    min-width: 52px;
    font-size: var(--type-row-value-size);
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
</style>
