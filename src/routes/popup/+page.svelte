<script lang="ts">
  import { tick, onMount } from "svelte";
  import CaptureFailed from "$lib/components/CaptureFailed.svelte";
  import ErrorBanner from "$lib/components/ErrorBanner.svelte";
  import PopupHeader from "$lib/components/PopupHeader.svelte";
  import PopupFooter from "$lib/components/PopupFooter.svelte";
  import { setUiLanguage, t as translator } from "$lib/i18n";
  import { initAppearance } from "$lib/stores/appearance";
  import { initTheme } from "$lib/stores/theme";
  import { buildNotices, isRtl, terminalModifier, type NoticeItem } from "$lib/notices";
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

  let popupState = $state<PopupState>({ state: "hidden" });
  let result = $state<TranslationResult | null>(null);
  let error = $state<EngineError | null>(null);
  let capture = $state<CaptureOutcome | null>(null);
  let settings = $state<SettingsProfile | null>(null);
  let languages = $state<Language[]>([]);
  let copied = $state(false);
  let speaking = $state(false);
  let pinned = $state(false);
  let step = $state(0);
  let stillTranslating = $state(false);
  let voiceAvailable = $state<boolean | null>(null);
  let root = $state<HTMLElement | null>(null);
  let unlisteners: Array<() => void> = [];
  let slowTimer: ReturnType<typeof setTimeout> | null = null;
  let sizeTimer: ReturnType<typeof setTimeout> | null = null;
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  const phase = $derived.by(() => {
    if (popupState.state === "status") return "status";
    if (error) return "error";
    if (result) return "result";
    if (popupState.state === "capture_failed") return "capture_failed";
    return "status";
  });

  const statusKind = $derived(popupState.state === "status" ? popupState.kind : "translating");

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

  const noticeText = $derived(
    notices.length ? `${$translator(notices[0].key, notices[0].params ?? {})}` : "",
  );

  const direction = $derived(
    result
      ? `${(result.direction_used.source || "").toUpperCase()} → ${(result.direction_used.target || "").toUpperCase()}`
      : "— → —",
  );

  const rtl = $derived(result ? isRtl(result.direction_used.target, languages) : false);

  // Result font: 13/14/16/18/20/22/24 px, one point larger on macOS (R11).
  const resultFontSize = $derived.by(() => {
    const sizes = [13, 14, 16, 18, 20, 22, 24];
    const base = sizes[Math.min(Math.max(step + 2, 0), sizes.length - 1)];
    return `${base + (isMac ? 1 : 0)}px`;
  });

  const isMac = $derived(typeof navigator !== "undefined" && navigator.userAgent.includes("Mac"));

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
    // A selection inside the popup keeps the native copy.
    if (window.getSelection()?.toString()) return;
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
    try {
      await commands.speak(result.text, result.target_lang);
    } catch {
      /* the disabled state already explains a missing voice */
    }
    setTimeout(() => (speaking = false), 1500);
  }

  function setStep(next: number): void {
    step = Math.min(4, Math.max(-2, next));
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      void commands.setTextSizeStep(step).catch(() => undefined);
    }, 400);
    requestSize();
  }

  async function setPinned(next: boolean): Promise<void> {
    pinned = next;
    await commands.setPopupPinned(pinned).catch(() => undefined);
  }

  function clearSlowTimer(): void {
    if (slowTimer) {
      clearTimeout(slowTimer);
      slowTimer = null;
    }
    stillTranslating = false;
  }

  /** Reports the measured content so Rust grows the pill in place (R2/R3). */
  function requestSize(): void {
    if (sizeTimer) clearTimeout(sizeTimer);
    sizeTimer = setTimeout(() => {
      void (async () => {
        await tick();
        if (!root) return;
        if (phase === "status") {
          await commands.setPopupSize(200, 36).catch(() => undefined);
          return;
        }
        const long = (result?.text.length ?? 0) > 240 || step > 1;
        const width = long ? 560 : 380;
        // Round up: a fractional content height used to clip the footer's last line.
        const height = Math.min(420, Math.max(96, Math.ceil(root.getBoundingClientRect().height)));
        await commands.setPopupSize(width, height).catch(() => undefined);
      })();
    }, 16);
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      commands.hidePopup();
      return;
    }
    if (event.metaKey || event.ctrlKey) {
      if (event.key.toLowerCase() === "c") {
        // Native copy of a selection; otherwise the whole translation.
        if (!window.getSelection()?.toString()) {
          event.preventDefault();
          void copyResult();
        }
      }
      return;
    }
    if (event.altKey) return;
    switch (event.key) {
      case "c":
        event.preventDefault();
        void copyResult();
        break;
      case "s":
        event.preventDefault();
        void speakResult();
        break;
      case "-":
        event.preventDefault();
        setStep(step - 1);
        break;
      case "=":
      case "+":
        event.preventDefault();
        setStep(step + 1);
        break;
      case "0":
        event.preventDefault();
        setStep(0);
        break;
    }
  }

  onMount(() => {
    let cancelled = false;
    void (async () => {
      await initAppearance();
      const loaded = await initTheme();
      languages = await commands.listLanguages().catch(() => []);
      settings = loaded ?? (await commands.getSettings().catch(() => null));
      if (cancelled) return;
      if (settings) {
        setUiLanguage(settings.ui_language);
        step = settings.text_size_step ?? 0;
      }

      unlisteners.push(
        await events.onPopupState((payload) => {
          popupState = payload;
          if (payload.state === "status") {
            pinned = false;
            if (payload.kind === "translating") {
              result = null;
              error = null;
              copied = false;
              clearSlowTimer();
              slowTimer = setTimeout(() => (stillTranslating = true), 3000);
            } else {
              // "Nothing selected" (Speak): no slow-translating hint.
              result = null;
              error = null;
              clearSlowTimer();
            }
            requestAnimationFrame(() => requestAnimationFrame(() => events.emitSelfTestPainted()));
          } else {
            clearSlowTimer();
          }
          requestSize();
        }),
      );

      unlisteners.push(
        await events.onTranslationStarted(() => {
          popupState = { state: "status", kind: "translating" };
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
            void refreshVoice();
          }
          requestSize();
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
      if (sizeTimer) clearTimeout(sizeTimer);
      if (saveTimer) clearTimeout(saveTimer);
    };
  });
</script>

<main
  class="material-surface popup phase-{phase}"
  class:rtl
  bind:this={root}
  onpointerdown={focusCurrentWindow}
  onpointerenter={() => commands.focusPopup().catch(() => undefined)}
>
  {#if phase === "status"}
    <div class="pill" aria-live="polite">
      {#if statusKind === "translating"}
        <span class="spinner" aria-hidden="true"></span>
        <span
          >{stillTranslating
            ? $translator("popup.still_translating")
            : $translator("popup.translating")}</span
        >
      {:else}
        <span>{$translator("popup.nothing_selected")}</span>
      {/if}
    </div>
  {:else if phase === "capture_failed"}
    <CaptureFailed
      failure={popupState.state === "capture_failed" ? popupState.failure : "nothing_selected"}
      terminalModifier={terminalModifier(capture?.frontmost_app)}
      ongrant={() => commands.requestPermission("accessibility")}
      onrecheck={() => commands.checkPermissions("accessibility")}
      onquick={() => commands.showQuick()}
    />
    <PopupFooter
      canSpeak={false}
      oncopy={() => undefined}
      onspeak={() => undefined}
      hint={$translator("popup.esc_hint")}
    />
  {:else}
    <PopupHeader
      {direction}
      engine={result?.engine_id ?? ""}
      {pinned}
      onpin={(value) => void setPinned(value)}
      onsize={(delta) => setStep(step + delta)}
      onclose={() => commands.hidePopup()}
    />
    <div class="body" class:rtl>
      {#if phase === "error" && error}
        <ErrorBanner
          {error}
          onsettings={() => commands.openSettings()}
          onreport={() => commands.reportProblem()}
        />
      {:else if result}
        <p class="result-text" style:font-size={resultFontSize}>{result.text}</p>
      {/if}
    </div>
    <PopupFooter
      {copied}
      {speaking}
      canSpeak={voiceAvailable !== false}
      notice={noticeText}
      oncopy={() => void copyResult()}
      onspeak={() => void speakResult()}
    />
  {/if}
</main>

<style>
  .popup {
    display: flex;
    flex-direction: column;
    min-height: 0;
    /* Size to the content so the page can report the height the window must grow to (R3). */
    height: auto;
  }

  .pill {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 12px;
    font-size: 12px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
  }

  .spinner {
    width: 13px;
    height: 13px;
    border-radius: 50%;
    border: 2px solid color-mix(in srgb, var(--accent) 30%, transparent);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
    flex: none;
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

  .body {
    padding: 10px 12px;
    overflow: auto;
    max-height: 320px;
  }

  .result-text {
    margin: 0;
    line-height: var(--line-height);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
    color: var(--text);
  }

  .rtl .result-text {
    direction: rtl;
    text-align: right;
  }

  /* Fade and scale on phase change, disabled with reduce-motion (T026). */
  .phase-result .body,
  .phase-error .body {
    animation: grow 120ms ease-out;
  }

  @keyframes grow {
    from {
      opacity: 0;
      transform: scale(0.98);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .phase-result .body,
    .phase-error .body {
      animation: none;
    }
  }
</style>
