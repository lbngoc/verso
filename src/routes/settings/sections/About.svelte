<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import appIcon from "$lib/assets/app-icon.png";
  import Card from "$lib/components/Card.svelte";
  import { t } from "$lib/i18n";
  import {
    commands,
    events,
    type SettingsProfile,
    type UpdateInfo,
  } from "$lib/ipc";

  // About: identity, report links and the update card (US4 replaces the inline update UI with the
  // dedicated dialog; the checkbox and the manual check live here meanwhile).
  let {
    settings,
    onupdate,
  }: {
    settings: SettingsProfile;
    onupdate: (patch: Record<string, unknown>) => Promise<void>;
  } = $props();

  let version = $state("");
  let phase = $state<"idle" | "checking" | "up_to_date" | "available" | "downloading" | "ready" | "failed">("idle");
  let update = $state<UpdateInfo | null>(null);
  let percent = $state(0);
  let copied = $state(false);

  onMount(() => {
    void getVersion().then((value) => (version = value)).catch(() => undefined);
    const listeners = Promise.all([
      events.onUpdateAvailable((info) => {
        update = info;
        if (phase !== "downloading") phase = "available";
      }),
      events.onUpdateProgress((payload) => (percent = payload.percent)),
    ]);
    return () => {
      void listeners.then((fns) => fns.forEach((unlisten) => unlisten()));
    };
  });

  async function check(): Promise<void> {
    phase = "checking";
    try {
      const info = await commands.checkForUpdate();
      update = info;
      phase = info ? "available" : "up_to_date";
    } catch {
      phase = "failed";
    }
  }

  async function install(): Promise<void> {
    percent = 0;
    phase = "downloading";
    try {
      await commands.installUpdate();
      phase = "ready";
    } catch {
      phase = "failed";
    }
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
      <div>
        <h2>{$t("app.name")}</h2>
        <p class="hint">{$t("about.full_name")}</p>
        <p class="hint">{$t("about.version", { version })}</p>
      </div>
    </div>
    <p class="hint credits">{$t("about.credits")}</p>
    <div class="links">
      <button type="button" onclick={() => void commands.reportProblem()}>
        {$t("about.report")}
      </button>
      <button type="button" onclick={() => void diagnostics()}>
        {$t("about.diagnostics")}
      </button>
      {#if copied}<span class="hint">{$t("about.diagnostics_copied")}</span>{/if}
    </div>
  </Card>

  <Card title={$t("about.updates")} helper={$t("about.automatic_hint")}>
    <label class="row check">
      <input
        type="checkbox"
        checked={settings.check_updates}
        onchange={(event) => void onupdate({ check_updates: event.currentTarget.checked })}
      />
      <span>{$t("about.automatic")}</span>
    </label>
    <p class="row state" aria-live="polite">
      {#if phase === "checking"}
        {$t("about.checking")}
      {:else if phase === "up_to_date"}
        {$t("about.up_to_date")}
      {:else if phase === "available" && update}
        {$t("about.available", { version: update.version })}
      {:else if phase === "downloading"}
        {$t("about.downloading", { percent })}
      {:else if phase === "ready"}
        {$t("about.ready")}
      {:else if phase === "failed"}
        {$t("about.check_failed")}
      {/if}
    </p>
    <div class="row actions">
      {#if phase === "available"}
        <button type="button" class="primary" onclick={() => void install()}>
          {$t("about.install")}
        </button>
      {/if}
      <button type="button" disabled={phase === "checking" || phase === "downloading"} onclick={() => void check()}>
        {$t("about.check_now")}
      </button>
    </div>
  </Card>
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
    gap: 16px;
    padding: 8px 0;
  }

  .identity img {
    border-radius: 22px;
  }

  .identity h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
  }

  .hint {
    margin: 2px 0;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .credits {
    margin: 4px 0 8px;
  }

  .links {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
  }

  .check {
    cursor: pointer;
  }

  .state {
    color: var(--text-secondary);
    font-size: 12px;
  }

  .actions {
    padding: 4px 0 6px;
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

  button.primary {
    background: var(--accent);
    color: var(--accent-contrast);
    box-shadow: none;
  }
</style>
