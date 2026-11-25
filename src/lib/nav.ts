import type { IconName } from "./icons";

/** The colour of a section's icon tile on macOS (System Settings style, spec 011 R5). */
export type SidebarTint = "blue" | "gray" | "green" | "orange" | "purple" | "red" | "teal";

/** One Settings sidebar entry (spec 009, contracts/settings-window.md; tint and badge spec 011). */
export interface SidebarItem {
  id: string;
  label: string;
  icon: IconName;
  /** macOS tile colour; Windows shows plain glyphs. */
  tint?: SidebarTint;
  /** A count shown as a red capsule (a dot on the icon when the sidebar is collapsed). */
  badge?: number;
}

export interface SidebarGroup {
  title: string;
  items: SidebarItem[];
}

/** The fixed tile colour of each section, so a section keeps its colour wherever it appears. */
export const SECTION_TINTS: Record<string, SidebarTint> = {
  languages: "blue",
  engines: "purple",
  shortcuts: "gray",
  popup: "teal",
  general: "gray",
  appearance: "orange",
  permissions: "blue",
  about: "gray",
};

/** Tile fills: the macOS system colours, defined in tokens.css. */
export const TINT_COLOURS: Record<SidebarTint, string> = {
  blue: "var(--tint-blue)",
  gray: "var(--tint-gray)",
  green: "var(--tint-green)",
  orange: "var(--tint-orange)",
  purple: "var(--tint-purple)",
  red: "var(--tint-red)",
  teal: "var(--tint-teal)",
};
