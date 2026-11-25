<script lang="ts">
  import Button from "$lib/components/Button.svelte";
  import Card from "$lib/components/Card.svelte";
  import SettingRow from "$lib/components/SettingRow.svelte";
  import Switch from "$lib/components/Switch.svelte";
  import ShortcutRecorder from "$lib/components/ShortcutRecorder.svelte";
  import { t } from "$lib/i18n";
  import { commands, type SettingsProfile } from "$lib/ipc";
  import { onMount } from "svelte";

  // Settings → Input → Shortcuts (spec 009, contracts/shortcuts.md).
  let {
    settings,
    onchange,
  }: { settings: SettingsProfile; onchange: (profile: SettingsProfile) => void } = $props();

  // The screen-region row is hidden on a platform without the module (Linux).
  let ocrSupported = $state(false);
  onMount(() => {
    void commands
      .ocrAvailability()
      .then((ocr) => (ocrSupported = ocr.reason !== "unsupported"))
      .catch(() => undefined);
  });

  const speakCollided = $derived(
    settings.shortcuts.speak === "" &&
      (settings.shortcuts.translate === "Alt+Shift+S" ||
        settings.shortcuts.quick === "Alt+Shift+S") &&
      !settings.notes_seen.includes("speak-unbound"),
  );

  // Revision 2: a missing Quick binding took the new default `Alt+Shift+T`, which an old saved
  // Translate (or Speak) binding already uses, so Quick was left unbound.
  const quickCollided = $derived(
    settings.shortcuts.quick === "" &&
      (settings.shortcuts.translate === "Alt+Shift+T" ||
        settings.shortcuts.speak === "Alt+Shift+T") &&
      !settings.notes_seen.includes("quick-unbound"),
  );

  async function setClipboardPrefill(enabled: boolean): Promise<void> {
    const profile = await commands
      .updateSettings({ quick_paste_clipboard: enabled })
      .catch(() => null);
    if (profile) onchange(profile);
  }

  async function dismissNote(note: "speak-unbound" | "quick-unbound"): Promise<void> {
    const notes = [...settings.notes_seen, note];
    const profile = await commands.updateSettings({ notes_seen: notes }).catch(() => null);
    if (profile) onchange(profile);
  }
</script>

<div class="section">
  <Card title={$t("settings.card_global_shortcuts")}>
    <SettingRow label={$t("shortcut.translate")}>
      <ShortcutRecorder
        action="translate"
        label={$t("shortcut.translate")}
        value={settings.shortcuts.translate}
        {onchange}
      />
    </SettingRow>
    <SettingRow label={$t("shortcut.speak")}>
      <ShortcutRecorder
        action="speak"
        label={$t("shortcut.speak")}
        value={settings.shortcuts.speak}
        {onchange}
      />
    </SettingRow>
    <SettingRow label={$t("shortcut.quick")}>
      <ShortcutRecorder
        action="quick"
        label={$t("shortcut.quick")}
        value={settings.shortcuts.quick}
        {onchange}
      />
    </SettingRow>
    {#if ocrSupported}
      <SettingRow
        label={$t("ocr.shortcut_row")}
        description={settings.ocr.enabled ? "" : $t("ocr.shortcut_off_hint")}
      >
        <ShortcutRecorder
          action="screen"
          label={$t("ocr.shortcut_row")}
          value={settings.shortcuts.screen}
          disabled={!settings.ocr.enabled}
          {onchange}
        />
      </SettingRow>
    {/if}
  </Card>

  {#if speakCollided}
    <div class="note" role="status">
      <p>{$t("shortcut.note.speak_unbound")}</p>
      <Button variant="primary" size="small" onclick={() => void dismissNote("speak-unbound")}>
        {$t("settings.note_ok")}
      </Button>
    </div>
  {/if}

  {#if quickCollided}
    <div class="note" role="status">
      <p>{$t("shortcut.note.quick_unbound")}</p>
      <Button variant="primary" size="small" onclick={() => void dismissNote("quick-unbound")}>
        {$t("settings.note_ok")}
      </Button>
    </div>
  {/if}

  <Card title={$t("settings.card_quick_panel")}>
    <SettingRow label={$t("shortcut.paste_clipboard")}>
      <Switch
        checked={settings.quick_paste_clipboard}
        label={$t("shortcut.paste_clipboard")}
        onchange={(checked) => void setClipboardPrefill(checked)}
      />
    </SettingRow>
  </Card>
</div>

<style>
  .section {
    display: flex;
    flex-direction: column;
    gap: var(--group-gap);
  }

  .note {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-radius: 10px;
    background: var(--accent-soft);
    color: var(--text);
    font-size: var(--type-row-description-size);
  }

  .note p {
    margin: 0;
    flex: 1;
  }
</style>
