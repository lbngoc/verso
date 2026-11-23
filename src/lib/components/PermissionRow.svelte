<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import SettingRow from "./SettingRow.svelte";
  import StatusBadge from "./StatusBadge.svelte";
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
  // granted, it is a name, a description and a switch that opens the system pane; missing, it adds
  // a warning and the way to fix it. Re-checked every second while missing, so the row flips
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

<div class="row">
  <SettingRow label={name} helper={body}>
    {#if granted}
      {#key switchKey}
        <Switch checked={true} label={name} onchange={() => void openPane()} />
      {/key}
    {/if}
  </SettingRow>

  {#if !granted}
    <StatusBadge kind="warn" text={$t("settings.permission_missing")} />
    <div class="actions">
      {#if reason === "after_update"}
        <button class="primary" type="button" onclick={() => void reset()}>
          {$t("permission.reset")}
        </button>
        <button type="button" onclick={() => void openPane()}>{$t("capture.grant")}</button>
      {:else}
        <button class="primary" type="button" onclick={() => void openPane()}>
          {$t("capture.grant")}
        </button>
      {/if}
      {#if restartHint}
        <button type="button" onclick={() => void commands.restartApp()}>
          {$t("permission.restart")}
        </button>
      {/if}
    </div>
    {#if resetState === "done"}
      <p class="note">{$t("permission.switch_on_now")}</p>
    {:else if resetState === "failed"}
      <p class="note">{$t("permission.reset_failed")}</p>
    {/if}
    {#if restartHint}
      <p class="note">{$t("permission.restart_hint")}</p>
    {/if}
  {/if}
</div>

<style>
  .row {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  button {
    height: var(--control-height);
    padding: 0 12px;
    border: none;
    border-radius: 6px;
    font-size: 12px;
    background: var(--surface-opaque);
    box-shadow: inset 0 0 0 1px var(--stroke);
    color: var(--text);
  }

  button.primary {
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
    box-shadow: none;
  }

  .note {
    margin: 0;
    font-size: 12px;
    color: var(--text-secondary);
  }
</style>
