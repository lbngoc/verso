<script lang="ts">
  import { onMount, tick } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import Engines from "./Engines.svelte";
  import appIcon from "$lib/assets/app-icon.png";
  import { setUiLanguage, t as translator } from "$lib/i18n";
  import { initAppearance } from "$lib/stores/appearance";
  import { initTheme } from "$lib/stores/theme";
  import {
    commands,
    events,
    type EngineInfo,
    type Language,
    type ModuleInfo,
    type SettingsProfile,
    type UpdateInfo,
  } from "$lib/ipc";

  let settings = $state<SettingsProfile | null>(null);
  let engines = $state<EngineInfo[]>([]);
  let languages = $state<Language[]>([]);
  let modules = $state<ModuleInfo[]>([]);
  let permission = $state<"granted" | "denied" | "not_determined">("denied");
  let saving = $state(false);
  let saved = $state(false);
  let conflict = $state<string | null>(null);
  let shortcutTranslate = $state("");
  let shortcutQuick = $state("");
  let isMac = $state(true);
  let version = $state("");
  let updateState = $state<"idle" | "checking" | "up_to_date" | "available" | "downloading" | "ready" | "check_failed" | "error">(
    "idle",
  );
  let availableUpdate = $state<UpdateInfo | null>(null);
  let updatePercent = $state(0);
  let diagnosticsCopied = $state(false);

  async function checkNow(): Promise<void> {
    updateState = "checking";
    try {
      const info = await commands.checkForUpdate();
      availableUpdate = info;
      updateState = info ? "available" : "up_to_date";
    } catch {
      updateState = "check_failed";
    }
  }

  async function installUpdate(): Promise<void> {
    updatePercent = 0;
    updateState = "downloading";
    try {
      await commands.installUpdate();
      updateState = "ready";
    } catch {
      updateState = "error";
    }
  }

  async function reportProblem(): Promise<void> {
    await commands.reportProblem().catch(() => undefined);
  }

  async function copyDiagnostics(): Promise<void> {
    await commands.copyDiagnostics().catch(() => undefined);
    diagnosticsCopied = true;
    setTimeout(() => (diagnosticsCopied = false), 2000);
  }

  async function scrollToAbout(): Promise<void> {
    // The card is below the cards that only exist once the settings load, so wait for the DOM and
    // scroll twice: the icon can still change the page height after the first attempt.
    await tick();
    const scroll = () => document.getElementById("about")?.scrollIntoView({ block: "start" });
    scroll();
    setTimeout(scroll, 150);
  }

  async function refreshEngines(): Promise<void> {
    engines = await commands.listEngines().catch(() => []);
  }

  async function update(patch: Record<string, unknown>): Promise<void> {
    saving = true;
    try {
      settings = await commands.updateSettings(patch);
      setUiLanguage(settings.ui_language);
      saved = true;
      setTimeout(() => (saved = false), 1600);
      refreshEngines();
    } finally {
      saving = false;
    }
  }

  async function updateShorts(): Promise<void> {
    if (!settings) return;
    if (
      shortcutTranslate === settings.shortcuts.translate &&
      shortcutQuick === settings.shortcuts.quick
    ) {
      return;
    }
    await update({
      shortcuts: { translate: shortcutTranslate, quick: shortcutQuick },
    });
  }

  async function chooseEngines(order: string[], disabled: string[]): Promise<void> {
    await update({ engine_order: order, engines_disabled: disabled });
  }

  onMount(() => {
    let cancelled = false;
    let unlisteners: Array<() => void> = [];
    void (async () => {
      const appearance = await initAppearance();
      isMac = appearance.os === "macos";
      settings = await initTheme();
      if (!settings) settings = await commands.getSettings();
      version = await getVersion().catch(() => "");
      if (cancelled) return;
      const section = await commands.settingsSection().catch(() => null);
      if (section === "about" || location.hash === "#about") void scrollToAbout();
      shortcutTranslate = settings.shortcuts.translate;
      shortcutQuick = settings.shortcuts.quick;
      setUiLanguage(settings.ui_language);
      languages = await commands.listLanguages().catch(() => []);
      modules = await commands.listModules().catch(() => []);
      await refreshEngines();
      permission = await commands
        .checkPermissions("accessibility")
        .catch(() => "denied");
      if (cancelled) return;

      unlisteners = await Promise.all([
        events.onSettingsChanged((next) => {
          settings = next;
          shortcutTranslate = next.shortcuts.translate;
          shortcutQuick = next.shortcuts.quick;
          refreshEngines();
        }),
        events.onShortcutConflict((payload) => {
          conflict = payload.reason;
        }),
        events.onPermissionChanged((payload) => {
          if (payload.status === "granted" || payload.status === "denied") {
            permission = payload.status;
          }
        }),
        events.onSettingsSection((section) => {
          if (section === "about") void scrollToAbout();
        }),
        events.onUpdateAvailable((info) => {
          availableUpdate = info;
          if (updateState !== "downloading") updateState = "available";
        }),
        events.onUpdateProgress((payload) => {
          updatePercent = payload.percent;
        }),
      ]);
    })();
    return () => {
      cancelled = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  });
</script>

{#if settings}
  <main class="settings">
    <header class="head">
      <h1>{$translator("settings.title")}</h1>
      <span class="state" aria-live="polite">
        {saving ? $translator("settings.saving") : saved ? $translator("settings.saved") : ""}
      </span>
    </header>

    <section class="card">
      <h2>{$translator("settings.translation")}</h2>
      <label class="field">
        <span>{$translator("settings.target_language")}</span>
        <select
          value={settings.target_lang}
          onchange={(event) => update({ target_lang: event.currentTarget.value })}
        >
          {#each languages as language (language.code)}
            <option value={language.code}>{language.name_en}</option>
          {/each}
        </select>
      </label>
      <label class="field">
        <span>{$translator("settings.secondary_language")}</span>
        <select
          value={settings.secondary_lang}
          onchange={(event) => update({ secondary_lang: event.currentTarget.value })}
        >
          {#each languages as language (language.code)}
            <option value={language.code}>{language.name_en}</option>
          {/each}
        </select>
      </label>
      <p class="hint">{$translator("settings.secondary_hint")}</p>
    </section>

    <section class="card">
      <h2>{$translator("settings.engines")}</h2>
      <p class="hint">{$translator("settings.engines_hint")}</p>
      <Engines
        {engines}
        disabled={settings.engines_disabled}
        onchange={(order, disabled) => chooseEngines(order, disabled)}
      />
    </section>

    <section class="card">
      <h2>{$translator("settings.shortcuts")}</h2>
      <label class="field">
        <span>{$translator("settings.shortcut_translate")}</span>
        <input
          bind:value={shortcutTranslate}
          onblur={updateShorts}
          onkeydown={(event) => event.key === "Enter" && updateShorts()}
        />
      </label>
      <label class="field">
        <span>{$translator("settings.shortcut_quick")}</span>
        <input
          bind:value={shortcutQuick}
          onblur={updateShorts}
          onkeydown={(event) => event.key === "Enter" && updateShorts()}
        />
      </label>
      <p class="hint">{$translator("settings.shortcut_hint")}</p>
      {#if conflict}
        <p class="conflict" role="alert">
          {$translator("settings.shortcut_conflict", { shortcut: shortcutTranslate, reason: conflict })}
        </p>
      {/if}
    </section>

    <section class="card">
      <h2>{$translator("settings.appearance")}</h2>
      <label class="field">
        <span>{$translator("settings.theme")}</span>
        <select
          value={settings.theme}
          onchange={(event) => update({ theme: event.currentTarget.value })}
        >
          <option value="system">{$translator("settings.theme_system")}</option>
          <option value="light">{$translator("settings.theme_light")}</option>
          <option value="dark">{$translator("settings.theme_dark")}</option>
        </select>
      </label>
    </section>

    <section class="card">
      <h2>{$translator("settings.general")}</h2>
      <label class="row">
        <input
          type="checkbox"
          checked={settings.launch_at_login}
          onchange={(event) => update({ launch_at_login: event.currentTarget.checked })}
        />
        <span>{$translator("settings.launch_at_login")}</span>
      </label>
      <label class="field">
        <span>{$translator("settings.ui_language")}</span>
        <select
          value={settings.ui_language}
          onchange={(event) => update({ ui_language: event.currentTarget.value })}
        >
          <option value="en">English</option>
          <option value="vi">Tiếng Việt</option>
        </select>
      </label>
    </section>

    {#if isMac}
      <section class="card">
        <h2>{$translator("settings.permission")}</h2>
        <p class="row">
          <span class:ok={permission === "granted"}>
            {permission === "granted"
              ? $translator("settings.permission_granted")
              : $translator("settings.permission_missing")}
          </span>
          <button
            type="button"
            onclick={async () => {
              await commands.requestPermission("accessibility");
              permission = await commands.checkPermissions("accessibility");
            }}
          >
            {$translator("settings.permission_fix")}
          </button>
        </p>
      </section>
    {/if}

    {#if modules.length > 0}
      <section class="card">
        <h2>{$translator("settings.modules")}</h2>
        {#each modules as module (module.id)}
          <p class="module">
            <span>{module.name}</span>
            <span class="hint">
              {$translator("settings.module_unavailable", { spec: module.planned_in })}
            </span>
          </p>
        {/each}
      </section>
    {/if}

    <section class="card" id="about">
      <h2>{$translator("about.title")}</h2>
      <div class="about-row">
        <img class="about-icon" src={appIcon} alt="" width="64" height="64" />
        <div class="about-text">
          <p class="about-name">{$translator("app.name")}</p>
          <p class="hint">{$translator("about.full_name")}</p>
          <p class="hint">{$translator("about.version", { version })}</p>
        </div>
      </div>
      <p class="hint">{$translator("about.credits")}</p>

      <div class="update">
        <h3>{$translator("about.updates")}</h3>
        <label class="row">
          <input
            type="checkbox"
            checked={settings.check_updates}
            onchange={(event) => update({ check_updates: event.currentTarget.checked })}
          />
          <span>{$translator("about.automatic")}</span>
        </label>
        <p class="hint">{$translator("about.automatic_hint")}</p>

        <p class="update-state" aria-live="polite">
          {#if updateState === "checking"}
            {$translator("about.checking")}
          {:else if updateState === "up_to_date"}
            {$translator("about.up_to_date")}
          {:else if updateState === "available" && availableUpdate}
            {$translator("about.available", { version: availableUpdate.version })}
          {:else if updateState === "downloading"}
            {$translator("about.downloading", { percent: updatePercent })}
          {:else if updateState === "ready"}
            {$translator("about.ready")}
          {:else if updateState === "check_failed"}
            {$translator("about.check_failed")}
          {:else if updateState === "error"}
            {$translator("about.failed")}
          {/if}
        </p>

        {#if updateState === "available" && availableUpdate?.notes}
          <p class="notes">{availableUpdate.notes}</p>
        {/if}

        <div class="row">
          {#if updateState === "available"}
            <button type="button" class="primary" onclick={installUpdate}>
              {$translator("about.install")}
            </button>
          {/if}
          <button
            type="button"
            disabled={updateState === "checking" || updateState === "downloading"}
            onclick={checkNow}
          >
            {$translator("about.check_now")}
          </button>
        </div>

        <div class="row">
          <button type="button" onclick={reportProblem}>
            {$translator("about.report")}
          </button>
          <button type="button" onclick={copyDiagnostics}>
            {$translator("about.diagnostics")}
          </button>
        </div>
        {#if diagnosticsCopied}
          <p class="hint" aria-live="polite">{$translator("about.diagnostics_copied")}</p>
        {/if}
      </div>
    </section>
  </main>
{:else}
  <main class="settings"><p>{$translator("popup.loading")}</p></main>
{/if}

<style>
  .settings {
    max-width: 680px;
    margin: 0 auto;
    padding: 20px 24px 32px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }

  h1 {
    font-size: 18px;
    margin: 0;
  }

  h2 {
    font-size: 13px;
    margin: 0 0 8px;
    color: var(--text-secondary);
    font-weight: 600;
  }

  .card {
    border: 1px solid var(--stroke);
    border-radius: var(--radius-medium);
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .field,
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 13px;
  }

  .field select,
  .field input {
    min-width: 220px;
  }

  .row {
    justify-content: flex-start;
  }

  .hint {
    margin: 0;
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .conflict {
    margin: 0;
    font-size: 12px;
    color: var(--danger);
  }

  .ok {
    color: var(--success);
  }

  .state {
    font-size: 11px;
    color: var(--text-tertiary);
    min-height: 14px;
  }

  .module {
    display: flex;
    justify-content: space-between;
    margin: 0;
    font-size: 13px;
  }

  .about-row {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .about-icon {
    width: 64px;
    height: 64px;
    border-radius: 14px;
  }

  .about-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .about-name {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }

  .update {
    border-top: 1px solid var(--stroke);
    padding-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .update h3 {
    margin: 0;
    font-size: 12px;
    color: var(--text-secondary);
    font-weight: 600;
  }

  .update .row {
    flex-wrap: wrap;
  }

  .update-state {
    margin: 0;
    font-size: 12px;
    min-height: 15px;
  }

  .notes {
    margin: 0;
    font-size: 12px;
    color: var(--text-secondary);
    white-space: pre-wrap;
    max-height: 140px;
    overflow-y: auto;
  }

  .update .primary {
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
  }
</style>
