<script lang="ts">
  import Icon from "./Icon.svelte";
  import { t as translator } from "$lib/i18n";

  let {
    canCopy = true,
    canSpeak = true,
    voiceAvailable = null,
    copied = false,
    language = "",
    oncopy,
    onspeak,
  }: {
    canCopy?: boolean;
    canSpeak?: boolean;
    voiceAvailable?: boolean | null;
    copied?: boolean;
    language?: string;
    oncopy: () => void;
    onspeak: () => void;
  } = $props();

  const speakTitle = $derived(
    voiceAvailable === false
      ? $translator("popup.no_voice", { language: language.toUpperCase() })
      : $translator("popup.speak"),
  );
</script>

<div class="actions">
  <button class="icon-button" onclick={oncopy} disabled={!canCopy} title={$translator("popup.copy")}>
    <Icon name={copied ? "check" : "copy"} />
    <span>{copied ? $translator("popup.copied") : $translator("popup.copy")}</span>
  </button>
  <button
    class="icon-button"
    onclick={onspeak}
    disabled={!canSpeak || voiceAvailable === false}
    title={speakTitle}
  >
    <Icon name="speak" />
    <span>{$translator("popup.speak")}</span>
  </button>
</div>

<style>
  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }
</style>
