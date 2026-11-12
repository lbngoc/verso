<script lang="ts">
  import { onMount } from "svelte";
  import ResultView from "$lib/components/ResultView.svelte";
  import ResultActions from "$lib/components/ResultActions.svelte";
  import CaptureFailed from "$lib/components/CaptureFailed.svelte";
  import ErrorBanner from "$lib/components/ErrorBanner.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import { setUiLanguage, t as translator } from "$lib/i18n";
  import { initAppearance } from "$lib/stores/appearance";
  import { initTheme } from "$lib/stores/theme";
  import {
    buildNotices,
    isRtl,
    terminalModifier,
    type NoticeItem,
  } from "$lib/notices";
  import {
    commands,
    events,
    focusCurrentWindow,
    type CaptureOutcome,
    type EngineError,
    type Language,
    type PopupState,
    type SettingsProfile,
    type TranslationResult,
  } from "$lib/ipc";

  let popupState = $state<PopupState>({ state: "loading" });
  let result = $state<TranslationResult | null>(null);
  let error = $state<EngineError | null>(null);
  let capture = $state<CaptureOutcome | null>(null);
  let settings = $state<SettingsProfile | null>(null);
  let languages = $state<Language[]>([]);
  let expanded = $state(false);
  let copied = $state(false);
  let stillTranslating = $state(false);
  let voiceAvailable = $state<boolean | null>(null);
  let unlisteners: Array<() => void> = [];
  let slowTimer: ReturnType<typeof setTimeout> | null = null;

  const notices = $derived.by(() => {
    const items: NoticeItem[] = [];
    if (result) {
      items.push(...buildNotices(result));
      if (settings && result.direction_used.target !== settings.target_lang) {
        items.push({
          key: "notice.redirected",
          params: { source: result.direction_used.source, target: result.direction_used.target },
        });
      }
    }
    if (capture?.clipboard === "partially_restored") {
      items.push({
        key: "notice.partial_restore",
        params: { formats: capture.skipped_formats.join(", ") },
      });
    }
    if (capture?.clipboard === "not_restored") {
      items.push({ key: "notice.not_restored" });
    }
    return items;
  });

  const rtl = $derived(
    result ? isRtl(result.direction_used.target, languages) : false,
  );

  async function refreshVoice(): Promise<void> {
    if (!result || !settings || !languages.length) return;
    try {
      voiceAvailable = await commands.voiceExists(result.target_lang);
    } catch {
      voiceAvailable = null;
    }
  }

  async function copyResult(): Promise<void> {
    if (!result) return;
    try {
      await navigator.clipboard.writeText(result.text);
    } catch {
      const area = document.createElement("textarea");
      area.value = result.text;
      document.body.append(area);
      area.select();
      document.execCommand("copy");
      area.remove();
    }
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  async function speakResult(): Promise<void> {
    if (!result) return;
    try {
      await commands.speak(result.text, result.target_lang);
    } catch {
      /* the disabled state already explains a missing voice */
    }
  }

  async function toggleExpanded(): Promise<void> {
    expanded = !expanded;
    settings = await commands.setPopupExpanded(expanded).catch(() => settings);
  }

  function clearSlowTimer(): void {
    if (slowTimer) {
      clearTimeout(slowTimer);
      slowTimer = null;
    }
    stillTranslating = false;
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      commands.hidePopup();
      return;
    }
    const mod = event.metaKey || event.ctrlKey;
    if (!mod) return;
    if (event.key.toLowerCase() === "c" && !window.getSelection()?.toString()) {
      event.preventDefault();
      copyResult();
    }
    if (event.key.toLowerCase() === "e") {
      event.preventDefault();
      toggleExpanded();
    }
    if (event.key === "Enter") {
      event.preventDefault();
      speakResult();
    }
  }

  onMount(() => {
    let cancelled = false;
    void (async () => {
      await initAppearance();
      await initTheme();
      languages = await commands.listLanguages().catch(() => []);
      settings = await commands.getSettings().catch(() => null);
      if (cancelled) return;
      if (settings) setUiLanguage(settings.ui_language);
      expanded = settings?.popup_expanded ?? false;

        unlisteners.push(
        await events.onPopupState((payload) => {
          popupState = payload;
          if (payload.state === "loading") {
            result = null;
            error = null;
            copied = false;
            clearSlowTimer();
            slowTimer = setTimeout(() => (stillTranslating = true), 3000);
            requestAnimationFrame(() =>
              requestAnimationFrame(() => events.emitSelfTestPainted()),
            );
          } else {
            clearSlowTimer();
          }
        }),
      );

      unlisteners.push(
        await events.onTranslationStarted(() => {
          popupState = { state: "loading" };
        }),
      );

      unlisteners.push(
        await events.onTranslationFinished((payload) => {
          clearSlowTimer();
          if ("error" in payload) {
            error = payload.error;
            result = null;
          } else {
            result = payload;
            error = null;
            refreshVoice();
          }
        }),
      );

      unlisteners.push(
        await events.onCaptureFinished((payload) => {
          capture = payload;
          if (payload.failure === "secure_field" || payload.failure === "nothing_selected") {
            commands.hidePopup();
          }
        }),
      );
    })();

    window.addEventListener("keydown", onKeydown);
    return () => {
      cancelled = true;
      window.removeEventListener("keydown", onKeydown);
      unlisteners.forEach((unlisten) => unlisten());
      clearSlowTimer();
    };
  });
