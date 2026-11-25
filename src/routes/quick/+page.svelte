<script lang="ts">
  import { onMount, tick } from "svelte";
  import LanguagePicker from "$lib/components/LanguagePicker.svelte";
  import ErrorBanner from "$lib/components/ErrorBanner.svelte";
  import PopupFooter from "$lib/components/PopupFooter.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import { setUiLanguage, t as translator } from "$lib/i18n";
  import { initAppearance } from "$lib/stores/appearance";
  import { initTheme } from "$lib/stores/theme";
  import EngineLabel from "$lib/components/EngineLabel.svelte";
  import type { FooterStatus } from "$lib/footerStatus";
  import { traceInput } from "$lib/inputTrace";
  import { panelFontPx } from "$lib/textSize";
  import { isRtl, type NoticeItem } from "$lib/notices";
  import { buildNotices } from "$lib/notices";
  import { quickKey, type QuickFocus } from "./keymap";
  import { initialSession, screenSession, type SessionEvent } from "./screenSession";
  import {
    commands,
    events,
    focusCurrentWindow,
    type CaptureFailure,
    type CaptureOutcome,
    type EngineError,
    type Language,
    type Origin,
    type QuickScreenEvent,
    type SettingsProfile,
    type TranslationResult,
    type AutoTranslate,
  } from "$lib/ipc";

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
  // One capture's state (reset rules in screenSession.ts); the fields below mirror it.
  let session = $state(initialSession("auto", "vi"));
  // The first translation of a capture carries the `screen` origin; later edits are typed.
  let nextOrigin: Origin = "typed";
  let copiedSource = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let throttleTimer: ReturnType<typeof setTimeout> | null = null;
  let slowTimer: ReturnType<typeof setTimeout> | null = null;
  let unlisteners: Array<() => void> = [];

  // The keys that work right now are tinted; the others stay quiet (FR-048).
  // Translate as you type (spec 004 FR-033): the delay after the last keystroke, none when off.
  const autoMode = $derived<AutoTranslate>(settings?.auto_translate ?? "slow");
  const AUTO_DELAY_MS: Record<AutoTranslate, number | null> = { fast: 400, slow: 1000, off: null };
  const NEXT_MODE: Record<AutoTranslate, AutoTranslate> = {
    fast: "slow",
    slow: "off",
    off: "fast",
  };

  async function cycleAutoMode(): Promise<void> {
    const next = NEXT_MODE[autoMode];
    const saved = await commands.updateSettings({ auto_translate: next }).catch(() => null);
    if (saved) settings = saved;
  }

  const notices = $derived(result ? buildNotices(result) : ([] as NoticeItem[]));

  const noticeText = $derived(
    notices.length ? $translator(notices[0].key, notices[0].params ?? {}) : "",
  );

  // The status bar's right side (FR-034): a notice when there is one, otherwise the
  // auto-translate mode or, when it is off, the Enter tip; a click cycles fast → slow → off.
  const footerStatus = $derived<FooterStatus>(
    noticeText
      ? { text: noticeText }
      : {
          text:
            autoMode === "off"
              ? $translator("quick.status_press_enter")
              : $translator("quick.status_auto", {
                  mode: $translator(`quick.auto_${autoMode}`),
                }),
          onclick: () => void cycleAutoMode(),
          title: $translator("quick.auto_cycle"),
        },
  );
  const rtl = $derived(result ? isRtl(result.direction_used.target, languages) : false);
  const direction = $derived(
    result
      ? `${(result.direction_used.source || "").toUpperCase()} → ${(result.direction_used.target || "").toUpperCase()}`
      : "",
  );
  const detectedLabel = $derived(detected ? detected.toUpperCase() : "");
  const mac = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");
  // One text size for the source field and the result (spec 004 FR-031).
  const panelFontSize = $derived(panelFontPx(step, mac));
  // Applies the reducer's state to the page fields it mirrors.
  function dispatch(event: SessionEvent): void {
    session = screenSession(session, event);
    if (
      event.phase === "reading" ||
      event.phase === "reset" ||
      event.phase === "text" ||
      event.phase === "no_text"
    ) {
      text = session.text;
      result = session.result;
      error = session.error;
      detected = session.detected;
      captureFailure = session.capture_note;
    }
    if (event.phase === "reading") {
      source = session.source_lang;
      target = session.target_lang;
    }
  }

  function clearTimers(): void {
    if (debounceTimer) clearTimeout(debounceTimer);
    if (throttleTimer) clearTimeout(throttleTimer);
    debounceTimer = null;
    throttleTimer = null;
    clearSlow();
  }

  async function onQuickScreen(event: QuickScreenEvent): Promise<void> {
    if (event.phase === "reset") {
      clearTimers();
      loading = false;
      copied = false;
      copiedSource = false;
      speaking = false;
      lastTranslated = "";
      nextOrigin = "typed";
      dispatch(event);
      await tick();
      textarea?.focus();
      return;
    }
    if (event.phase === "reading") {
      clearTimers();
      loading = false;
      copied = false;
      copiedSource = false;
      speaking = false;
      focus = "input";
      lastTranslated = "";
      lastRequestAt = 0;
      nextOrigin = "typed";
      dispatch(event);
      // The backend shows the panel once the reset state is on screen (FR-021).
      await tick();
      if (event.seq !== undefined) {
        void commands.quickScreenPrepared(event.seq).catch(() => undefined);
      }
      return;
    }
    dispatch(event);
    if (event.phase === "text" && event.text) {
      nextOrigin = "screen";
      lastTranslated = "";
      commands
        .detectLanguage(text)
        .then((language) => {
          if (language) detected = language;
        })
        .catch(() => undefined);
      void runTranslation();
      await tick();
      textarea?.focus();
      textarea?.setSelectionRange(text.length, text.length);
    }
  }

  async function copySource(): Promise<void> {
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      textarea?.select();
      document.execCommand("copy");
    }
    copiedSource = true;
    setTimeout(() => (copiedSource = false), 1500);
  }

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

    dispatch({ phase: "request" });
    const seq = session.request_seq;
    const origin = nextOrigin;
    nextOrigin = "typed";
    lastRequestAt = Date.now();
    lastTranslated = value;
    loading = true;
    error = null;
    clearSlow();
    slowTimer = setTimeout(() => (stillTranslating = true), 3000);

    try {
      const translated = await commands.translate(value, source, target, origin);
      if (seq !== session.request_seq) return;
      result = translated;
      if (translated.direction_used.source !== "auto") {
        detected = translated.direction_used.source;
      }
      void refreshVoice();
    } catch (failure) {
      if (seq !== session.request_seq) return;
      error = failure as EngineError;
      result = null;
    } finally {
      if (seq === session.request_seq) {
        loading = false;
        clearSlow();
      }
    }
  }

  function schedule(): void {
    if (debounceTimer) clearTimeout(debounceTimer);
    const delay = AUTO_DELAY_MS[autoMode];
    if (delay !== null) debounceTimer = setTimeout(runTranslation, delay);
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
    nextOrigin = "typed";
    // Typing replaces the "nothing found" note: the user is writing the text instead.
    if (session.phase === "no_text" || session.phase === "read_failed") {
      session = { ...session, phase: "idle" };
    }
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
        isComposing: event.isComposing,
        keyCode: event.keyCode,
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
      case "settings":
        event.preventDefault();
        void commands.openSettings().then(() => commands.hideQuick());
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
        session = initialSession(settings.source_lang, settings.target_lang);
      }

      // Settings changed elsewhere (the language and size apply without reopening the panel).
      unlisteners.push(
        await events.onSettingsChanged((next) => {
          settings = next;
          setUiLanguage(next.ui_language);
          step = next.text_size_step ?? 0;
        }),
      );

      unlisteners.push(await events.onQuickScreen((payload) => void onQuickScreen(payload)));

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
          nextOrigin = "typed";
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
      if (textarea) unlisteners.push(await traceInput(textarea));
      // Created on first use (T122): the listeners are in place, so the backend may show the panel
      // and send a queued prefill. Not inside requestAnimationFrame: WebKit pauses it while the
      // window is hidden, which it is until this call.
      void commands.quickReady().catch(() => undefined);
      requestAnimationFrame(() => requestAnimationFrame(() => events.emitSelfTestPainted()));
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

