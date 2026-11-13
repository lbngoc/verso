import type { IconName } from "./icons";

/** One Settings sidebar entry (spec 009, contracts/settings-window.md). */
export interface SidebarItem {
  id: string;
  label: string;
  icon: IconName;
}

export interface SidebarGroup {
  title: string;
  items: SidebarItem[];
}
