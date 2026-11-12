<script lang="ts">
  import { onMount, tick } from "svelte";
  import LanguagePicker from "$lib/components/LanguagePicker.svelte";
  import ResultView from "$lib/components/ResultView.svelte";
  import ResultActions from "$lib/components/ResultActions.svelte";
  import ErrorBanner from "$lib/components/ErrorBanner.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import { setUiLanguage, t as translator } from "$lib/i18n";
  import { initAppearance } from "$lib/stores/appearance";
  import { initTheme } from "$lib/stores/theme";
  import { buildNotices, isRtl, type NoticeItem } from "$lib/notices";
  import {
    commands,
    events,
    focusCurrentWindow,
    type CaptureFailure,
    type CaptureOutcome,
    type EngineError,
    type Language,
    type SettingsProfile,
    type TranslationResult,
  } from "$lib/ipc";

  const DEBOUNCE_MS = 400;
  const MIN_REQUEST_INTERVAL_MS = 1000;

  let textarea: HTMLTextAreaElement | undefined = $state();
  let settings = $state<SettingsProfile | null>(null);
  let languages = $state<Language[]>([]);
  let source = $state("auto");
  let target = $state("vi");
  let text = $state("");
  let result = $state<TranslationResult | null>(null);
  let error = $state<EngineError | null>(null);
  let loading = $state(false);
  let stillTranslating = $state(false);
  let detected = $state<string | null>(null);
  let copied = $state(false);
  let voiceAvailable = $state<boolean | null>(null);
  let captureFailure = $state<CaptureFailure | null>(null);
  let lastTranslated = "";
  let lastRequestAt = 0;
  let requestSeq = 0;
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let throttleTimer: ReturnType<typeof setTimeout> | null = null;
  let slowTimer: ReturnType<typeof setTimeout> | null = null;
  let unlisteners: Array<() => void> = [];

  const notices = $derived(result ? buildNotices(result) : ([] as NoticeItem[]));
  const rtl = $derived(result ? isRtl(result.direction_used.target, languages) : false);
  const detectedLabel = $derived(
    detected ? `${$translator("quick.detected", { language: detected.toUpperCase() })}` : "",
  );

  function clearSlow(): void {
    if (slowTimer) clearTimeout(slowTimer);
    slowTimer = null;
    stillTranslating = false;
  }

  async function refreshVoice(): Promise<void> {
    if (!result) return;
    voiceAvailable = await commands.voiceExists(result.target_lang).catch(() => null);
  }

  async function runTranslation(): Promise<void> {
    const value = text.trim();
    if (!value) {
      result = null;
      error = null;
      lastTranslated = "";
      return;
    }
    if (value === lastTranslated && result) return;

    const wait = Math.max(0, MIN_REQUEST_INTERVAL_MS - (Date.now() - lastRequestAt));
    if (throttleTimer) clearTimeout(throttleTimer);
    if (wait > 0) {
      throttleTimer = setTimeout(runTranslation, wait);
      return;
    }

    const seq = ++requestSeq;
    lastRequestAt = Date.now();
    lastTranslated = value;
    loading = true;
    error = null;
    clearSlow();
    slowTimer = setTimeout(() => (stillTranslating = true), 3000);

    try {
      const translated = await commands.translate(value, source, target, "typed");
      if (seq !== requestSeq) return;
      result = translated;
      if (translated.direction_used.source !== "auto") {
        detected = translated.direction_used.source;
      }
      refreshVoice();
    } catch (failure) {
      if (seq !== requestSeq) return;
      error = failure as EngineError;
      result = null;
    } finally {
      if (seq === requestSeq) {
        loading = false;
        clearSlow();
      }
    }
  }

  function schedule(): void {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(runTranslation, DEBOUNCE_MS);
    commands.detectLanguage(text).then((language) => {
      if (language) detected = language;
    }).catch(() => undefined);
  }

  function onInput(): void {
    lastTranslated = "";
    captureFailure = null;
    schedule();
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      commands.hideQuick();
      return;
    }
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      if (debounceTimer) clearTimeout(debounceTimer);
      if (throttleTimer) clearTimeout(throttleTimer);
      lastTranslated = "";
      runTranslation();
    }
  }

  function swap(): void {
    const previousSource = source === "auto" ? (detected ?? "en") : source;
    const previousTarget = target;
    source = previousTarget;
    target = previousSource === previousTarget ? (previousTarget === "en" ? "vi" : "en") : previousSource;
    lastTranslated = "";
    runTranslation();
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
    await commands.speak(result.text, result.target_lang).catch(() => undefined);
  }

  onMount(() => {
    let cancelled = false;
    void (async () => {
      await initAppearance();
      settings = await initTheme();
      if (!settings) settings = await commands.getSettings().catch(() => null);
      languages = await commands.listLanguages().catch(() => []);
      if (cancelled) return;
      if (settings) {
        setUiLanguage(settings.ui_language);
        source = settings.source_lang;
        target = settings.target_lang;
      }

      unlisteners.push(
        await events.onCaptureFinished((payload: CaptureOutcome) => {
          captureFailure = payload.failure;
        }),
      );

      await tick();
      textarea?.focus();
      requestAnimationFrame(() => requestAnimationFrame(() => events.emitSelfTestPainted()));
    })();

    return () => {
      cancelled = true;
      unlisteners.forEach((unlisten) => unlisten());
      if (debounceTimer) clearTimeout(debounceTimer);
      if (throttleTimer) clearTimeout(throttleTimer);
      clearSlow();
    };
  });
