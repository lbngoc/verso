<script lang="ts">
  import { onMount, tick } from "svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import { SECTION_TINTS, type SidebarGroup, type SidebarItem } from "$lib/nav";
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
  import {
    commands,
    events,
    type EngineInfo,
    type Language,
    type SettingsProfile,
    type UpdateStatus,
  } from "$lib/ipc";

  let settings = $state<SettingsProfile | null>(null);
  let engines = $state<EngineInfo[]>([]);
  let languages = $state<Language[]>([]);
  let saving = $state(false);
  let saved = $state(false);
  let conflict = $state<string | null>(null);
  let isMac = $state(true);
  let section = $state("languages");
  let collapsed = $state(false);
  let updateStatus = $state<UpdateStatus | null>(null);
  // The About badge: an offer the user has not skipped (spec 011 FR-013), live while Settings is open.
  const updateBadge = $derived(updateStatus?.offer && !updateStatus.offer.skipped ? 1 : 0);

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
    const groupsList: SidebarGroup[] = [
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
        items: [
          {
            id: "about",
            label: $translator("settings.section_about"),
            icon: "info",
            badge: updateBadge,
          },
        ],
      },
    ];
    // macOS draws each section on its coloured tile; Windows Settings uses plain glyphs.
    return groupsList.map((group) => ({
      ...group,
      items: group.items.map((item) => ({
        ...item,
        tint: isMac ? SECTION_TINTS[item.id] : undefined,
      })),
    }));
  });

  const sectionTitle = $derived(
    groups.flatMap((group) => group.items).find((item) => item.id === section)?.label ?? "",
  );

  function allowed(id: string): boolean {
    return groups.some((group) => group.items.some((item) => item.id === id));
  }

  function select(requested: string): void {
    // The Screen Translate section was folded into General, Shortcuts and Permissions (R14).
    const id = requested === "screen" ? "general" : requested;
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

        const requested = await commands.settingsSection().catch(() => null);
        const hash = location.hash.replace("#", "");
        const initial = requested ?? (hash || "languages");
        const wanted = initial === "screen" ? "general" : initial;
        section = allowed(wanted) ? wanted : "languages";

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
          events.onSettingsSection((next) => select(next)),
          events.onUpdateStatus((next) => (updateStatus = next)),
        ]);
        updateStatus = await commands.updateStatus().catch(() => null);
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
        <h1>{sectionTitle}</h1>
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
        <General {settings} onupdate={update} onpermissions={() => select("permissions")} />
      {:else if section === "appearance"}
        <Appearance {settings} onupdate={update} />
      {:else if section === "permissions"}
        <Permissions {settings} />
      {:else if section === "about"}
        <About {settings} {updateStatus} onupdate={update} />
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
    background: var(--settings-surface);
  }

  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
  }

  /* macOS: the section name in the toolbar area at the top of the content column (System Settings);
     Windows: the large page title at the top of the content (Windows Settings). */
  h1 {
    margin: 0;
    font-size: var(--type-page-title-size);
    font-weight: var(--type-page-title-weight);
    line-height: 1.25;
  }

  :global([data-os="windows"]) .head {
    margin: 6px 0 20px;
  }

  .state {
    font-size: var(--type-caption-size);
    color: var(--text-tertiary);
  }

  .stack {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .conflict {
    margin: 0;
    font-size: var(--type-row-description-size);
    color: var(--danger);
  }

  .loading {
    color: var(--text-secondary);
  }
</style>
