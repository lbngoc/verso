<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import Keycap from "$lib/components/Keycap.svelte";
  import SettingRow from "$lib/components/SettingRow.svelte";
  import Stepper from "$lib/components/Stepper.svelte";
  import { t } from "$lib/i18n";
  import { commands, type AutoTranslate, type SettingsProfile } from "$lib/ipc";
  import { canEnlarge, canShrink, clampStep, fontPx, label } from "$lib/textSize";

  // Settings → Input → Popup & panel: shared text size and the key reference (FR-038, FR-039).
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

  const mac = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");
  const previewSize = $derived(fontPx(step, mac));

  function setStep(next: number): void {
    step = clampStep(next);
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

  // Translate as you type in the panel (spec 004 FR-033).
  const AUTO_MODES: AutoTranslate[] = ["fast", "slow", "off"];

  async function setAutoTranslate(mode: AutoTranslate): Promise<void> {
    const saved = await commands.updateSettings({ auto_translate: mode }).catch(() => null);
    if (saved) onchange(saved);
  }
</script>

<div class="stack">
  <Card title={$t("settings.card_typing")}>
    <SettingRow label={$t("settings.auto_translate")} helper={$t("settings.auto_translate_hint")}>
      <select
        value={settings.auto_translate}
        aria-label={$t("settings.auto_translate")}
        onchange={(event) => void setAutoTranslate(event.currentTarget.value as AutoTranslate)}
      >
        {#each AUTO_MODES as mode (mode)}
          <option value={mode}>{$t(`settings.auto_translate_${mode}`)}</option>
        {/each}
      </select>
    </SettingRow>
  </Card>

  <Card title={$t("settings.card_text_size")}>
    <SettingRow label={$t("settings.text_size_row")}>
      <Stepper
        value={label(step)}
        canShrink={canShrink(step)}
        canEnlarge={canEnlarge(step)}
        onshrink={() => setStep(step - 1)}
        onenlarge={() => setStep(step + 1)}
        onreset={() => setStep(0)}
        smallerLabel={$t("popup.text_smaller")}
        largerLabel={$t("popup.text_larger")}
        resetLabel={$t("settings.text_size_reset")}
      />
    </SettingRow>
    <div class="preview" style:font-size={`${previewSize}px`}>
      {$t("settings.text_size_preview")}
    </div>
  </Card>

  <Card title={$t("settings.card_keys")}>
    <dl class="keys">
      <dt class="group">{$t("settings.keys_popup_and_panel")}</dt>
      <dd class="group-gap"></dd>
      <dt><Keycap label="c" /></dt>
      <dd>{$t("popup.copy")}</dd>
      <dt><Keycap label="s" /></dt>
      <dd>{$t("popup.speak")}</dd>
      <dt>
        <Keycap label="−" />
        <Keycap label="=" />
        <Keycap label="0" />
      </dt>
      <dd>{$t("settings.keys_text_size")}</dd>
      <dt><Keycap label="Esc" /></dt>
      <dd>{$t("popup.close")}</dd>
      <dt><Keycap label={mac ? "⌘," : "Ctrl+,"} /></dt>
      <dd>{$t("quick.key_settings")}</dd>

      <dt class="group">{$t("settings.keys_panel_only")}</dt>
      <dd class="group-gap"></dd>
      <dt><Keycap label="⇥" /></dt>
      <dd>{$t("quick.swap")}</dd>
      <dt><Keycap label="↵" /></dt>
      <dd>{$t("settings.keys_translate_now")}</dd>
      <dt><Keycap label="Esc" /></dt>
      <dd>{$t("settings.keys_close_from_input")}</dd>
    </dl>
  </Card>
</div>

<style>
  .stack {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .preview {
    display: block;
    padding: 8px 0;
    color: var(--text);
  }

  .stack .keys {
    display: grid;
    min-height: 0;
    border-top: none;
    grid-template-columns: 96px 1fr;
    align-items: center;
    gap: 6px 12px;
    margin: 0;
    padding: 8px 0;
    font-size: 12px;
    color: var(--text-secondary);
  }

  /* The Card turns direct children into rows; this grid is one child, its cells are plain. */
  dt,
  dd {
    margin: 0;
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 0;
    border: none;
  }

  dt.group {
    grid-column: 1 / -1;
    margin-top: 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text);
  }

  dd.group-gap {
    display: none;
  }
</style>
