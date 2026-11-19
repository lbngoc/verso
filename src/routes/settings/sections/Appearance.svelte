<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import SettingRow from "$lib/components/SettingRow.svelte";
  import { t } from "$lib/i18n";
  import type { SettingsProfile } from "$lib/ipc";

  // US6 lands the accent control here; the theme is the same card the 0.1.x page had.
  let {
    settings,
    onupdate,
  }: {
    settings: SettingsProfile;
    onupdate: (patch: Record<string, unknown>) => Promise<void>;
  } = $props();
</script>

<div class="stack">
  <Card title={$t("settings.card_theme")}>
    <SettingRow label={$t("settings.theme")}>
      <select
        aria-label={$t("settings.theme")}
        value={settings.theme}
        onchange={(event) => void onupdate({ theme: event.currentTarget.value })}
      >
        <option value="system">{$t("settings.theme_system")}</option>
        <option value="light">{$t("settings.theme_light")}</option>
        <option value="dark">{$t("settings.theme_dark")}</option>
      </select>
    </SettingRow>
  </Card>

  <Card title={$t("settings.card_accent")} helper={$t("settings.accent_hint")}>
    <div class="row accents" role="radiogroup" aria-label={$t("settings.card_accent")}>
      <button
        class="swatch"
        class:selected={settings.accent === "app"}
        type="button"
        role="radio"
        aria-checked={settings.accent === "app"}
        onclick={() => void onupdate({ accent: "app" })}
      >
        <span class="chip app" aria-hidden="true"></span>
        <span>{$t("settings.accent_app")}</span>
      </button>
      <button
        class="swatch"
        class:selected={settings.accent === "system"}
        type="button"
        role="radio"
        aria-checked={settings.accent === "system"}
        onclick={() => void onupdate({ accent: "system" })}
      >
        <span class="chip system" aria-hidden="true"></span>
        <span>{$t("settings.accent_system")}</span>
      </button>
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

  select {
    min-width: 160px;
  }

  .accents {
    justify-content: flex-start;
    gap: 16px;
    padding: 6px 0;
  }

  .swatch {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px 4px 6px;
    border: none;
    border-radius: 999px;
    background: transparent;
    color: var(--text);
    font-size: 12px;
  }

  .swatch.selected {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .chip {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1px var(--hairline-strong);
  }

  .chip.app {
    background: var(--accent);
  }

  .chip.system {
    background: conic-gradient(var(--accent), var(--success), var(--accent));
  }
</style>
