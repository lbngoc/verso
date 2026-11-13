<script lang="ts">
  import Card from "$lib/components/Card.svelte";
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
    <label class="row check">
      <input
        type="checkbox"
        checked={settings.launch_at_login}
        onchange={(event) => void onupdate({ launch_at_login: event.currentTarget.checked })}
      />
      <span>{$t("settings.launch_at_login")}</span>
    </label>
  </Card>

  <Card title={$t("settings.card_language")}>
    <label class="row">
      <span>{$t("settings.ui_language")}</span>
      <select
        value={settings.ui_language}
        onchange={(event) => void onupdate({ ui_language: event.currentTarget.value })}
      >
        <option value="en">English</option>
        <option value="vi">Tiếng Việt</option>
      </select>
    </label>
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

  .check {
    justify-content: flex-start;
    cursor: pointer;
  }

  select {
    min-width: 160px;
  }
</style>