<main
  class="material-surface quick"
  style:--panel-text={`${panelFontSize}px`}
  style:--panel-line="1.45"
  onpointerdown={focusCurrentWindow}
>
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

  <div class="source">
    <textarea
      bind:this={textarea}
      bind:value={text}
      oninput={onInput}
      rows="3"
      placeholder={$translator("quick.placeholder")}
      aria-label={$translator("quick.input_label")}
      spellcheck="false"
    ></textarea>
    <button
      class="copy-source"
      type="button"
      disabled={!text}
      title={copiedSource ? $translator("quick.copied_source") : $translator("quick.copy_source")}
      aria-label={$translator("quick.copy_source")}
      onclick={() => void copySource()}
    >
      <Icon name={copiedSource ? "check" : "copy"} />
    </button>
  </div>

  {#if captureFailure}
    <p class="capture-note" role="status">{$translator(`capture.${captureFailure}`)}</p>
  {/if}
  {#if session.phase === "no_text" || session.phase === "read_failed"}
    <p class="capture-note" role="status">
      {$translator(session.phase === "no_text" ? "ocr.no_text" : "ocr.read_failed")}
      <button class="link" type="button" onclick={() => void commands.startScreenTranslate()}>
        {$translator("ocr.try_again")}
      </button>
    </p>
  {/if}

  <section
    class="result-area"
    class:focused={focus === "result"}
    tabindex="-1"
    bind:this={resultEl}
    aria-live="polite"
    aria-busy={loading}
  >
    {#if session.phase === "reading"}
      <div class="loading">
        <span class="spinner" aria-hidden="true"></span>
        <span>{$translator("ocr.reading")}</span>
      </div>
    {:else if error}
      <ErrorBanner
        {error}
        onsettings={() => commands.openSettings()}
        onreport={() => commands.reportProblem()}
      />
    {:else if result}
      <div class="result-meta">
        <span class="direction">{direction}</span>
        <EngineLabel engineId={result.engine_id} />
      </div>
      <p class="result-text" class:rtl>{result.text}</p>
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
    canCopy={Boolean(result)}
    status={footerStatus}
    keysActive={focus === "result" && Boolean(result)}
    oncopy={() => void copyResult()}
    onspeak={() => void speakResult()}
  />
</main>

<style>
  .quick {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100vh;
    box-sizing: border-box;
    padding: 14px 16px 0;
  }

  .quick-header {
    display: flex;
    align-items: center;
    gap: 8px;
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

  .source {
    position: relative;
    flex: none;
  }

  .copy-source {
    position: absolute;
    top: 6px;
    right: 6px;
    padding: 4px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
  }

  .copy-source:disabled {
    opacity: 0.35;
  }

  .copy-source:not(:disabled):hover {
    background: var(--surface-opaque);
    color: var(--text);
  }

  .link {
    border: none;
    background: none;
    padding: 0 0 0 6px;
    color: var(--accent);
    font-size: 11px;
  }

  .link:hover {
    text-decoration: underline;
  }

  /* Source field and result share one box and one text style (spec 004 FR-031): same radius,
     fill, stroke, padding, size and line height. */
  textarea,
  .result-area {
    box-sizing: border-box;
    border: none;
    border-radius: 10px;
    background: var(--surface-raised);
    box-shadow: inset 0 0 0 1px var(--stroke);
    padding: 10px 12px;
    font-family: inherit;
    font-size: var(--panel-text);
    line-height: var(--panel-line);
    color: var(--text);
  }

  textarea {
    width: 100%;
    resize: none;
    min-height: 72px;
    max-height: 104px;
    padding-right: 36px;
    user-select: text;
    flex: none;
  }

  textarea:focus {
    outline: none;
    box-shadow:
      var(--focus-ring),
      inset 0 0 0 1px var(--accent);
  }

  .result-area {
    flex: 1;
    min-height: 80px;
    overflow: auto;
    user-select: text;
  }

  .result-area.focused {
    box-shadow: var(--focus-ring);
  }

  .result-meta {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 6px;
  }

  .direction {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.06em;
    color: var(--text-secondary);
  }

  /* Wins over the platform stylesheets' larger popup result size: the panel result matches its
     source field (FR-031). */
  .result-area .result-text {
    margin: 0;
    font-size: inherit;
    line-height: inherit;
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
