<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import Button from "./Button.svelte";
  import Icon from "./Icon.svelte";
  import SettingRow from "./SettingRow.svelte";
  import Switch from "./Switch.svelte";
  import { t } from "$lib/i18n";
  import {
    commands,
    type PermissionKind,
    type PermissionReason,
    type PermissionStatus,
  } from "$lib/ipc";
  import { createPermissionPoll } from "$lib/permissionPoll";

  // One permission as one row in Settings › Permissions (spec 004 FR-014, contracts/ui-polish.md):
  // granted, it is a name, a description and a switch that opens the system pane; missing, the
  // buttons that fix it take the switch's place on the right and a warning sits under the row. Re-checked every second while missing, so the row flips
  // without reopening Settings.
  let { kind }: { kind: PermissionKind } = $props();

  let status = $state<PermissionStatus>("denied");
  let reason = $state<PermissionReason>("missing");
  let paneOpened = $state(false);
  let resetState = $state<"idle" | "done" | "failed">("idle");
  // Re-creates the switch after a click, so it never shows "off" while the grant stays effective.
  let switchKey = $state(0);

  const granted = $derived(status === "granted");
  const restartHint = $derived(kind === "screen_recording" && paneOpened && !granted);
  const name = $derived(
    kind === "screen_recording"
      ? $t("permission.screen_recording_name")
      : $t("permission.accessibility_name"),
  );
  const body = $derived(
    kind === "screen_recording"
      ? $t("permission.screen_recording_row_body")
      : $t("permission.accessibility_body"),
  );

  const check = () => commands.checkPermissions(kind);
  let poll = createPermissionPoll(check, () => (status = "granted"));

  $effect(() => {
    if (granted) poll.stop();
    else poll.start();
  });

  async function refresh(): Promise<void> {
    status = await check().catch(() => "denied" as const);
    if (status !== "granted") {
      // A grant lost after an update keeps its own wording and the one-click reset.
      reason = await commands.permissionReason(kind).catch(() => "missing" as const);
    }
  }

  async function openPane(): Promise<void> {
    await commands.requestPermission(kind).catch(() => undefined);
    paneOpened = true;
    await refresh();
    switchKey += 1;
  }

  async function reset(): Promise<void> {
    const outcome = await commands
      .resetPermission(kind)
      .catch(() => ({ ok: false, status: "denied" as const }));
    resetState = outcome.ok ? "done" : "failed";
    await refresh();
  }

  function onFocus(): void {
    if (!granted) void refresh();
  }

  onMount(() => {
    void refresh();
    window.addEventListener("focus", onFocus);
  });

  onDestroy(() => {
    poll.stop();
    window.removeEventListener("focus", onFocus);
  });
</script>

{#snippet missing()}
  <!-- Missing: the fixes sit under the row, right-aligned, so a long label or button never squeezes. -->
  <div class="actions">
    {#if restartHint}
      <Button onclick={() => void commands.restartApp()}>{$t("permission.restart")}</Button>
    {/if}
    {#if reason === "after_update"}
      <Button onclick={() => void openPane()}>{$t("capture.grant")}</Button>
      <Button variant="primary" onclick={() => void reset()}>{$t("permission.reset")}</Button>
    {:else}
      <Button variant="primary" onclick={() => void openPane()}>{$t("capture.grant")}</Button>
    {/if}
  </div>
  <p class="warning" role="status">
    <Icon name="warning" size={14} />
    {kind === "screen_recording"
      ? $t("settings.screen_permission_missing")
      : $t("settings.permission_missing")}
  </p>
  {#if resetState === "done"}
    <p class="note">{$t("permission.switch_on_now")}</p>
  {:else if resetState === "failed"}
    <p class="note">{$t("permission.reset_failed")}</p>
  {/if}
  {#if restartHint}
    <p class="note">{$t("permission.restart_hint")}</p>
  {/if}
{/snippet}

<SettingRow kind="block" label={name} description={body} content={granted ? undefined : missing}>
  {#if granted}
    {#key switchKey}
      <Switch checked={true} label={name} onchange={() => void openPane()} />
    {/key}
  {/if}
</SettingRow>

<style>
  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
    margin-bottom: 6px;
  }

  .warning {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: var(--type-row-description-size);
    color: var(--warning);
  }

  .note {
    margin: 4px 0 0;
    font-size: var(--type-row-description-size);
    color: var(--text-secondary);
  }
</style>
