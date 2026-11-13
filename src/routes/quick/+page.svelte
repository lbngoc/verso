<script lang="ts">
  import { onMount, tick } from "svelte";
  import LanguagePicker from "$lib/components/LanguagePicker.svelte";
  import ErrorBanner from "$lib/components/ErrorBanner.svelte";
  import PopupFooter from "$lib/components/PopupFooter.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import { setUiLanguage, t as translator } from "$lib/i18n";
  import { initAppearance } from "$lib/stores/appearance";
  import { initTheme } from "$lib/stores/theme";
  import { isRtl, type NoticeItem } from "$lib/notices";
  import { buildNotices } from "$lib/notices";
  import { quickKey, type QuickFocus } from "./keymap";
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
  let resultEl = $state<HTMLElement | null>(null);
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
  let speaking = $state(false);
  let step = $state(0);
  let focus = $state<QuickFocus>("input");
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
  const direction = $derived(
    result
      ? `${(result.direction_used.source || "").toUpperCase()} → ${(result.direction_used.target || "").toUpperCase()}`
      : "",
  );
  const detectedLabel = $derived(detected ? detected.toUpperCase() : "");
  const mac = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");
  const resultFontSize = $derived.by(() => {
    const sizes = [13, 14, 16, 18, 20, 22, 24];
    return sizes[Math.min(Math.max(step + 2, 0), sizes.length - 1)] + (mac ? 1 : 0);
  });
  const noticeText = $derived(
    notices.length ? $translator(notices[0].key, notices[0].params ?? {}) : "",
  );

  function clearSlow(): void {
    if (slowTimer) clearTimeout(slowTimer);
    slowTimer = null;
    stillTranslating = false;
  }

  async function refreshVoice(): Promise<void> {
    if (!result) return;
    await commands.voiceExists(result.target_lang).catch(() => null);
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
      void refreshVoice();
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
    commands
      .detectLanguage(text)
      .then((language) => {
        if (language) detected = language;
      })
      .catch(() => undefined);
  }

  function onInput(): void {
    lastTranslated = "";
    captureFailure = null;
    schedule();
  }

  function swap(): void {
    const previousSource = source === "auto" ? (detected ?? "en") : source;
    const previousTarget = target;
    source = previousTarget;
    target =
      previousSource === previousTarget ? (previousTarget === "en" ? "vi" : "en") : previousSource;
    lastTranslated = "";
    // Tab with a result moves the translation into the input and translates it back.
    if (result) {
      text = result.text;
      result = null;
      void tick().then(() => textarea?.focus());
    }
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
    if (speaking) {
      speaking = false;
      await commands.stopSpeaking().catch(() => undefined);
      return;
    }
    speaking = true;
    await commands.speak(result.text, result.target_lang).catch(() => undefined);
    setTimeout(() => (speaking = false), 1500);
  }

  function setStep(next: number): void {
    step = Math.min(4, Math.max(-2, next));
    void commands.setTextSizeStep(step).catch(() => undefined);
  }

  function onKeydown(event: KeyboardEvent): void {
    const action = quickKey(
      focus,
      {
        key: event.key,
        shiftKey: event.shiftKey,
        metaKey: event.metaKey,
        ctrlKey: event.ctrlKey,
        altKey: event.altKey,
      },
      Boolean(result),
    );
    switch (action.type) {
      case "none":
        return;
      case "swap":
        event.preventDefault();
        swap();
        break;
      case "close":
        event.preventDefault();
        void commands.hideQuick();
        break;
      case "translate":
        event.preventDefault();
        if (debounceTimer) clearTimeout(debounceTimer);
        if (throttleTimer) clearTimeout(throttleTimer);
        lastTranslated = "";
        void runTranslation();
        break;
      case "focus-result":
        event.preventDefault();
        focus = "result";
        resultEl?.focus();
        break;
      case "copy":
        event.preventDefault();
        void copyResult();
        break;
      case "speak":
        event.preventDefault();
        void speakResult();
        break;
      case "text-size":
        event.preventDefault();
        setStep(step + action.delta);
        break;
      case "reset-size":
        event.preventDefault();
        setStep(0);
        break;
      case "type":
        event.preventDefault();
        focus = "input";
        text += action.text;
        lastTranslated = "";
        void tick().then(() => {
          textarea?.focus();
          textarea?.setSelectionRange(text.length, text.length);
          schedule();
        });
        break;
    }
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
        step = settings.text_size_step ?? 0;
      }

      unlisteners.push(
        await events.onCaptureFinished((payload: CaptureOutcome) => {
          captureFailure = payload.failure;
        }),
      );

      // Clipboard prefill from the Translate shortcut with no selection (FR-011).
      unlisteners.push(
        await events.onQuickPrefill((payload) => {
          text = payload.text;
          lastTranslated = "";
          result = null;
          focus = "input";
          void tick().then(() => {
            textarea?.focus();
            void runTranslation();
          });
        }),
      );

      await tick();
      textarea?.focus();
      requestAnimationFrame(() =>
        requestAnimationFrame(() => events.emitSelfTestPainted()),
      );
    })();

    window.addEventListener("keydown", onKeydown, true);
    return () => {
      cancelled = true;
      window.removeEventListener("keydown", onKeydown, true);
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
        void runTranslation();
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
        void runTranslation();
      }}
    />
    <span class="spacer"></span>
    {#if detectedLabel}
      <span class="detected">{$translator("quick.detected", { language: detectedLabel })}</span>
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

  <textarea
    bind:this={textarea}
    bind:value={text}
    oninput={onInput}
    rows="3"
    placeholder={$translator("quick.placeholder")}
    aria-label={$translator("quick.input_label")}
    spellcheck="false"
  ></textarea>

  {#if captureFailure}
    <p class="capture-note" role="status">{$translator(`capture.${captureFailure}`)}</p>
  {/if}

  <section
    class="result-area"
    class:focused={focus === "result"}
    tabindex="-1"
    bind:this={resultEl}
    aria-live="polite"
    aria-busy={loading}
  >
    {#if error}
      <ErrorBanner
        {error}
        onsettings={() => commands.openSettings()}
        onreport={() => commands.reportProblem()}
      />
    {:else if result}
      <div class="result-meta">
        <span class="direction">{direction}</span>
        <span class="engine">{result.engine_id}</span>
      </div>
      <p class="result-text" class:rtl style:font-size={`${resultFontSize}px`}>{result.text}</p>
    {:else if loading}
      <div class="loading">
        <span class="spinner" aria-hidden="true"></span>
        <span>{$translator("popup.translating")}</span>
        {#if stillTranslating}
          <span class="muted">{$translator("popup.still_translating")}</span>
        {/if}
      </div>
    {:else}
      <p class="placeholder">{$translator("quick.result_placeholder")}</p>
    {/if}
  </section>

  <PopupFooter
    {copied}
    {speaking}
    canSpeak={Boolean(result)}
    notice={noticeText}
    hint={$translator("quick.hint")}
    oncopy={() => void copyResult()}
    onspeak={() => void speakResult()}
  />
</main>

<style>
  .quick {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 100vh;
    box-sizing: border-box;
    padding: 12px 14px 0;
  }

  .quick-header {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
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
    border: none;
    border-radius: 10px;
    background: var(--surface-raised);
    box-shadow: inset 0 0 0 1px var(--stroke);
    padding: 10px;
    font: inherit;
    color: var(--text);
    user-select: text;
    flex: none;
  }

  textarea:focus {
    outline: none;
    box-shadow: var(--focus-ring), inset 0 0 0 1px var(--accent);
  }

  .result-area {
    flex: 1;
    min-height: 72px;
    overflow: auto;
    user-select: text;
    border-radius: 8px;
    padding: 4px 6px;
  }

  .result-area.focused {
    box-shadow: var(--focus-ring);
  }

  .result-meta {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 4px;
  }

  .direction {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.06em;
    color: var(--text-secondary);
  }

  .engine {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .result-text {
    margin: 0;
    line-height: var(--line-height);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    color: var(--text);
  }

  .rtl {
    direction: rtl;
    text-align: right;
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

  .spinner {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 2px solid color-mix(in srgb, var(--accent) 30%, transparent);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation: none;
    }
  }

  .capture-note {
    margin: 0;
    font-size: 11px;
    color: var(--text-secondary);
  }
</style>
