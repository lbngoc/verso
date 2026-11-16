<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import Keycap from "$lib/components/Keycap.svelte";
  import { t } from "$lib/i18n";
  import { commands, type SettingsProfile } from "$lib/ipc";

  // Settings → Input → Popup & panel: shared text size and the key reference (FR-005).
  let {
    settings,
    onchange,
  }: { settings: SettingsProfile; onchange: (profile: SettingsProfile) => void } = $props();

  let step = $state(0);
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  // Follow the profile when it changes elsewhere (popup keys, another window).
  $effect(() => {
    step = settings.text_size_step ?? 0;
  });

  // Result font: 13/14/16/18/20/22/24 px, one point larger on macOS (R11).
  const mac = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");
  const previewSize = $derived.by(() => {
    const sizes = [13, 14, 16, 18, 20, 22, 24];
    return sizes[Math.min(Math.max(step + 2, 0), sizes.length - 1)] + (mac ? 1 : 0);
  });

  function setStep(next: number): void {
    step = Math.min(4, Math.max(-2, next));
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      void commands
        .setTextSizeStep(step)
        .then(onchange)
        .catch(() => undefined);
    }, 400);
  }

  $effect(() => () => {
    if (saveTimer) clearTimeout(saveTimer);
  });
</script>

<div class="stack">
  <Card title={$t("settings.card_text_size")}>
    <div class="row">
      <span>{$t("settings.card_text_size")}</span>
      <div class="sizer">
        <button
          type="button"
          aria-label={$t("popup.text_smaller")}
          onclick={() => setStep(step - 1)}>−</button
        >
        <span class="value">{step > 0 ? `+${step}` : step}</span>
        <button type="button" aria-label={$t("popup.text_larger")} onclick={() => setStep(step + 1)}
          >+</button
        >
        <button type="button" class="reset" onclick={() => setStep(0)}
          >{$t("settings.text_size_reset")}</button
        >
      </div>
    </div>
    <div class="row preview" style:font-size={`${previewSize}px`}>
      {$t("settings.text_size_preview")}
    </div>
  </Card>

  <Card title={$t("settings.card_keys")}>
    <div class="keys">
      <span><Keycap label="c" /> {$t("popup.copy")}</span>
      <span><Keycap label="s" /> {$t("popup.speak")}</span>
      <span
        ><Keycap label="−" />
        <Keycap label="=" />
        <Keycap label="0" />
        {$t("settings.card_text_size")}</span
      >
      <span><Keycap label="Tab" /> {$t("quick.swap")}</span>
      <span><Keycap label="Esc" /> {$t("popup.close")}</span>
    </div>
  </Card>
</div>

<style>
  .stack {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 13px;
  }

  .sizer {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .sizer button {
    width: 26px;
    height: 26px;
    border: none;
    border-radius: 6px;
    background: var(--surface-opaque);
    box-shadow: inset 0 0 0 1px var(--stroke);
    color: var(--text);
  }

  .sizer .reset {
    width: auto;
    padding: 0 8px;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .value {
    min-width: 24px;
    text-align: center;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .preview {
    justify-content: flex-start;
    color: var(--text);
  }

  .keys {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 18px;
    padding: 8px 0;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .keys span {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
</style>
