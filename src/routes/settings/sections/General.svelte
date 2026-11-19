<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import SettingRow from "$lib/components/SettingRow.svelte";
  import Switch from "$lib/components/Switch.svelte";
  import { t } from "$lib/i18n";
  import type { SettingsProfile } from "$lib/ipc";

  let {
    settings,
    onupdate,
  }: {
    settings: SettingsProfile;
    onupdate: (patch: Record<string, unknown>) => Promise<void>;
  } = $props();
</script>

<div class="stack">
  <Card title={$t("settings.card_startup")}>
    <SettingRow label={$t("settings.launch_at_login")}>
      <Switch
        checked={settings.launch_at_login}
        label={$t("settings.launch_at_login")}
        onchange={(checked) => void onupdate({ launch_at_login: checked })}
      />
    </SettingRow>
  </Card>

  <Card title={$t("settings.card_language")}>
    <SettingRow label={$t("settings.ui_language")}>
      <select
        value={settings.ui_language}
        aria-label={$t("settings.ui_language")}
        onchange={(event) => void onupdate({ ui_language: event.currentTarget.value })}
      >
        <option value="en">English</option>
        <option value="vi">Tiếng Việt</option>
      </select>
    </SettingRow>
  </Card>
</div>

<style>
  .stack {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  select {
    min-width: 160px;
  }
</style>
