<script lang="ts">
  import { t as translator } from "$lib/i18n";
  import type { CaptureFailure } from "$lib/ipc";

  let {
    failure,
    terminalModifier = null,
    ongrant,
    onrecheck,
    onquick,
  }: {
    failure: CaptureFailure;
    terminalModifier?: "option" | "shift" | null;
    ongrant: () => void;
    onrecheck: () => void;
    onquick?: () => void;
  } = $props();

  const messageKey = $derived(`capture.${failure}`);
  const modifierLabel = $derived(
    terminalModifier === "shift"
      ? $translator("capture.terminal_shift")
      : terminalModifier === "option"
        ? $translator("capture.terminal_option")
        : "",
  );
</script>

<div class="capture-failed" role="alert">
  {#if failure === "terminal_tui"}
    <p>{$translator(messageKey, { modifier: modifierLabel })}</p>
  {:else}
    <p>{$translator(messageKey)}</p>
  {/if}

  {#if failure === "nothing_selected" || failure === "secure_field"}
    {#if onquick}
      <button class="primary" type="button" onclick={onquick}>{$translator("tray.quick")}</button>
    {/if}
  {/if}

  {#if failure === "permission_missing"}
    <div class="row">
      <button class="primary" type="button" onclick={ongrant}>
        {$translator("capture.grant")}
      </button>
      <button type="button" onclick={onrecheck}>{$translator("capture.recheck")}</button>
    </div>
    <p class="hint">{$translator("capture.stale_hint")}</p>
  {/if}
</div>

<style>
  .capture-failed {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 12px;
  }

  p {
    margin: 0;
    color: var(--text-secondary);
  }

  .row {
    display: flex;
    gap: 8px;
  }

  .primary {
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
  }

  .hint {
    font-size: 11px;
    color: var(--text-tertiary);
  }
</style>
