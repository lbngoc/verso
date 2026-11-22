<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import Card from "./Card.svelte";
  import SettingRow from "./SettingRow.svelte";
  import StatusBadge from "./StatusBadge.svelte";
  import { t } from "$lib/i18n";
  import {
    commands,
    type PermissionKind,
    type PermissionReason,
    type PermissionStatus,
  } from "$lib/ipc";
  import { createPermissionPoll } from "$lib/permissionPoll";

  // The permission step (spec 010 contracts/permission-step.md), shared by Setup and Settings ›
  // Permissions, and generic over the permission kind so Screen Recording can reuse it later.
  let {
    kind,
    reason = "missing",
    status = $bindable("denied"),
    ongranted,
  }: {
    kind: PermissionKind;
    reason?: PermissionReason;
    status?: PermissionStatus;
    ongranted?: () => void;
  } = $props();

  // Set after "Reset and grant again": `done` = the fresh row is waiting to be switched on,
  // `failed` = the system tool refused, so the manual steps are the way forward.
  let resetState = $state<"idle" | "done" | "failed">("idle");
  // Screen Recording: macOS may apply a new grant to a running process only after a relaunch
  // (spec 004 R8). Once the pane was opened and the grant is still missing, offer the restart.
  let paneOpened = $state(false);
  let restartHint = $derived(kind === "screen_recording" && paneOpened && status !== "granted");
  let granted = $derived(status === "granted");

  const check = () => commands.checkPermissions(kind);
  const poll = createPermissionPoll(check, () => {
    status = "granted";
    ongranted?.();
  });

  // Polls only while the grant is missing; success shows only once the check reports it.
  $effect(() => {
    if (granted) poll.stop();
    else poll.start();
  });

  function onFocus(): void {
    if (!granted) void poll.recheckNow();
  }

  onMount(() => {
    window.addEventListener("focus", onFocus);
  });

  onDestroy(() => {
    poll.stop();
    window.removeEventListener("focus", onFocus);
  });

  async function openSettings(): Promise<void> {
    await commands.requestPermission(kind).catch(() => undefined);
    paneOpened = true;
    await poll.recheckNow();
  }

  async function resetAndGrant(): Promise<void> {
    const outcome = await commands
      .resetPermission(kind)
      .catch(() => ({ ok: false, status: "denied" as const }));
    resetState = outcome.ok ? "done" : "failed";
    await poll.recheckNow();
  }
</script>

<div class="guide">
  {#if !granted && reason === "after_update"}
    <div class="notice">
      <strong>{$t("permission.after_update_title")}</strong>
      <span>{$t("permission.after_update_body")}</span>
    </div>
  {/if}

  <Card>
    <SettingRow
      label={kind === "screen_recording"
        ? $t("permission.screen_recording_name")
        : $t("settings.permission")}
    >
      {#if granted}
        <!-- Nothing to do: the badge below says it is granted. -->
      {:else if reason === "after_update"}
        <button class="primary" type="button" onclick={() => void resetAndGrant()}>
          {$t("permission.reset")}
        </button>
        <button class="secondary" type="button" onclick={() => void openSettings()}>
          {$t("capture.grant")}
        </button>
      {:else}
        <button class="primary" type="button" onclick={() => void openSettings()}>
          {$t("capture.grant")}
        </button>
        <button class="secondary" type="button" onclick={() => void poll.recheckNow()}>
          {$t("capture.recheck")}
        </button>
      {/if}
    </SettingRow>
  </Card>

  <StatusBadge
    kind={granted ? "ok" : "warn"}
    text={granted ? $t("settings.permission_granted") : $t("settings.permission_missing")}
  />

  {#if !granted}
    {#if restartHint}
      <p class="next">{$t("permission.restart_hint")}</p>
      <div>
        <button class="secondary" type="button" onclick={() => void commands.restartApp()}>
          {$t("permission.restart")}
        </button>
      </div>
    {/if}
    {#if resetState === "done"}
      <p class="next">{$t("permission.switch_on_now")}</p>
    {:else if resetState === "failed"}
      <p class="next">{$t("permission.reset_failed")}</p>
    {/if}
    <p class="hint" class:emphasis={resetState === "failed"}>{$t("capture.stale_hint")}</p>
  {/if}
</div>

<style>
  .guide {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .notice {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12px;
    color: var(--text);
  }

  .notice strong {
    font-size: 13px;
  }

  button {
    height: var(--control-height);
    padding: 0 12px;
    border: none;
    border-radius: 6px;
    font-size: 12px;
  }

  .primary {
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
  }

  .secondary {
    background: var(--surface-opaque);
    box-shadow: inset 0 0 0 1px var(--stroke);
    color: var(--text);
  }

  .next {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
  }

  .hint {
    margin: 0;
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .hint.emphasis {
    font-size: 12px;
    color: var(--text);
  }
</style>
