<script lang="ts">
  import { onMount, tick } from "svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import type { SidebarGroup, SidebarItem } from "$lib/nav";
  import { setUiLanguage, t as translator } from "$lib/i18n";
  import { initAppearance } from "$lib/stores/appearance";
  import { initTheme } from "$lib/stores/theme";
  import Shortcuts from "./sections/Shortcuts.svelte";
  import Languages from "./sections/Languages.svelte";
  import EnginesSection from "./sections/Engines.svelte";
  import General from "./sections/General.svelte";
  import Appearance from "./sections/Appearance.svelte";
  import PopupPanel from "./sections/PopupPanel.svelte";
  import Permissions from "./sections/Permissions.svelte";
  import About from "./sections/About.svelte";
  import { commands, events, type EngineInfo, type Language, type SettingsProfile } from "$lib/ipc";

  let settings = $state<SettingsProfile | null>(null);
  let engines = $state<EngineInfo[]>([]);
  let languages = $state<Language[]>([]);
  let permission = $state<"granted" | "denied" | "not_determined">("denied");
  let saving = $state(false);
  let saved = $state(false);
  let conflict = $state<string | null>(null);
  let isMac = $state(true);
  let section = $state("languages");
  let collapsed = $state(false);

  const groups = $derived.by<SidebarGroup[]>(() => {
    const appItems: SidebarItem[] = [
      { id: "general", label: $translator("settings.section_general"), icon: "info" },
      { id: "appearance", label: $translator("settings.section_appearance"), icon: "paintbrush" },
    ];
    if (isMac) {
      appItems.push({
        id: "permissions",
        label: $translator("settings.section_permissions"),
        icon: "shield",
      });
    }
    return [
      {
        title: $translator("settings.group_translation"),
        items: [
          { id: "languages", label: $translator("settings.section_languages"), icon: "globe" },
          { id: "engines", label: $translator("settings.section_engines"), icon: "puzzle" },
        ],
      },
      {
        title: $translator("settings.group_input"),
        items: [
          { id: "shortcuts", label: $translator("settings.section_shortcuts"), icon: "keyboard" },
          { id: "popup", label: $translator("settings.section_popup"), icon: "sidebar" },
        ],
      },
      {
        title: $translator("settings.group_app"),
        items: appItems,
      },
      {
        title: "",
        items: [{ id: "about", label: $translator("settings.section_about"), icon: "info" }],
      },
    ];
  });

  function allowed(id: string): boolean {
    return groups.some((group) => group.items.some((item) => item.id === id));
  }

  function select(id: string): void {
    if (!allowed(id)) return;
    section = id;
    history.replaceState(null, "", `#${id}`);
    void tick().then(() => document.querySelector(".content")?.scrollTo({ top: 0 }));
  }

  async function update(patch: Record<string, unknown>): Promise<void> {
    saving = true;
    try {
      const next = await commands.updateSettings(patch);
      settings = next;
      setUiLanguage(next.ui_language);
      saved = true;
      setTimeout(() => (saved = false), 1600);
      engines = await commands.listEngines().catch(() => engines);
    } finally {
      saving = false;
    }
  }

  function onProfile(profile: SettingsProfile): void {
    settings = profile;
  }

  function onKeydown(event: KeyboardEvent): void {
    if ((event.metaKey || event.ctrlKey) && event.key === "\\") {
      event.preventDefault();
      collapsed = !collapsed;
    }
  }

  onMount(() => {
    let cancelled = false;
    let unlisteners: Array<() => void> = [];
    void (async () => {
      try {
        const appearance = await initAppearance();
        isMac = appearance.os === "macos";
        settings = await initTheme();
        if (!settings) settings = await commands.getSettings().catch(() => null);
        if (cancelled) return;
        const profile = settings;
        if (!profile) return;
        setUiLanguage(profile.ui_language);
        languages = await commands.listLanguages().catch(() => []);
        engines = await commands.listEngines().catch(() => []);
        permission = await commands.checkPermissions("accessibility").catch(() => "denied");

        const requested = await commands.settingsSection().catch(() => null);
        const hash = location.hash.replace("#", "");
        const initial = requested ?? (hash || "languages");
        section = allowed(initial) ? initial : "languages";

        unlisteners = await Promise.all([
          events.onSettingsChanged((next) => {
            settings = next;
            void commands
              .listEngines()
              .then((list) => (engines = list))
              .catch(() => undefined);
          }),
          events.onShortcutConflict((payload) => {
            conflict = payload.reason;
          }),
          events.onPermissionChanged((payload) => {
            if (payload.status === "granted" || payload.status === "denied") {
              permission = payload.status;
            }
          }),
          events.onSettingsSection((next) => select(next)),
        ]);
      } catch (error) {
        console.error("settings could not load", error);
      } finally {
        // The window is created hidden and shown on this event (R13): paint once, then reveal.
        await tick();
        requestAnimationFrame(() => requestAnimationFrame(() => void events.emitPageReady()));
      }
    })();

    window.addEventListener("keydown", onKeydown);
    return () => {
      cancelled = true;
      window.removeEventListener("keydown", onKeydown);
      unlisteners.forEach((unlisten) => unlisten());
    };
  });
</script>

<div class="settings-shell">
  <Sidebar
    {groups}
    current={section}
    {collapsed}
    trafficLights={isMac}
    onselect={select}
    ontoggle={() => (collapsed = !collapsed)}
  />
  <main class="content">
    {#if !settings}
      <p class="loading">{$translator("popup.loading")}</p>
    {:else}
      <header class="head">
        <h1>{$translator("settings.title")}</h1>
        <span class="state" aria-live="polite">
          {saving ? $translator("settings.saving") : saved ? $translator("settings.saved") : ""}
        </span>
      </header>

      {#if section === "languages"}
        <Languages {settings} {languages} onupdate={update} />
      {:else if section === "engines"}
        <EnginesSection {engines} disabled={settings.engines_disabled} onupdate={update} />
      {:else if section === "shortcuts"}
        <div class="stack">
          <Shortcuts {settings} onchange={onProfile} />
          {#if conflict}
            <p class="conflict" role="alert">{conflict}</p>
          {/if}
        </div>
      {:else if section === "popup"}
        <PopupPanel {settings} onchange={onProfile} />
      {:else if section === "general"}
        <General {settings} onupdate={update} />
      {:else if section === "appearance"}
        <Appearance {settings} onupdate={update} />
      {:else if section === "permissions"}
        <Permissions {permission} ongranted={(status) => (permission = status)} />
      {:else if section === "about"}
        <About {settings} onupdate={update} />
      {/if}
    {/if}
  </main>
</div>

<style>
  .settings-shell {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  .content {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: 18px 24px 28px;
    background: var(--surface-opaque);
  }

  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 14px;
  }

  h1 {
    margin: 0;
    font-size: 17px;
  }

  .state {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .stack {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .conflict {
    margin: 0;
    font-size: 12px;
    color: var(--danger);
  }

  .loading {
    color: var(--text-secondary);
  }
</style>
