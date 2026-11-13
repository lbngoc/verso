<script lang="ts">
  import Card from "$lib/components/Card.svelte";
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
  <label class="row">
    <span>{$t("settings.target_language")}</span>
    <select
      value={settings.target_lang}
      onchange={(event) => void onupdate({ target_lang: event.currentTarget.value })}
    >
      {#each languages as language (language.code)}
        <option value={language.code}>{language.name_en}</option>
      {/each}
    </select>
  </label>
  <label class="row">
    <span>{$t("settings.secondary_language")}</span>
    <select
      value={settings.secondary_lang}
      onchange={(event) => void onupdate({ secondary_lang: event.currentTarget.value })}
    >
      {#each languages as language (language.code)}
        <option value={language.code}>{language.name_en}</option>
      {/each}
    </select>
  </label>
  <label class="row">
    <span>{$t("settings.source_language")}</span>
    <select
      value={settings.source_lang}
      onchange={(event) => void onupdate({ source_lang: event.currentTarget.value })}
    >
      <option value="auto">{$t("quick.auto_detect")}</option>
      {#each languages as language (language.code)}
        <option value={language.code}>{language.name_en}</option>
      {/each}
    </select>
  </label>
</Card>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 13px;
  }

  select {
    min-width: 200px;
  }
</style>
