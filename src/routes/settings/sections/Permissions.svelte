<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import SettingRow from "$lib/components/SettingRow.svelte";
  import StatusBadge from "$lib/components/StatusBadge.svelte";
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
  <SettingRow label={$t("settings.permission")}>
    <button type="button" onclick={() => void fix()}>{$t("settings.permission_fix")}</button>
  </SettingRow>
</Card>
<div class="status">
  <StatusBadge
    kind={permission === "granted" ? "ok" : "warn"}
    text={permission === "granted"
      ? $t("settings.permission_granted")
      : $t("settings.permission_missing")}
  />
</div>

<style>
  .status {
    margin-top: 12px;
  }

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
