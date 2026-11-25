<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import SettingRow from "$lib/components/SettingRow.svelte";
  import { t } from "$lib/i18n";
  import type { SettingsProfile } from "$lib/ipc";

  // Theme and accent in one group, as in System Settings › Appearance (spec 011).
  let {
    settings,
    onupdate,
  }: {
    settings: SettingsProfile;
    onupdate: (patch: Record<string, unknown>) => Promise<void>;
  } = $props();

  const ACCENTS = ["app", "system"] as const;
</script>

<Card>
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
  <SettingRow label={$t("settings.card_accent")} description={$t("settings.accent_hint")}>
    <div class="accents" role="radiogroup" aria-label={$t("settings.card_accent")}>
      {#each ACCENTS as option (option)}
        <label class="swatch" class:selected={settings.accent === option}>
          <input
            class="sr-only"
            type="radio"
            name="accent"
            value={option}
            checked={settings.accent === option}
            onchange={() => void onupdate({ accent: option })}
          />
          <span class="chip {option}" aria-hidden="true"></span>
          <span>{$t(`settings.accent_${option}`)}</span>
        </label>
      {/each}
    </div>
  </SettingRow>
</Card>

<style>
  .accents {
    display: flex;
    gap: 6px;
  }

  .swatch {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px 2px 4px;
    border-radius: 999px;
    color: var(--text);
    font-size: var(--type-row-value-size);
    cursor: pointer;
  }

  .swatch.selected {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .swatch:has(input:focus-visible) {
    box-shadow: var(--focus-ring);
  }

  .chip {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1px var(--hairline-strong);
  }

  /* The swatch shows the app's own colour, so it reads --accent-app, which the system accent never overrides. */
  .chip.app {
    background: var(--accent-app);
  }

  .chip.system {
    background: conic-gradient(var(--accent), var(--success), var(--accent));
  }
</style>
