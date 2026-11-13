<script lang="ts">
  import Icon from "./Icon.svelte";
  import type { SidebarGroup } from "$lib/nav";
  import { t } from "$lib/i18n";

  // Settings sidebar: grouped sections with icons, a selected row in the accent colour and a collapse
  // toggle next to the traffic lights (spec 009, contracts/settings-window.md).

  let {
    groups,
    current,
    collapsed,
    onselect,
    ontoggle,
  }: {
    groups: SidebarGroup[];
    current: string;
    collapsed: boolean;
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

<aside class="sidebar" class:collapsed>
  <div class="topbar">
    <button class="collapse" type="button" onclick={ontoggle} title={collapsed ? $t("settings.expand_sidebar") : $t("settings.collapse_sidebar")}>
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
          <Icon name={item.icon} />
          <span>{item.label}</span>
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
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-tertiary);
  }

  .sidebar.collapsed .group {
    display: none;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 8px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--text);
    font-size: 13px;
    text-align: left;
  }

  .item:hover {
    background: var(--hover);
  }

  .item.selected {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .sidebar.collapsed .item span {
    display: none;
  }

  .sidebar.collapsed .item {
    justify-content: center;
  }
</style>
