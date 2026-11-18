<script lang="ts">
  import Icon from "./Icon.svelte";
  import { t } from "$lib/i18n";
  import { canEnlarge, canShrink, clampStep, label } from "$lib/textSize";

  // Header of the result popup: direction on the left, engine, then pin / text size / close.
  let {
    direction,
    engine = "",
    pinned = false,
    step = 0,
    onpin,
    onsize,
    onclose,
  }: {
    direction: string;
    engine?: string;
    pinned?: boolean;
    step?: number;
    onpin: (pinned: boolean) => void;
    onsize: (delta: number) => void;
    onclose: () => void;
  } = $props();

  const smallerTip = $derived(
    canShrink(step)
      ? $t("popup.text_smaller_to", { size: label(clampStep(step - 1)) })
      : $t("popup.text_smaller"),
  );
  const largerTip = $derived(
    canEnlarge(step)
      ? $t("popup.text_larger_to", { size: label(clampStep(step + 1)) })
      : $t("popup.text_larger"),
  );
</script>

<header class="popup-header">
  <span class="direction">{direction}</span>
  {#if engine}<span class="engine">{engine}</span>{/if}
  <span class="spacer"></span>
  <button
    class="icon"
    class:pinned
    type="button"
    aria-pressed={pinned}
    title={pinned ? $t("popup.unpin") : $t("popup.pin")}
    aria-label={pinned ? $t("popup.unpin") : $t("popup.pin")}
    onclick={() => onpin(!pinned)}
  >
    <Icon name={pinned ? "pin-filled" : "pin"} />
  </button>
  <button
    class="icon"
    type="button"
    title={smallerTip}
    aria-label={$t("popup.text_smaller")}
    disabled={!canShrink(step)}
    onclick={() => onsize(-1)}
  >
    <Icon name="minus" />
  </button>
  <button
    class="icon"
    type="button"
    title={largerTip}
    aria-label={$t("popup.text_larger")}
    disabled={!canEnlarge(step)}
    onclick={() => onsize(1)}
  >
    <Icon name="plus" />
  </button>
  <button
    class="icon"
    type="button"
    title={$t("popup.esc_hint")}
    aria-label={$t("popup.close")}
    onclick={onclose}
  >
    <Icon name="close" />
  </button>
</header>

<style>
  .popup-header {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 8px 0 12px;
    border-bottom: 1px solid var(--separator);
    flex: none;
  }

  .direction {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    color: var(--text);
  }

  .engine {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .spacer {
    flex: 1;
  }

  .icon {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
  }

  .icon:hover {
    background: var(--hover);
    color: var(--text);
  }

  .icon:disabled {
    opacity: 0.35;
    pointer-events: none;
  }

  .icon.pinned {
    color: var(--accent-text);
  }
</style>
