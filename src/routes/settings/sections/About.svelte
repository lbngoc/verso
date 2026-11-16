<script lang="ts">
  import { onMount } from "svelte";
  import appIcon from "$lib/assets/app-icon.png";
  import Card from "$lib/components/Card.svelte";
  import { t } from "$lib/i18n";
  import { commands, type AppInfo, type SettingsProfile } from "$lib/ipc";

  // About: identity, links and the update card (spec 009, contracts/settings-window.md).
  // The inline update UI of 0.1.x is gone: "Check for Updates…" opens the update dialog (T052).
  let {
    settings,
    onupdate,
  }: {
    settings: SettingsProfile;
    onupdate: (patch: Record<string, unknown>) => Promise<void>;
  } = $props();

  let info = $state<AppInfo | null>(null);
  let copied = $state(false);
  let lastChecked = $state("");

  onMount(() => {
    void commands
      .appInfo()
      .then((value) => (info = value))
      .catch(() => (info = { version: "", build_date: "dev" }));
  });

  async function openUpdates(): Promise<void> {
    lastChecked = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    await commands.openUpdateWindow("check").catch(() => undefined);
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
      <img src={appIcon} alt="" width="96" height="96" />
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
      <button
        type="button"
        onclick={() => void commands.openUrl("https://github.com/lbngoc/verso")}
      >
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
  </Card>

  <Card title={$t("about.updates")}>
    <label class="row check">
      <input
        type="checkbox"
        checked={settings.check_updates}
        onchange={(event) => void onupdate({ check_updates: event.currentTarget.checked })}
      />
      <span>{$t("about.automatic")}</span>
    </label>
    <div class="row">
      <button type="button" onclick={() => void openUpdates()}>{$t("about.check_updates")}</button>
      {#if lastChecked}
        <span class="hint">{$t("about.last_checked", { when: lastChecked })}</span>
      {/if}
    </div>
    <p class="hint">{$t("about.automatic_hint")}</p>
  </Card>

  <p class="legal">
    <button
      type="button"
      class="link"
      onclick={() => void commands.openUrl("https://github.com/lbngoc/verso/blob/main/LICENSE")}
    >
      {$t("about.terms")}
    </button>
    ·
    <button
      type="button"
      class="link"
      onclick={() =>
        void commands.openUrl("https://github.com/lbngoc/verso/blob/main/THIRD_PARTY.md")}
    >
      {$t("about.licences")}
    </button>
  </p>
</div>

<style>
  .stack {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .identity {
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 8px 0 12px;
  }

  .identity img {
    border-radius: 22px;
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

  .links {
    border-top: 1px solid var(--separator);
    padding-top: 12px;
  }

  .check {
    cursor: pointer;
    min-height: 36px;
  }

  .row {
    min-height: 36px;
  }

  button {
    border: none;
    border-radius: 6px;
    padding: 5px 12px;
    background: var(--surface-opaque);
    box-shadow: inset 0 0 0 1px var(--stroke);
    color: var(--text);
    font-size: 12px;
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
