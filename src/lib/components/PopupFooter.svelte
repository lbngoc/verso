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
    keys = [],
    keysActive = true,
    oncopy,
    onspeak,
  }: {
    copied?: boolean;
    speaking?: boolean;
    canSpeak?: boolean;
    notice?: string;
    hint?: string;
    /** Keys worth showing next to the actions, e.g. Enter and Esc in the quick panel. */
    keys?: Array<{ label: string; text: string; active?: boolean }>;
    /** Whether the c and s keys act right now (in the panel only while the result has focus). */
    keysActive?: boolean;
    oncopy: () => void;
    onspeak: () => void;
  } = $props();
</script>

<footer class="popup-footer">
  <button class="action" type="button" onclick={oncopy}>
    <Icon name="copy" />
    <span>{copied ? $t("popup.copied") : $t("popup.copy")}</span>
    <Keycap label="c" active={keysActive} muted={!keysActive} />
  </button>
  <button class="action" type="button" disabled={!canSpeak} onclick={onspeak}>
    <Icon name="speak" />
    <span>{speaking ? $t("popup.speaking") : $t("popup.speak")}</span>
    <Keycap label="s" active={keysActive && canSpeak} muted={!keysActive || !canSpeak} />
  </button>
  <span class="spacer"></span>
  {#if notice}
    <span class="notice" title={notice}>{notice}</span>
  {:else if keys.length}
    <span class="keys">
      {#each keys as key (key.label)}
        <span class="key">
          <Keycap label={key.label} active={key.active ?? false} />
          <span>{key.text}</span>
        </span>
      {/each}
    </span>
  {:else if hint}
    <span class="hint">{hint}</span>
  {/if}
</footer>

<style>
  .popup-footer {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 40px;
    padding: 0 12px;
    border-top: 1px solid var(--separator);
    flex: none;
  }

  .action {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
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

  .keys {
    display: inline-flex;
    align-items: center;
    gap: 12px;
  }

  .key {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--text-secondary);
  }
</style>
