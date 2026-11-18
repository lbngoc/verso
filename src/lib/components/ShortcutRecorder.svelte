<script lang="ts">
  import { onDestroy } from "svelte";
  import Keycap from "./Keycap.svelte";
  import { t } from "$lib/i18n";
  import { acceleratorFromEvent, displayAccelerator } from "$lib/keys";
  import { commands, type BindingError, type SettingsProfile, type ShortcutAction } from "$lib/ipc";

  // Recorder for one global shortcut (spec 009, contracts/shortcuts.md).
  let {
    action,
    label,
    value,
    onchange,
  }: {
    action: ShortcutAction;
    label: string;
    value: string;
    onchange: (profile: SettingsProfile) => void;
  } = $props();

  const mac = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");
  let recording = $state(false);
  let refused = $state<string>("");
  let refuseTimer: ReturnType<typeof setTimeout> | null = null;
  // Not reactive: the listener lives outside Svelte's rendering.
  let listening = false;
  // The window listener and the button share one handler; the same event must count once.
  let lastEvent: KeyboardEvent | null = null;
  let field: HTMLButtonElement | undefined = $state();

  const shown = $derived(displayAccelerator(value, { mac }));

  function refuseMessage(error: BindingError): string {
    switch (error.reason) {
      case "no_modifier":
        return $t("shortcut.err.no_modifier");
      case "shift_only":
        return $t("shortcut.err.shift_only");
      case "duplicate":
        return $t("shortcut.err.duplicate", { action: error.action });
      case "reserved":
        return $t("shortcut.err.reserved");
      case "taken":
        return $t("shortcut.err.taken");
    }
  }

  function showRefusal(error: BindingError): void {
    refused = refuseMessage(error);
    if (refuseTimer) clearTimeout(refuseTimer);
    refuseTimer = setTimeout(() => (refused = ""), 4000);
  }

  function attach(): void {
    if (listening) return;
    listening = true;
    window.addEventListener("keydown", onKeydown, true);
  }

  function detach(): void {
    if (!listening) return;
    listening = false;
    window.removeEventListener("keydown", onKeydown, true);
  }

  async function startRecording(): Promise<void> {
    if (recording) return;
    recording = true;
    refused = "";
    // Listen first: the suspend call is async and the user may type immediately.
    attach();
    // Pressing the current binding must not fire the action while recording.
    await commands.suspendShortcuts(true).catch(() => undefined);
  }

  async function stopRecording(resume = true): Promise<void> {
    if (!recording) return;
    recording = false;
    detach();
    if (resume) await commands.suspendShortcuts(false).catch(() => undefined);
  }

  async function save(accel: string): Promise<void> {
    await stopRecording(true);
    try {
      const profile = await commands.setShortcut(action, accel);
      onchange(profile);
    } catch (error) {
      showRefusal(error as BindingError);
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event === lastEvent) return;
    lastEvent = event;
    if (!recording) return;
    // Modifier-only keys keep the recorder listening.
    const accel = acceleratorFromEvent(event, { mac });
    if (!accel && event.key !== "Escape") {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Escape") {
      void stopRecording(true);
      return;
    }
    void save(accel!);
  }

  function onFocusOut(event: FocusEvent): void {
    // Leaving the field cancels, but moving focus inside the recorder must not.
    const next = event.relatedTarget as Node | null;
    if (next && field?.parentElement?.contains(next)) return;
    if (recording) void stopRecording(true);
  }

  async function clear(): Promise<void> {
    await save("");
  }

  async function reset(): Promise<void> {
    await stopRecording(true);
    try {
      onchange(await commands.resetShortcut(action));
    } catch (error) {
      showRefusal(error as BindingError);
    }
  }

  onDestroy(() => {
    detach();
    if (refuseTimer) clearTimeout(refuseTimer);
    if (recording) void commands.suspendShortcuts(false).catch(() => undefined);
  });
</script>

<div class="recorder" class:refused={refused !== ""}>
  <button
    class="field"
    class:recording
    type="button"
    bind:this={field}
    onclick={() => void startRecording()}
    onkeydown={onKeydown}
    onfocusout={onFocusOut}
    aria-label={label}
    title={label}
  >
    {#if recording}
      <span class="recording-text">{$t("shortcut.recording")}</span>
    {:else if value}
      <Keycap label={shown} />
    {:else}
      <span class="unset">{$t("shortcut.not_set")}</span>
    {/if}
  </button>
  <button
    class="mini"
    type="button"
    title={$t("shortcut.reset")}
    aria-label={$t("shortcut.reset")}
    onclick={() => void reset()}
  >
    ↺
  </button>
  <button
    class="mini"
    type="button"
    title={$t("shortcut.clear")}
    aria-label={$t("shortcut.clear")}
    onclick={() => void clear()}
  >
    ⌫
  </button>
  {#if refused}
    <p class="error" role="alert">{refused}</p>
  {/if}
</div>

<style>
  .recorder {
    position: relative;
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
  }

  .field {
    min-width: 96px;
    height: var(--control-height);
    padding: 0 8px;
    border: none;
    border-radius: 6px;
    background: var(--surface-opaque);
    box-shadow: inset 0 0 0 1px var(--stroke);
    font-size: 12px;
    color: var(--text);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .field.recording {
    box-shadow:
      var(--focus-ring),
      inset 0 0 0 1px var(--accent);
    background: var(--accent-soft);
  }

  .recording-text,
  .unset {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .mini {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--text-tertiary);
    font-size: 12px;
  }

  .mini:hover {
    background: var(--hover);
    color: var(--text);
  }

  .error {
    position: absolute;
    top: 30px;
    right: 0;
    z-index: 2;
    margin: 0;
    padding: 4px 8px;
    border-radius: 6px;
    background: var(--surface-opaque);
    box-shadow:
      inset 0 0 0 1px var(--stroke),
      var(--shadow-menu);
    font-size: 11px;
    color: var(--danger);
    white-space: nowrap;
  }
</style>
