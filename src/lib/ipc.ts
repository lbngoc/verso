import { invoke } from "@tauri-apps/api/core";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type Origin = "selection" | "typed";
export type Theme = "system" | "light" | "dark";
export type Material = "glass" | "vibrancy" | "acrylic" | "opaque";
export type UiTheme = "light" | "dark";

export type EngineError =
  | { kind: "no_connection" }
  | { kind: "timeout" }
  | { kind: "rate_limited" }
  | { kind: "blocked" }
  | { kind: "invalid_key" }
  | { kind: "unsupported_pair" }
  | { kind: "not_installed" }
  | { kind: "parse_error" }
  | { kind: "empty_input" }
  | { kind: "internal"; message: string };

export interface Direction {
  source: string;
  target: string;
}

export interface EngineAttempt {
  engine_id: string;
  outcome: { outcome: "ok" } | { outcome: "error"; error: EngineError };
  latency_ms: number;
}

export interface TranslationResult {
  text: string;
  source_lang: string;
  target_lang: string;
  engine_id: string;
  direction_used: Direction;
  attempts: EngineAttempt[];
  truncated: boolean;
  original_length: number | null;
  from_cache: boolean;
  offline: boolean;
  dictionary: unknown | null;
}

export type TranslationOutcome =
  { ok: true; value: TranslationResult } | { ok: false; error: EngineError };

export interface Shortcuts {
  translate: string;
  quick: string;
  speak: string;
}

export type Accent = "app" | "system";

export type PermissionKind = "accessibility" | "screen_recording";
export type PermissionStatus = "granted" | "denied" | "not_determined";
/** Why the permission step is shown (spec 010): a grant lost across an update has its own wording. */
export type PermissionReason = "missing" | "after_update";
export interface ResetOutcome {
  ok: boolean;
  status: PermissionStatus;
}

export type ShortcutAction = "translate" | "speak" | "quick";

/** Why the OS or the rules refused a recorded shortcut (`shortcut.err.*`). */
export type BindingError =
  | { reason: "no_modifier" }
  | { reason: "shift_only" }
  | { reason: "duplicate"; action: string }
  | { reason: "reserved" }
  | { reason: "taken"; detail: string };

export interface SettingsProfile {
  schema: number;
  target_lang: string;
  secondary_lang: string;
  source_lang: string;
  engine_order: string[];
  engines_disabled: string[];
  shortcuts: Shortcuts;
  theme: Theme;
  launch_at_login: boolean;
  ui_language: "en" | "vi";
  popup_expanded: boolean;
  first_run_engine_choice_at: string | null;
  check_updates: boolean;
  telemetry_opt_in: boolean;
  quick_paste_clipboard: boolean;
  accent: Accent;
  text_size_step: number;
  skipped_update_version: string | null;
  notes_seen: string[];
  [key: string]: unknown;
}

export interface AppInfo {
  version: string;
  build_date: string;
}

export interface UpdateInfo {
  version: string;
  current: string;
  notes: string;
  skipped: boolean;
}

export interface UpdateProgress {
  downloaded: number;
  total: number;
  percent: number;
}

export interface EngineAvailability {
  state: "available" | "unavailable" | "needs_key" | "needs_pack";
  reason?: string;
}

export interface EngineInfo {
  id: string;
  service: string;
  availability: EngineAvailability;
  priority: number;
  cooldown_remaining_s: number;
  enabled: boolean;
  timeout_ms: number;
}

export interface Language {
  code: string;
  name_en: string;
  name_native: string;
  rtl: boolean;
  google: string | null;
  bing: string | null;
}

export interface Appearance {
  os: "macos" | "windows";
  os_version: string;
  material: Material;
  theme: UiTheme;
  reduce_transparency: boolean;
  high_contrast: boolean;
  accent: string | null;
}

export type CaptureFailure =
  | "nothing_selected"
  | "permission_missing"
  | "secure_field"
  | "terminal_tui"
  | "elevated_window"
  | "timeout";

export interface CaptureOutcome {
  method: "accessibility" | "copy" | "none";
  text: string | null;
  clipboard: "untouched" | "restored" | "partially_restored" | "not_restored";
  skipped_formats: string[];
  failure: CaptureFailure | null;
  frontmost_app: string | null;
  duration_ms: number;
  copy_attempted: boolean;
}

