<script lang="ts">
  import { onMount } from "svelte";
  import Card from "./Card.svelte";
  import SettingRow from "./SettingRow.svelte";
  import Switch from "./Switch.svelte";
  import { t } from "$lib/i18n";
  import {
    commands,
    events,
    type OcrAvailability,
    type PermissionStatus,
    type SettingsProfile,
  } from "$lib/ipc";

  // Settings › General › Screen translation (spec 004 FR-011, contracts/screen-result-panel.md §
  // Settings layout): the module switch, the recognition language, and why the module cannot be
  // used when it cannot. The shortcut lives in Shortcuts, the Screen Recording grant in
  // Permissions. Switching takes effect at once (shortcut, tray item, overlays).
  let {
    settings,
    onupdate,
    onpermissions,
  }: {
    settings: SettingsProfile;
    onupdate: (patch: Record<string, unknown>) => Promise<void>;
    onpermissions: () => void;
  } = $props();

  let availability = $state<OcrAvailability | null>(null);
  let permission = $state<PermissionStatus>("granted");
  let refusal = $state("");
  const mac = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");

  const available = $derived(availability?.available ?? false);
  const enabled = $derived(settings.ocr.enabled);
  const unsupported = $derived(availability?.reason === "unsupported");
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

{#if !unsupported}
  <Card title={$t("ocr.card_title")}>
    <SettingRow label={$t("ocr.enable")} helper={$t("ocr.enable_hint")}>
      <Switch
        checked={enabled}
        disabled={!available && !enabled}
        label={$t("ocr.enable")}
        onchange={(checked) => void setEnabled(checked)}
      />
    </SettingRow>
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
    <SettingRow label={$t("ocr.language")} helper={enabled ? "" : $t("ocr.language_off_hint")}>
      <select
        class:dim={!enabled}
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
    {#if mac && enabled && available && permission !== "granted"}
      <button class="link" type="button" onclick={onpermissions}>
        {$t("ocr.permission_link")}
      </button>
    {/if}
  </Card>
{/if}

<style>
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

  .dim {
    opacity: 0.5;
  }

  .notice {
    margin: 0;
    padding: 6px 0;
    font-size: 12px;
    color: var(--warning);
  }

  .hint {
    display: block;
    margin-top: 2px;
    color: var(--text-secondary);
  }

  .link {
    justify-content: flex-start;
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    font-size: 12px;
    text-align: left;
  }

  .link:hover {
    text-decoration: underline;
  }
</style>