</script>

<main class="material-surface quick" onpointerdown={focusCurrentWindow}>
  <header class="quick-header">
    <LanguagePicker
      {languages}
      value={source}
      label={$translator("quick.source")}
      allowAuto
      onChange={(code) => {
        source = code;
        lastTranslated = "";
        runTranslation();
      }}
    />
    <button
      class="swap"
      type="button"
      title={$translator("quick.swap")}
      aria-label={$translator("quick.swap")}
      onclick={swap}
    >
      <Icon name="swap" />
    </button>
    <LanguagePicker
      {languages}
      value={target}
      label={$translator("quick.target")}
      onChange={(code) => {
        target = code;
        lastTranslated = "";
        runTranslation();
      }}
    />
    <span class="spacer"></span>
    {#if detectedLabel}
      <span class="detected">{$translator("quick.detected", { language: detected?.toUpperCase() ?? "" })}</span>
    {/if}
    <button
      class="icon-button"
      type="button"
      onclick={() => commands.hideQuick()}
      title={$translator("quick.close")}
      aria-label={$translator("quick.close")}
    >
      <Icon name="close" />
    </button>
  </header>

  <hr class="hairline" />

  <textarea
    bind:this={textarea}
    bind:value={text}
    oninput={onInput}
    onkeydown={onKeydown}
    rows="3"
    placeholder={$translator("quick.placeholder")}
    aria-label={$translator("quick.input_label")}
    spellcheck="false"
  ></textarea>

  {#if captureFailure}
    <p class="capture-note" role="status">{$translator(`capture.${captureFailure}`)}</p>
  {/if}

  <section class="result-area" aria-live="polite" aria-busy={loading}>
    {#if error}
      <ErrorBanner {error} onsettings={() => commands.openSettings()} onreport={() => commands.reportProblem()} />
    {:else if result}
      <ResultView {result} sourceText={text} {notices} {rtl} />
    {:else if loading}
      <div class="loading">
        <span class="spinner" aria-hidden="true"></span>
        <span>{$translator("popup.loading")}</span>
        {#if stillTranslating}
          <span class="muted">{$translator("popup.still_translating")}</span>
        {/if}
      </div>
    {:else}
      <p class="placeholder">{$translator("quick.result_placeholder")}</p>
    {/if}
  </section>

  <footer class="action-bar">
    <ResultActions
      canCopy={Boolean(result)}
      canSpeak={Boolean(result)}
      language={result?.target_lang ?? ""}
      {voiceAvailable}
      {copied}
      oncopy={copyResult}
      onspeak={speakResult}
    />
    <span class="spacer"></span>
    <span class="esc-hint">{$translator("quick.hint")}</span>
  </footer>
</main>

<style>
  .quick-header {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .spacer {
    flex: 1;
  }

  .swap {
    padding: 4px;
    color: var(--text-secondary);
  }

  .detected {
    font-size: 11px;
    color: var(--text-tertiary);
    margin-right: 4px;
  }

  textarea {
    width: 100%;
    resize: none;
    min-height: 64px;
    max-height: 96px;
    border: 1px solid var(--stroke);
    border-radius: var(--radius-small);
    background: var(--surface-raised);
    padding: 8px;
    user-select: text;
  }

  .result-area {
    flex: 1;
    min-height: 72px;
    overflow: auto;
    user-select: text;
  }

  .placeholder {
    margin: 0;
    color: var(--text-tertiary);
    font-size: 12px;
  }

  .loading {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .capture-note {
    margin: 0;
    font-size: 11px;
    color: var(--text-secondary);
  }
</style>
