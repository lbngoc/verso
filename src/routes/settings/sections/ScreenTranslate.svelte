<script lang="ts">
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import PermissionGuide from "$lib/components/PermissionGuide.svelte";
  import SettingRow from "$lib/components/SettingRow.svelte";
  import ShortcutRecorder from "$lib/components/ShortcutRecorder.svelte";
  import Switch from "$lib/components/Switch.svelte";
  import { t } from "$lib/i18n";
  import {
    commands,
    events,
    type OcrAvailability,
    type PermissionStatus,
    type SettingsProfile,
  } from "$lib/ipc";

  // Settings › Screen Translate (spec 004 contracts/screen-translate.md § Settings): the module
  // switch, its shortcut and the recognition language, and why the module cannot be used when it
  // cannot. Switching takes effect at once (shortcut, tray item, overlays).
  let {
    settings,
    onupdate,
    onchange,
  }: {
    settings: SettingsProfile;
    onupdate: (patch: Record<string, unknown>) => Promise<void>;
    onchange: (profile: SettingsProfile) => void;
  } = $props();

  let availability = $state<OcrAvailability | null>(null);
  let permission = $state<PermissionStatus>("granted");
  let refusal = $state("");
  const mac = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");

  const available = $derived(availability?.available ?? false);
  const enabled = $derived(settings.ocr.enabled);
  const reasonKey = $derived(
    availability?.reason === "os_too_old"
      ? "ocr.unavailable_os_too_old"
      : availability?.reason === "no_language"
        ? "ocr.unavailable_no_language"
        : "",
  );

  // Language names come from the browser's own tables in the UI language; the system's name is
  // the fallback.
  function languageName(code: string, fallback: string): string {
    try {
      const names = new Intl.DisplayNames([settings.ui_language], { type: "language" });
      return names.of(code) ?? fallback;
    } catch {
      return fallback;
    }
  }

  async function refreshPermission(): Promise<void> {
    permission = mac
      ? await commands.checkPermissions("screen_recording").catch(() => "denied")
      : "granted";
  }

  async function setEnabled(next: boolean): Promise<void> {
    refusal = "";
    try {
      await onupdate({ ocr: { enabled: next } });
    } catch {
      refusal = $t(reasonKey || "ocr.unavailable_os_too_old");
    }
    await refreshPermission();
  }

  onMount(() => {
    let off: (() => void) | undefined;
    void (async () => {
      availability = await commands.ocrAvailability().catch(() => null);
      await refreshPermission();
      off = await events.onOcrChanged(() => void refreshPermission());
    })();
    return () => off?.();
  });
</script>

<div class="section">
  <Card title={$t("ocr.section_title")}>
    <SettingRow label={$t("ocr.enable")} helper={$t("ocr.enable_hint")}>
      <Switch
        checked={enabled}
        disabled={!available && !enabled}
        label={$t("ocr.enable")}
        onchange={(checked) => void setEnabled(checked)}
      />
    </SettingRow>
    <SettingRow label={$t("ocr.shortcut")}>
      <ShortcutRecorder
        action="screen"
        label={$t("ocr.shortcut")}
        value={settings.shortcuts.screen}
        disabled={!enabled}
        {onchange}
      />
    </SettingRow>
    <SettingRow label={$t("ocr.language")}>
      <select
        value={settings.ocr.language}
        disabled={!enabled}
        aria-label={$t("ocr.language")}
        onchange={(event) => void onupdate({ ocr: { language: event.currentTarget.value } })}
      >
        <option value="auto">{$t("ocr.language_auto")}</option>
        {#each availability?.languages ?? [] as language (language.code)}
          <option value={language.code}>{languageName(language.code, language.name)}</option>
        {/each}
      </select>
    </SettingRow>
  </Card>

  {#if availability && !availability.available && reasonKey}
    <p class="notice" role="status">
      {$t(reasonKey)}
      {#if availability.reason === "no_language"}
        <span class="hint">{$t("ocr.unavailable_no_language_hint")}</span>
      {/if}
    </p>
  {/if}
  {#if refusal}
    <p class="notice" role="alert">{refusal}</p>
  {/if}

  {#if mac && enabled && available && permission !== "granted"}
    <Card title={$t("permission.screen_recording_name")}>
      <PermissionGuide kind="screen_recording" reason="missing" bind:status={permission} />
    </Card>
  {/if}
</div>

<style>
  .section {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  select {
    height: var(--control-height);
    max-width: 220px;
    border: none;
    border-radius: 6px;
    padding: 0 8px;
    background: var(--surface-opaque);
    box-shadow: inset 0 0 0 1px var(--stroke);
    color: var(--text);
    font-size: 12px;
  }

  .notice {
    margin: 0;
    padding: 8px 12px;
    border-radius: 10px;
    background: var(--accent-soft);
    color: var(--text);
    font-size: 12px;
  }

  .hint {
    display: block;
    margin-top: 2px;
    color: var(--text-secondary);
  }
</style>