export type StatusKind = "translating" | "nothing_selected";

export type PopupState =
  | { state: "hidden" }
  | { state: "status"; kind: StatusKind }
  | { state: "result" }
  | { state: "error"; error: EngineError }
  | { state: "capture_failed"; failure: CaptureFailure };

/** The panel becomes key only when the user clicks or types into it (FR-002, research P5). */
export async function focusCurrentWindow(): Promise<void> {
  try {
    await getCurrentWindow().setFocus();
  } catch {
    /* browser preview */
  }
}

export const commands = {
  translate: (text: string, source: string, target: string, origin: Origin = "typed") =>
    invoke<TranslationResult>("translate", { text, source, target, origin }),
  translateSelection: () => invoke<TranslationResult | null>("translate_selection"),
  getSettings: () => invoke<SettingsProfile>("get_settings"),
  settingsSection: () => invoke<string | null>("settings_section"),
  updateSettings: (patch: Record<string, unknown>) =>
    invoke<SettingsProfile>("update_settings", { patch }),
  listEngines: () => invoke<EngineInfo[]>("list_engines"),
  setEngineOrder: (order: string[]) => invoke<EngineInfo[]>("set_engine_order", { order }),
  speak: (text: string, lang: string) => invoke<void>("speak", { text, lang }),
  voiceExists: (lang: string) => invoke<boolean>("voice_exists", { lang }),
  checkPermissions: (kind: "accessibility" | "screen_recording") =>
    invoke<"granted" | "denied" | "not_determined">("check_permissions", { kind }),
  requestPermission: (kind: "accessibility" | "screen_recording") =>
    invoke<"granted" | "denied" | "not_determined">("request_permission", { kind }),
  /** Why the permission step for `kind` is open (spec 010). */
  permissionReason: (kind: PermissionKind) =>
    invoke<PermissionReason>("permission_reason", { kind }),
  /** "Reset and grant again": clears Verso's own entry and asks again (spec 010 R2). */
  resetPermission: (kind: PermissionKind) => invoke<ResetOutcome>("reset_permission", { kind }),
  detectLanguage: (text: string) => invoke<string | null>("detect_language", { text }),
  listLanguages: () => invoke<Language[]>("list_languages"),
  getAppearance: () => invoke<Appearance>("get_appearance"),
  hidePopup: () => invoke<void>("hide_popup"),
  showQuick: () => invoke<void>("show_quick"),
  hideQuick: () => invoke<void>("hide_quick"),
  openSettings: () => invoke<void>("open_settings"),
  openOnboarding: () => invoke<void>("open_onboarding"),
  onboardingStep: () => invoke<number>("onboarding_step"),
  setPopupSize: (width: number, height: number) =>
    invoke<void>("set_popup_size", { width, height }),
  setPopupPinned: (pinned: boolean) => invoke<void>("set_popup_pinned", { pinned }),
  focusPopup: () => invoke<void>("focus_popup"),
  setTextSizeStep: (step: number) => invoke<SettingsProfile>("set_text_size_step", { step }),
  suspendShortcuts: (suspended: boolean) => invoke<void>("suspend_shortcuts", { suspended }),
  setShortcut: (action: ShortcutAction, accel: string) =>
    invoke<SettingsProfile>("set_shortcut", { action, accel }),
  resetShortcut: (action: ShortcutAction) => invoke<SettingsProfile>("reset_shortcut", { action }),
  stopSpeaking: () => invoke<void>("stop_speaking"),
  completeEngineChoice: (enginesDisabled: string[], engineOrder: string[]) =>
    invoke<SettingsProfile>("complete_engine_choice", {
      enginesDisabled,
      engineOrder,
    }),
  checkForUpdate: () => invoke<UpdateInfo | null>("check_for_update"),
  installUpdate: () => invoke<void>("install_update"),
  installUpdateOnQuit: () => invoke<void>("install_update_on_quit"),
  skipUpdateVersion: (version: string) => invoke<void>("skip_update_version", { version }),
  openUpdateWindow: (mode: "check" | "offer") => invoke<void>("open_update_window", { mode }),
  updateMode: () => invoke<string>("update_mode"),
  appInfo: () => invoke<AppInfo>("app_info"),
  hideUpdateWindow: () => invoke<void>("hide_update_window"),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  /** The quick page has its listeners; the backend may show it and send a queued prefill. */
  quickReady: () => invoke<void>("quick_ready"),
  /** About links; the URLs live in `src-tauri/src/links.rs`. */
  openPublicPage: (page: "home" | "terms" | "privacy" | "licences") =>
    invoke<void>("open_public_page", { page }),
  reportProblem: () => invoke<string>("report_problem"),
  copyDiagnostics: () => invoke<string>("copy_diagnostics"),
};

