<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import SettingRow from "$lib/components/SettingRow.svelte";
  import { t } from "$lib/i18n";
  import { commands } from "$lib/ipc";

  let {
    permission,
    ongranted,
  }: {
    permission: "granted" | "denied" | "not_determined";
    ongranted: (status: "granted" | "denied" | "not_determined") => void;
  } = $props();

  async function fix(): Promise<void> {
    await commands.requestPermission("accessibility").catch(() => undefined);
    const status = await commands.checkPermissions("accessibility").catch(() => "denied" as const);
    ongranted(status);
  }
</script>

<Card title={$t("settings.permission")}>
  <SettingRow
    label={permission === "granted"
      ? $t("settings.permission_granted")
      : $t("settings.permission_missing")}
  >
    <button type="button" onclick={() => void fix()}>{$t("settings.permission_fix")}</button>
  </SettingRow>
</Card>

<style>
  button {
    border: none;
    border-radius: 6px;
    height: var(--control-height);
    padding: 0 12px;
    background: var(--accent);
    color: var(--accent-contrast);
    font-size: 12px;
  }
</style>
