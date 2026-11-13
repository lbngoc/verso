<script lang="ts">
  import Card from "$lib/components/Card.svelte";
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
  <div class="row">
    <span class:ok={permission === "granted"}>
      {permission === "granted"
        ? $t("settings.permission_granted")
        : $t("settings.permission_missing")}
    </span>
    <button type="button" onclick={() => void fix()}>{$t("settings.permission_fix")}</button>
  </div>
</Card>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 13px;
  }

  .ok {
    color: var(--success);
  }

  button {
    border: none;
    border-radius: 6px;
    padding: 4px 12px;
    background: var(--accent);
    color: var(--accent-contrast);
    font-size: 12px;
  }
</style>
