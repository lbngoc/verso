<script lang="ts">
  import { onMount } from "svelte";
  import appIcon from "$lib/assets/app-icon.png";
  import Button from "$lib/components/Button.svelte";
  import Card from "$lib/components/Card.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import SettingRow from "$lib/components/SettingRow.svelte";
  import Switch from "$lib/components/Switch.svelte";
  import UpdateCard from "$lib/components/UpdateCard.svelte";
  import { t } from "$lib/i18n";
  import { commands, type AppInfo, type SettingsProfile, type UpdateStatus } from "$lib/ipc";
  import { runUpdateCheck } from "$lib/updateCheck";

  // About: identity, links, the update offer and Software Update (spec 011 US2,
  // contracts/update-in-settings.md § About layout). The update status lives in the backend; the
  // Settings shell reads it on open and follows `update-status`, so a download keeps going and shows
  // here when Settings is reopened. Links open by name; their URLs live in src-tauri/src/links.rs.
  let {
    settings,
    updateStatus,
    onupdate,
  }: {
    settings: SettingsProfile;
    updateStatus: UpdateStatus | null;
    onupdate: (patch: Record<string, unknown>) => Promise<void>;
  } = $props();

  const IDLE: UpdateStatus = {
    phase: "idle",
    offer: null,
    last_checked: null,
    progress: null,
    error: null,
  };

  let info = $state<AppInfo | null>(null);
  let copied = $state(false);
  // A check reported here before (or without) the backend's event, e.g. when the call itself fails.
  let reported = $state<UpdateStatus | null>(null);
  const windows = typeof navigator !== "undefined" && navigator.userAgent.includes("Windows");

  $effect(() => {
    // A newer status from the backend replaces what the page reported.
    void updateStatus;
    reported = null;
  });

  const status = $derived(reported ?? updateStatus ?? IDLE);
  const busy = $derived(
    status.phase === "checking" ||
      status.phase === "downloading" ||
      status.phase === "ready_on_quit" ||
      status.phase === "installing",
  );
  const lastChecked = $derived(
    status.last_checked
      ? $t("about.last_checked", { when: formatTime(status.last_checked) })
      : $t("about.update_never_checked"),
  );

  function formatTime(iso: string): string {
    const date = new Date(iso);
    if (Number.isNaN(date.getTime())) return iso;
    return date.toLocaleString(settings.ui_language === "vi" ? "vi-VN" : "en", {
      dateStyle: "medium",
      timeStyle: "short",
    });
  }

  onMount(() => {
    void commands
      .appInfo()
      .then((value) => (info = value))
      .catch(() => (info = { version: "", build_date: "dev" }));
  });

  async function checkForUpdates(): Promise<void> {
    await runUpdateCheck(commands.checkForUpdate, (next) => (reported = next), status);
  }

  function install(): void {
    void commands.installUpdate().catch(() => undefined);
  }

  function installOnQuit(): void {
    void commands.installUpdateOnQuit().catch(() => undefined);
  }

  function skip(): void {
    const version = status.offer?.version;
    if (version) void commands.skipUpdateVersion(version).catch(() => undefined);
  }

  async function diagnostics(): Promise<void> {
    await commands.copyDiagnostics().catch(() => undefined);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }
</script>

<div class="stack">
  <Card>
    <div class="identity">
      <img src={appIcon} alt="" width="64" height="64" />
      <div class="names">
        <h2>{$t("app.name")}</h2>
        <p class="row-value">{$t("about.full_name")}</p>
        <p class="row-value">
          {#if info}
            {$t("about.version_build", { version: info.version, date: info.build_date })}
          {/if}
        </p>
      </div>
    </div>
    {#snippet footer()}
      {#if copied}<span class="copied caption" role="status">{$t("about.diagnostics_copied")}</span
        >{/if}
      <Button onclick={() => void commands.openPublicPage("home")}>{$t("about.github")}</Button>
      <Button onclick={() => void commands.reportProblem()}>{$t("about.report")}</Button>
      <Button onclick={() => void diagnostics()}>{$t("about.diagnostics")}</Button>
    {/snippet}
  </Card>

  <UpdateCard
    {status}
    {windows}
    oninstall={install}
    oninstallonquit={installOnQuit}
    onskip={skip}
  />

  <Card title={$t("about.updates")}>
    <SettingRow label={$t("about.automatic")} description={$t("about.automatic_hint")}>
      <Switch
        checked={settings.check_updates}
        label={$t("about.automatic")}
        onchange={(checked) => void onupdate({ check_updates: checked })}
      />
    </SettingRow>
    <div class="check" role="status">
      {#if status.phase === "checking"}
        <span class="row-value">{$t("about.checking")}</span>
      {:else if status.phase === "up_to_date"}
        <span class="ok"><Icon name="check" size={14} />{$t("about.up_to_date")}</span>
        <span class="row-value">{lastChecked}</span>
      {:else if status.phase === "check_failed"}
        <span class="warn"><Icon name="warning" size={14} />{$t("about.check_failed")}</span>
        <span class="row-value">{lastChecked}</span>
      {:else}
        <span class="row-value">{lastChecked}</span>
      {/if}
    </div>
    {#snippet footer()}
      <Button disabled={busy} onclick={() => void checkForUpdates()}>
        {$t("about.check_updates")}
      </Button>
    {/snippet}
  </Card>

  <p class="legal caption">
    <Button variant="link" onclick={() => void commands.openPublicPage("terms")}>
      {$t("about.terms")}
    </Button>
    ·
    <Button variant="link" onclick={() => void commands.openPublicPage("privacy")}>
      {$t("about.privacy")}
    </Button>
    ·
    <Button variant="link" onclick={() => void commands.openPublicPage("licences")}>
      {$t("about.licences")}
    </Button>
  </p>
</div>

<style>
  .stack {
    display: flex;
    flex-direction: column;
    gap: var(--group-gap);
  }

  .identity {
    gap: 14px;
    padding: 14px 0 6px;
  }

  .identity img {
    border-radius: 14px;
  }

  .names {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .names h2 {
    margin: 0;
    font-size: var(--type-page-title-size);
    font-weight: var(--type-page-title-weight);
  }

  .names p {
    margin: 0;
  }

  .copied {
    margin-right: auto;
    align-self: center;
  }

  .check {
    flex-wrap: wrap;
    gap: 6px 12px;
  }

  .check .ok,
  .check .warn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--type-row-value-size);
    font-weight: 600;
  }

  .ok {
    color: var(--success);
  }

  .warn {
    color: var(--warning);
  }

  .legal {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 6px;
    margin: 0;
    padding: 0 var(--card-padding-x);
  }
</style>