</script>

<main class="material-surface popup" onpointerdown={focusCurrentWindow}>
  {#if popupState.state === "capture_failed"}
    <CaptureFailed
      failure={popupState.failure}
      terminalModifier={terminalModifier(capture?.frontmost_app)}
      ongrant={() => commands.requestPermission("accessibility")}
      onrecheck={() => commands.checkPermissions("accessibility")}
      onquick={() => commands.showQuick()}
    />
    <div class="action-bar">
      <span class="spacer"></span>
      <button
        class="icon-button"
        type="button"
        title={$translator("popup.esc_hint")}
        aria-label={$translator("popup.close")}
        onclick={() => commands.hidePopup()}
      >
        <Icon name="close" />
      </button>
    </div>
  {:else if popupState.state === "hidden"}
    <p class="muted">{$translator("popup.loading")}</p>
  {:else if error}
    <ErrorBanner {error} onsettings={() => commands.openSettings()} onreport={() => commands.reportProblem()} />
    <div class="action-bar">
      <ResultActions
        canCopy={false}
        canSpeak={false}
        language={result?.target_lang ?? ""}
        {copied}
        oncopy={copyResult}
        onspeak={speakResult}
      />
      <span class="spacer"></span>
      <button
        class="icon-button"
        type="button"
        title={$translator("popup.esc_hint")}
        aria-label={$translator("popup.close")}
        onclick={() => commands.hidePopup()}
      >
        <Icon name="close" />
      </button>
    </div>
  {:else if result}
    <ResultView
      {result}
      sourceText={capture?.text ?? ""}
      {expanded}
      {notices}
      {rtl}
    />
    <div class="action-bar">
      <ResultActions
        {copied}
        language={result.target_lang}
        {voiceAvailable}
        oncopy={copyResult}
        onspeak={speakResult}
      />
      <span class="spacer"></span>
      <button
        class="icon-button"
        type="button"
        onclick={toggleExpanded}
        title={expanded ? $translator("popup.collapse") : $translator("popup.expand")}
        aria-label={expanded ? $translator("popup.collapse") : $translator("popup.expand")}
      >
        <Icon name="expand" />
      </button>
      <button
        class="icon-button"
        type="button"
        onclick={() => commands.hidePopup()}
        title={$translator("popup.esc_hint")}
        aria-label={$translator("popup.close")}
      >
        <Icon name="close" />
      </button>
    </div>
  {:else}
    <div class="loading" aria-live="polite">
      <span class="spinner" aria-hidden="true"></span>
      <div class="loading-text">
        <p>{$translator("popup.loading")}</p>
        {#if stillTranslating}
          <span class="muted">{$translator("popup.still_translating")}</span>
        {/if}
      </div>
    </div>
    <div class="action-bar">
      <span class="spacer"></span>
      <span class="esc-hint">{$translator("popup.esc_hint")}</span>
    </div>
  {/if}
</main>

<style>
  .loading {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: 1;
  }

  .loading-text p {
    margin: 0;
  }

  .loading-text span {
    font-size: 11px;
  }
</style>