export const events = {
  onPopupState: (handler: (payload: PopupState) => void): Promise<UnlistenFn> =>
    listen<PopupState>("popup-state", (event) => handler(event.payload)),
  onTranslationStarted: (
    handler: (payload: {
      origin: Origin;
      truncated: boolean;
      original_length: number | null;
    }) => void,
  ): Promise<UnlistenFn> =>
    listen("translation-started", (event) => handler(event.payload as never)),
  onTranslationFinished: (
    handler: (payload: TranslationResult | { error: EngineError }) => void,
  ): Promise<UnlistenFn> =>
    listen("translation-finished", (event) => handler(event.payload as never)),
  onEngineFallback: (
    handler: (payload: { from: string; to: string; reason: EngineError }) => void,
  ): Promise<UnlistenFn> => listen("engine-fallback", (event) => handler(event.payload as never)),
  onCaptureFinished: (handler: (payload: CaptureOutcome) => void): Promise<UnlistenFn> =>
    listen<CaptureOutcome>("capture-finished", (event) => handler(event.payload)),
  onAppearanceChanged: (handler: (payload: Appearance) => void): Promise<UnlistenFn> =>
    listen<Appearance>("appearance-changed", (event) => handler(event.payload)),
  onPermissionChanged: (
    handler: (payload: { kind: string; status: string }) => void,
  ): Promise<UnlistenFn> =>
    listen("permission-changed", (event) => handler(event.payload as never)),
  onSettingsChanged: (handler: (payload: SettingsProfile) => void): Promise<UnlistenFn> =>
    listen<SettingsProfile>("settings-changed", (event) => handler(event.payload)),
  onQuickPrefill: (handler: (payload: { text: string }) => void): Promise<UnlistenFn> =>
    listen<{ text: string }>("quick-prefill", (event) => handler(event.payload)),
  onSettingsSection: (handler: (payload: string) => void): Promise<UnlistenFn> =>
    listen<string>("settings-section", (event) => handler(event.payload)),
  onShortcutConflict: (
    handler: (payload: { shortcut: string; reason: string }) => void,
  ): Promise<UnlistenFn> => listen("shortcut-conflict", (event) => handler(event.payload as never)),
  onSelfTestPainted: (handler: () => void): Promise<UnlistenFn> =>
    listen("selftest-painted", () => handler()),
  emitSelfTestPainted: () => emit("selftest-painted", {}),
  emitPageReady: () => emit("page-ready", {}),
  onUpdateAvailable: (handler: (payload: UpdateInfo) => void): Promise<UnlistenFn> =>
    listen<UpdateInfo>("update-available", (event) => handler(event.payload)),
  onUpdateProgress: (handler: (payload: UpdateProgress) => void): Promise<UnlistenFn> =>
    listen<UpdateProgress>("update-progress", (event) => handler(event.payload)),
};

export const errorTextKey = (error: EngineError): string => {
  switch (error.kind) {
    case "no_connection":
      return "error.no_connection";
    case "timeout":
      return "error.timeout";
    case "rate_limited":
      return "error.rate_limited";
    case "blocked":
      return "error.blocked";
    case "invalid_key":
      return "error.invalid_key";
    case "unsupported_pair":
      return "error.unsupported_pair";
    case "not_installed":
      return "error.not_installed";
    case "parse_error":
      return "error.parse_error";
    case "empty_input":
      return "error.empty_input";
    case "internal":
      return "error.internal";
  }
};
