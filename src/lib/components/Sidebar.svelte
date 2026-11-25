<script lang="ts">
  import Icon from "./Icon.svelte";
  import { TINT_COLOURS, type SidebarGroup } from "$lib/nav";
  import { t } from "$lib/i18n";

  // Settings sidebar: grouped sections with icons, a selected row in the accent colour and a collapse
  // toggle next to the traffic lights (spec 009, contracts/settings-window.md). macOS draws System
  // Settings' coloured icon tiles; Windows keeps plain glyphs. A badge is a red count capsule, a dot
  // on the icon when collapsed (spec 011 R5).

  let {
    groups,
    current,
    collapsed,
    trafficLights = false,
    onselect,
    ontoggle,
  }: {
    groups: SidebarGroup[];
    current: string;
    collapsed: boolean;
    /** macOS draws the window buttons over the top-left corner of the sidebar (overlay title bar). */
    trafficLights?: boolean;
    onselect: (id: string) => void;
    ontoggle: () => void;
  } = $props();

  let nav = $state<HTMLElement | null>(null);

  // Arrow keys move the selection while the sidebar has focus; attached on mount so the <nav> keeps
  // its plain landmark role.
  $effect(() => {
    const element = nav;
    if (!element) return;
    element.addEventListener("keydown", onKeydown);
    return () => element.removeEventListener("keydown", onKeydown);
  });

  const flat = $derived(groups.flatMap((group) => group.items));

  function onKeydown(event: KeyboardEvent): void {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    event.preventDefault();
    const index = flat.findIndex((item) => item.id === current);
    const delta = event.key === "ArrowDown" ? 1 : -1;
    const next = flat[(index + delta + flat.length) % flat.length];
    if (next) onselect(next.id);
  }
</script>

<aside class="sidebar" class:collapsed class:lights={trafficLights}>
  <div class="topbar">
    <button
      class="collapse"
      type="button"
      onclick={ontoggle}
      title={collapsed ? $t("settings.expand_sidebar") : $t("settings.collapse_sidebar")}
    >
      <Icon name="sidebar" />
    </button>
  </div>
  <nav bind:this={nav} aria-label={$t("settings.title")}>
    {#each groups as group (group.title)}
      <p class="group">{group.title}</p>
      {#each group.items as item (item.id)}
        <button
          class="item"
          class:selected={item.id === current}
          type="button"
          onclick={() => onselect(item.id)}
        >
          <span
            class="glyph"
            class:tile={item.tint !== undefined}
            style:--tile={item.tint ? TINT_COLOURS[item.tint] : undefined}
          >
            <Icon name={item.icon} size={item.tint ? 12 : 16} />
            {#if item.badge}<span class="dot" aria-hidden="true"></span>{/if}
          </span>
          <span class="label">{item.label}</span>
          {#if item.badge}
            <span class="badge" aria-label={String(item.badge)}>{item.badge}</span>
          {/if}
        </button>
      {/each}
    {/each}
  </nav>
</aside>

<style>
  .sidebar {
    width: 220px;
    flex: none;
    display: flex;
    flex-direction: column;
    padding: 8px 10px 12px;
    overflow-y: auto;
    background: transparent;
  }

  .sidebar.collapsed {
    width: 64px;
  }

  .topbar {
    display: flex;
    justify-content: flex-end;
    height: 28px;
    padding-top: 4px;
  }

  .sidebar.collapsed .topbar {
    justify-content: center;
  }

  /* Collapsed, the sidebar is only as wide as the window buttons, so the toggle moves below them
     instead of sitting under the red, yellow and green dots. */
  .sidebar.lights.collapsed .topbar {
    height: 52px;
    align-items: flex-end;
  }

  .collapse {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-tertiary);
  }

  .collapse:hover {
    background: var(--hover);
    color: var(--text);
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding-top: 6px;
  }

  .group {
    margin: 12px 8px 4px;
    font-size: var(--type-caption-size);
    font-weight: 600;
    color: var(--text-tertiary);
  }

  .sidebar.collapsed .group {
    display: none;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 8px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--text);
    font-size: var(--type-sidebar-item-size);
    font-weight: var(--type-sidebar-item-weight);
    text-align: left;
  }

  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .glyph {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
    width: 20px;
    height: 20px;
  }

  /* System Settings' rounded tiles: a white glyph on the section colour (macOS only). */
  :global([data-os="macos"]) .glyph.tile {
    border-radius: 5px;
    background: var(--tile);
    color: var(--tile-glyph);
  }

  .badge {
    flex: none;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 9px;
    background: var(--badge-fill);
    color: var(--badge-text);
    font-size: var(--type-caption-size);
    font-weight: 600;
    line-height: 18px;
    text-align: center;
  }

  .dot {
    display: none;
    position: absolute;
    top: -3px;
    right: -3px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--badge-fill);
    box-shadow: 0 0 0 1.5px var(--surface-opaque);
  }

  .sidebar.collapsed .dot {
    display: block;
  }

  .sidebar.collapsed .badge {
    display: none;
  }

  .item:hover {
    background: var(--hover);
  }

  .item.selected {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  /* Windows Settings: a light fill, the normal text colour and an accent pill at the left edge. */
  :global([data-os="windows"]) .item.selected {
    position: relative;
    background: var(--hover);
    color: var(--text);
  }

  :global([data-os="windows"]) .item.selected::before {
    content: "";
    position: absolute;
    left: 0;
    top: 50%;
    width: 3px;
    height: 16px;
    margin-top: -8px;
    border-radius: 2px;
    background: var(--accent);
  }

  /* Contrast themes (forced colours): the system's own selection colours, so the row never turns
     dark-on-dark; the tile and the pill are not needed. */
  @media (forced-colors: active) {
    .item.selected,
    :global([data-os]) .item.selected {
      forced-color-adjust: none;
      background: Highlight;
      color: HighlightText;
    }

    :global([data-os]) .item.selected::before {
      display: none;
    }
  }

  /* System Settings: the selected section is an accent-filled row with white text. */
  :global([data-os="macos"]) .item.selected {
    background: var(--accent);
    color: var(--accent-contrast);
  }

  .sidebar.collapsed .item .label {
    display: none;
  }

  .sidebar.collapsed .item {
    justify-content: center;
  }
</style>
