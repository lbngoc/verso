<script lang="ts">
  import { onMount } from "svelte";
  import appIcon from "$lib/assets/app-icon.png";
  import Markdown from "$lib/components/Markdown.svelte";
  import { setUiLanguage, t as translator } from "$lib/i18n";
  import { initAppearance } from "$lib/stores/appearance";
  import { initTheme } from "$lib/stores/theme";
  import { commands, events, type SettingsProfile, type UpdateInfo } from "$lib/ipc";

  // macOS-style update dialog (spec 009, contracts/update-dialog.md).
  type Phase =
    | "checking"
    | "up_to_date"
    | "available"
    | "downloading"
    | "ready"
    | "install_on_quit"
    | "failed";

  let phase = $state<Phase>("checking");
  let offer = $state<UpdateInfo | null>(null);
  let settings = $state<SettingsProfile | null>(null);
  let percent = $state(0);
  let failure = $state<"offline" | "verified" | "download">("download");
  let current = $state("");

  async function check(): Promise<void> {
    phase = "checking";
    try {
      const info = await commands.checkForUpdate();
      offer = info;
      phase = info ? "available" : "up_to_date";
    } catch {
      failure = "offline";
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
      failure = "verified";
      phase = "failed";
    }
  }

  async function installOnQuit(): Promise<void> {
    percent = 0;
    phase = "downloading";
    try {
      await commands.installUpdateOnQuit();
      phase = "install_on_quit";
    } catch {
      failure = "download";
      phase = "failed";
    }
  }

  async function skip(): Promise<void> {
    if (!offer) return;
    await commands.skipUpdateVersion(offer.version).catch(() => undefined);
    await close();
  }

  async function close(): Promise<void> {
    await commands.hideUpdateWindow().catch(() => undefined);
  }

  async function setAutoCheck(enabled: boolean): Promise<void> {
    const profile = await commands.updateSettings({ check_updates: enabled }).catch(() => null);
    if (profile) settings = profile;
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      void close();
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (phase === "available") void install();
      else void close();
    }
  }

  onMount(() => {
    void (async () => {
      try {
        await initAppearance();
        current = await commands
          .appInfo()
          .then((info) => info.version)
          .catch(() => "");
        settings = await initTheme();
        if (!settings) settings = await commands.getSettings().catch(() => null);
        if (settings) setUiLanguage(settings.ui_language);

        const mode = await commands.updateMode().catch(() => "check");
        const listeners = await Promise.all([
          events.onUpdateAvailable((info) => {
            offer = info;
            if (phase !== "downloading") phase = "available";
          }),
          events.onUpdateProgress((payload) => (percent = payload.percent)),
          events.onSettingsChanged((profile) => (settings = profile)),
        ]);
        unlisteners.push(...listeners);

        if (mode === "offer") {
          phase = "available";
          // The tray opens the dialog only when a check already found an offer; re-check silently if
          // nothing is cached in this window yet.
          if (!offer) void check();
        } else {
          await check();
        }
      } finally {
        requestAnimationFrame(() => requestAnimationFrame(() => void events.emitPageReady()));
      }
    })();

    window.addEventListener("keydown", onKeydown);
    return () => {
      window.removeEventListener("keydown", onKeydown);
      unlisteners.forEach((unlisten) => unlisten());
    };
  });

  let unlisteners: Array<() => void> = [];
</script>

