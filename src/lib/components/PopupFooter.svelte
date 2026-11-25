<script lang="ts">
  import Keycap from "./Keycap.svelte";
  import { t } from "$lib/i18n";
  import type { FooterStatus } from "$lib/footerStatus";

  // The status bar of the result popup and the quick panel (spec 004 FR-034): on the left, always,
  // the Copy (c) and Speak (s) shortcuts; on the right, what applies right now (a notice, the
  // auto-translate mode or the Enter tip, the close key). Small text so more fits.

  let {
    copied = false,
    speaking = false,
    canSpeak = true,
    canCopy = true,
    status = null,
    keysActive = true,
    oncopy,
    onspeak,
  }: {
    copied?: boolean;
    speaking?: boolean;
    canSpeak?: boolean;
    canCopy?: boolean;
    status?: FooterStatus | null;
    /** Whether the c and s keys act right now (in the panel only while the result has focus). */
    keysActive?: boolean;
    oncopy: () => void;
    onspeak: () => void;
  } = $props();
</script>

<footer class="popup-footer">
  <button class="action" type="button" disabled={!canCopy} onclick={oncopy}>
    <span>{copied ? $t("popup.copied") : $t("popup.copy")}</span>
    <Keycap label="c" active={keysActive && canCopy} muted={!keysActive || !canCopy} />
  </button>
  <button class="action" type="button" disabled={!canSpeak} onclick={onspeak}>
    <span>{speaking ? $t("popup.speaking") : $t("popup.speak")}</span>
    <Keycap label="s" active={keysActive && canSpeak} muted={!keysActive || !canSpeak} />
  </button>
  <span class="spacer"></span>
  {#if status}
    {#if status.onclick}
      <button
        class="status status-button"
        type="button"
        title={status.title}
        onclick={status.onclick}
      >
        {#if status.key}<Keycap label={status.key} />{/if}
        <span class="status-text">{status.text}</span>
      </button>
    {:else}
      <span class="status" title={status.title ?? status.text}>
        {#if status.key}<Keycap label={status.key} />{/if}
        <span class="status-text">{status.text}</span>
      </span>
    {/if}
  {/if}
</footer>

<style>
  .popup-footer {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 32px;
    padding: 0 10px;
    border-top: 1px solid var(--separator);
    flex: none;
    font-size: 11px;
  }

  .action {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 22px;
    padding: 0 6px;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--text-secondary);
    font: inherit;
  }

  .action:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }

  .action:disabled {
    color: var(--text-tertiary);
  }

  .spacer {
    flex: 1;
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    max-width: 60%;
    color: var(--text-secondary);
  }

  .status-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-button {
    height: 22px;
    padding: 0 6px;
    border: none;
    border-radius: 5px;
    background: transparent;
    font: inherit;
    cursor: pointer;
  }

  .status-button:hover {
    background: var(--hover);
    color: var(--text);
  }
</style>
