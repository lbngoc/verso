<script lang="ts">
  import { onMount } from "svelte";
  import appIcon from "$lib/assets/app-icon.png";
  import Card from "$lib/components/Card.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import SettingRow from "$lib/components/SettingRow.svelte";
  import Switch from "$lib/components/Switch.svelte";
  import { t } from "$lib/i18n";
  import { commands, type AppInfo, type SettingsProfile } from "$lib/ipc";
  import { runUpdateCheck, type CheckState } from "$lib/updateCheck";

  // About: identity, links and the update card (spec 009, contracts/settings-window.md).
  // Links open by name; their URLs live in src-tauri/src/links.rs (the public repository).
  let {
    settings,
    onupdate,
  }: {
    settings: SettingsProfile;
    onupdate: (patch: Record<string, unknown>) => Promise<void>;
  } = $props();

  let info = $state<AppInfo | null>(null);
  let copied = $state(false);
  // The check runs here; the update dialog opens only when there is something to install (FR-050).
  let check = $state<CheckState>("idle");

  onMount(() => {
    void commands
      .appInfo()
      .then((value) => (info = value))
      .catch(() => (info = { version: "", build_date: "dev" }));
  });

  async function checkForUpdates(): Promise<void> {
    await runUpdateCheck(
      { check: commands.checkForUpdate, openOffer: () => commands.openUpdateWindow("offer") },
      (state) => (check = state),
    );
  }

  async function diagnostics(): Promise<void> {
    await commands.copyDiagnostics().catch(() => undefined);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }
</script>

<div class="stack">
  <Card>
    <div class="head">
      <div class="identity">
        <img src={appIcon} alt="" width="72" height="72" />
        <div class="names">
          <h2>{$t("app.name")}</h2>
          <p class="hint">{$t("about.full_name")}</p>
          <p class="hint">
            {#if info}
              {$t("about.version_build", { version: info.version, date: info.build_date })}
            {/if}
          </p>
        </div>
      </div>
      <div class="links">
        <button type="button" onclick={() => void commands.openPublicPage("home")}>
          {$t("about.github")}
        </button>
        <button type="button" onclick={() => void commands.reportProblem()}>
          {$t("about.report")}
        </button>
        <button type="button" onclick={() => void diagnostics()}>
          {$t("about.diagnostics")}
        </button>
        {#if copied}<span class="hint">{$t("about.diagnostics_copied")}</span>{/if}
      </div>
    </div>
  </Card>

  <Card title={$t("about.updates")} helper={$t("about.automatic_hint")}>
    <SettingRow label={$t("about.automatic")}>
      <Switch
        checked={settings.check_updates}
        label={$t("about.automatic")}
        onchange={(checked) => void onupdate({ check_updates: checked })}
      />
    </SettingRow>
    <div class="row">
      <button type="button" disabled={check === "checking"} onclick={() => void checkForUpdates()}>
        {$t("about.check_updates")}
      </button>
      <span class="status" role="status">
        {#if check === "checking"}
          <span class="hint">{$t("about.checking")}</span>
        {:else if check === "up_to_date"}
          <span class="ok"><Icon name="check" size={14} />{$t("about.up_to_date")}</span>
        {:else if check === "failed"}
          <span class="warn"><Icon name="warning" size={14} />{$t("about.check_failed")}</span>
        {/if}
      </span>
    </div>
  </Card>

  <p class="legal">
    <button type="button" class="link" onclick={() => void commands.openPublicPage("terms")}>
      {$t("about.terms")}
    </button>
    ·
    <button type="button" class="link" onclick={() => void commands.openPublicPage("privacy")}>
      {$t("about.privacy")}
    </button>
    ·
    <button type="button" class="link" onclick={() => void commands.openPublicPage("licences")}>
      {$t("about.licences")}
    </button>
  </p>
</div>

<style>
  .stack {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  /* Card rows draw a hairline above each row: the identity and its links form one row, no line. */
  .head {
    flex-direction: column;
    align-items: stretch !important;
    gap: 10px;
    padding: 10px 0 12px;
  }

  .identity {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .identity img {
    border-radius: 16px;
  }

  .names h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
  }

  .hint {
    margin: 2px 0;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .links,
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 13px;
  }

  .status span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }

  .status .ok {
    color: var(--success);
    font-weight: 600;
  }

  .status .warn {
    color: var(--warning);
    font-weight: 600;
  }

  button {
    border: none;
    border-radius: 6px;
    height: var(--control-height);
    padding: 0 12px;
    background: var(--surface-opaque);
    box-shadow: inset 0 0 0 1px var(--stroke);
    color: var(--text);
    font-size: 12px;
  }

  button:disabled {
    opacity: 0.6;
  }

  .legal {
    margin: 0;
    font-size: 11px;
    color: var(--text-tertiary);
    text-align: center;
  }

  .link {
    background: none;
    box-shadow: none;
    padding: 0;
    color: var(--text-secondary);
    text-decoration: underline;
  }
</style>
