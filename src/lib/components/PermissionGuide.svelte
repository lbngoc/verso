<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import Button from "./Button.svelte";
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
    ></SettingRow>
    <!-- The actions belong to the group: bottom right of the box, so a long label never squeezes. -->
    {#snippet footer()}
      {#if granted}
        <!-- Nothing to do: the badge below says it is granted. -->
      {:else if reason === "after_update"}
        <Button onclick={() => void openSettings()}>{$t("capture.grant")}</Button>
        <Button variant="primary" onclick={() => void resetAndGrant()}>
          {$t("permission.reset")}
        </Button>
      {:else}
        <Button onclick={() => void poll.recheckNow()}>{$t("capture.recheck")}</Button>
        <Button variant="primary" onclick={() => void openSettings()}>{$t("capture.grant")}</Button>
      {/if}
    {/snippet}
  </Card>

  <StatusBadge
    kind={granted ? "ok" : "warn"}
    text={granted ? $t("settings.permission_granted") : $t("settings.permission_missing")}
  />

  {#if !granted}
    {#if restartHint}
      <p class="next">{$t("permission.restart_hint")}</p>
      <div class="restart">
        <Button onclick={() => void commands.restartApp()}>{$t("permission.restart")}</Button>
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
    font-size: var(--type-row-description-size);
    color: var(--text);
  }

  .notice strong {
    font-size: var(--type-row-label-size);
  }

  .restart {
    display: flex;
    justify-content: flex-end;
  }

  .next {
    margin: 0;
    font-size: var(--type-row-description-size);
    font-weight: 600;
    color: var(--text);
  }

  .hint {
    margin: 0;
    font-size: var(--type-caption-size);
    color: var(--text-tertiary);
  }

  .hint.emphasis {
    font-size: var(--type-row-description-size);
    color: var(--text);
  }
</style>