<main class="update">
  <header class="head">
    <img src={appIcon} alt="" width="64" height="64" />
    <div class="titles">
      {#if phase === "checking"}
        <h1>{$translator("update.checking")}</h1>
      {:else if phase === "up_to_date"}
        <h1>{$translator("update.up_to_date")}</h1>
        <p>{$translator("update.up_to_date_body", { version: offer?.current ?? current })}</p>
      {:else if phase === "available" && offer}
        <h1>{$translator("update.available_title")}</h1>
        <p>
          {$translator("update.available_body", { version: offer.version, current: offer.current })}
        </p>
        {#if offer.skipped}
          <p class="skipped">{$translator("update.skipped_note")}</p>
        {/if}
      {:else if phase === "downloading" && offer}
        <h1>{$translator("update.downloading", { version: offer.version })}</h1>
      {:else if phase === "ready" && offer}
        <h1>{$translator("update.ready_title", { version: offer.version })}</h1>
      {:else if phase === "install_on_quit" && offer}
        <h1>{$translator("update.install_on_quit_title", { version: offer.version })}</h1>
      {:else if phase === "failed"}
        <h1>{$translator("update.failed_title")}</h1>
        <p>
          {failure === "offline"
            ? $translator("update.failed_offline")
            : failure === "verified"
              ? $translator("update.failed_verified")
              : $translator("update.failed_download")}
        </p>
      {/if}
    </div>
  </header>

  {#if phase === "available" && offer}
    <section class="notes">
      <h2>{$translator("update.whats_new")}</h2>
      <div class="scroll">
        <Markdown source={offer.notes} />
      </div>
    </section>
  {/if}

  {#if phase === "downloading"}
    <div
      class="progress"
      role="progressbar"
      aria-valuenow={percent}
      aria-valuemin="0"
      aria-valuemax="100"
    >
      <span style:width={`${percent}%`}></span>
    </div>
    <p class="percent">{percent}%</p>
  {/if}

  {#if settings}
    <label class="auto">
      <input
        type="checkbox"
        checked={settings.check_updates}
        onchange={(event) => void setAutoCheck(event.currentTarget.checked)}
      />
      <span>{$translator("about.automatic")}</span>
    </label>
  {/if}

  <footer class="actions">
    {#if phase === "available"}
      <button type="button" class="left" onclick={() => void skip()}>
        {$translator("update.skip")}
      </button>
      <span class="spacer"></span>
      <button type="button" disabled={!offer} onclick={() => void installOnQuit()}>
        {$translator("update.install_on_quit")}
      </button>
      <button type="button" class="primary" onclick={() => void install()}>
        {$translator("update.install_relaunch")}
      </button>
    {:else if phase === "ready"}
      <span class="spacer"></span>
      <button type="button" onclick={() => void close()}>{$translator("update.later")}</button>
      <button type="button" class="primary" onclick={() => void install()}>
        {$translator("update.install_relaunch")}
      </button>
    {:else if phase === "install_on_quit"}
      <span class="spacer"></span>
      <button type="button" onclick={() => void close()}>{$translator("update.ok")}</button>
      <button type="button" class="primary" onclick={() => void install()}>
        {$translator("update.install_relaunch_now")}
      </button>
    {:else if phase === "failed"}
      <span class="spacer"></span>
      <button type="button" onclick={() => void close()}>{$translator("update.ok")}</button>
      <button type="button" class="primary" onclick={() => void check()}>
        {$translator("update.retry")}
      </button>
    {:else if phase === "downloading"}
      <span class="spacer"></span>
      <button type="button" onclick={() => void close()}>{$translator("update.cancel")}</button>
    {:else}
      <span class="spacer"></span>
      <button type="button" class="primary" onclick={() => void close()}>
        {$translator("update.ok")}
      </button>
    {/if}
  </footer>
</main>

<style>
  .update {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100vh;
    padding: 20px 20px 16px;
    box-sizing: border-box;
    background: var(--surface-opaque);
  }

  .head {
    display: flex;
    gap: 14px;
    align-items: flex-start;
  }

  .head img {
    border-radius: 14px;
    flex: none;
  }

  .titles h1 {
    margin: 2px 0 4px;
    font-size: 15px;
    font-weight: 600;
  }

  .titles p {
    margin: 0;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .skipped {
    margin-top: 4px;
    color: var(--accent-text);
  }

  .notes {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .notes h2 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 8px 10px;
    border-radius: 10px;
    background: var(--surface-raised);
  }

  .progress {
    height: 6px;
    border-radius: 3px;
    background: var(--surface-raised);
    overflow: hidden;
  }

  .progress span {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 120ms linear;
  }

  .percent {
    margin: 0;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .auto {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .spacer {
    flex: 1;
  }

  button {
    border: none;
    border-radius: 7px;
    padding: 6px 14px;
    background: var(--surface-raised);
    color: var(--text);
    font-size: 12px;
  }

  button.primary {
    background: var(--accent);
    color: var(--accent-contrast);
  }

  button.left {
    background: transparent;
    color: var(--text-secondary);
  }
</style>
