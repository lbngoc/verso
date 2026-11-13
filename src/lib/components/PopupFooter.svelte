<script lang="ts">
  import Icon from "./Icon.svelte";
  import Keycap from "./Keycap.svelte";
  import { t } from "$lib/i18n";

  // Footer of the result popup and the quick panel: Copy (c) / Speak (s), plus one notice line.
  let {
    copied = false,
    speaking = false,
    canSpeak = true,
    notice = "",
    hint = "",
    oncopy,
    onspeak,
  }: {
    copied?: boolean;
    speaking?: boolean;
    canSpeak?: boolean;
    notice?: string;
    hint?: string;
    oncopy: () => void;
    onspeak: () => void;
  } = $props();
</script>

<footer class="popup-footer">
  <button class="action" type="button" onclick={oncopy}>
    <Icon name="copy" />
    <span>{copied ? $t("popup.copied") : $t("popup.copy")}</span>
    <Keycap label="c" />
  </button>
  <button class="action" type="button" disabled={!canSpeak} onclick={onspeak}>
    <Icon name="speak" />
    <span>{speaking ? $t("popup.speaking") : $t("popup.speak")}</span>
    <Keycap label="s" />
  </button>
  <span class="spacer"></span>
  {#if notice}
    <span class="notice" title={notice}>{notice}</span>
  {:else if hint}
    <span class="hint">{hint}</span>
  {/if}
</footer>

<style>
  .popup-footer {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 36px;
    padding: 0 10px;
    border-top: 1px solid var(--separator);
    flex: none;
  }

  .action {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 8px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text);
    font-size: 12px;
  }

  .action:hover:not(:disabled) {
    background: var(--hover);
  }

  .action:disabled {
    color: var(--text-tertiary);
  }

  .spacer {
    flex: 1;
  }

  .notice {
    max-width: 55%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .hint {
    font-size: 11px;
    color: var(--text-tertiary);
  }
</style>
