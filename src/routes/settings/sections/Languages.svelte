<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import SettingRow from "$lib/components/SettingRow.svelte";
  import { t } from "$lib/i18n";
  import type { Language, SettingsProfile } from "$lib/ipc";

  let {
    settings,
    languages,
    onupdate,
  }: {
    settings: SettingsProfile;
    languages: Language[];
    onupdate: (patch: Record<string, unknown>) => Promise<void>;
  } = $props();
</script>

<Card title={$t("settings.card_languages")} helper={$t("settings.secondary_hint")}>
  <SettingRow label={$t("settings.target_language")}>
    <select
      aria-label={$t("settings.target_language")}
      value={settings.target_lang}
      onchange={(event) => void onupdate({ target_lang: event.currentTarget.value })}
    >
      {#each languages as language (language.code)}
        <option value={language.code}>{language.name_en}</option>
      {/each}
    </select>
  </SettingRow>
  <SettingRow label={$t("settings.secondary_language")}>
    <select
      aria-label={$t("settings.secondary_language")}
      value={settings.secondary_lang}
      onchange={(event) => void onupdate({ secondary_lang: event.currentTarget.value })}
    >
      {#each languages as language (language.code)}
        <option value={language.code}>{language.name_en}</option>
      {/each}
    </select>
  </SettingRow>
  <SettingRow label={$t("settings.source_language")}>
    <select
      aria-label={$t("settings.source_language")}
      value={settings.source_lang}
      onchange={(event) => void onupdate({ source_lang: event.currentTarget.value })}
    >
      <option value="auto">{$t("quick.auto_detect")}</option>
      {#each languages as language (language.code)}
        <option value={language.code}>{language.name_en}</option>
      {/each}
    </select>
  </SettingRow>
</Card>

<style>
  select {
    min-width: 200px;
  }
</style>
