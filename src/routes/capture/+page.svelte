<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { setUiLanguage, t as translator } from "$lib/i18n";
  import { commands } from "$lib/ipc";
  import { initAppearance } from "$lib/stores/appearance";
  import { initTheme } from "$lib/stores/theme";

  // The region picker (spec 004 contracts/screen-translate.md § Overlay page). One of these pages
  // covers each monitor; the window label `capture-<index>` says which one this is. Pointer
  // coordinates are CSS pixels = logical points relative to the monitor's frame.
  const MIN_SIZE = 4;

  let monitor = 0;
  let start = $state<{ x: number; y: number } | null>(null);
  let current = $state<{ x: number; y: number } | null>(null);
  let hint = $state(true);

  const rect = $derived.by(() => {
    if (!start || !current) return null;
    const x = Math.min(start.x, current.x);
    const y = Math.min(start.y, current.y);
    return { x, y, width: Math.abs(current.x - start.x), height: Math.abs(current.y - start.y) };
  });

  function clamp(point: { x: number; y: number }): { x: number; y: number } {
    return {
      x: Math.min(Math.max(point.x, 0), window.innerWidth),
      y: Math.min(Math.max(point.y, 0), window.innerHeight),
    };
  }

  function onPointerDown(event: PointerEvent): void {
    if (event.button !== 0) {
      void commands.cancelRegion();
      return;
    }
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    start = clamp({ x: event.clientX, y: event.clientY });
    current = start;
    hint = false;
  }

  function onPointerMove(event: PointerEvent): void {
    if (start) current = clamp({ x: event.clientX, y: event.clientY });
  }

  function onPointerUp(event: PointerEvent): void {
    if (!start) return;
    current = clamp({ x: event.clientX, y: event.clientY });
    const done = rect;
    start = null;
    current = null;
    if (done && done.width >= MIN_SIZE && done.height >= MIN_SIZE) {
      void commands.finishRegion(monitor, done);
    } else {
      void commands.cancelRegion();
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      void commands.cancelRegion();
    }
  }

  onMount(() => {
    const label = getCurrentWindow().label;
    monitor = Number(label.split("-")[1] ?? 0) || 0;
    void (async () => {
      // A hidden webview does not run animation frames, so readiness is reported on a timer;
      // theme or language failures must never keep the overlay from showing.
      try {
        await initAppearance();
        const profile = await initTheme();
        if (profile) setUiLanguage(profile.ui_language);
      } catch {
        /* the scrim and crosshair do not depend on them */
      }
      setTimeout(() => void commands.overlayReady(monitor), 30);
    })();
    let timer = setTimeout(() => (hint = false), 2000);
    // A kept-warm overlay is shown later: the hint and the selection start over then.
    let unlisten: (() => void) | undefined;
    void listen("overlay-activate", () => {
      start = null;
      current = null;
      hint = true;
      clearTimeout(timer);
      timer = setTimeout(() => (hint = false), 2000);
    }).then((off) => (unlisten = off));
    window.addEventListener("keydown", onKeydown);
    return () => {
      unlisten?.();
      clearTimeout(timer);
      window.removeEventListener("keydown", onKeydown);
    };
  });
</script>

<div
  class="surface"
  class:selecting={rect !== null}
  role="application"
  aria-label={$translator("ocr.overlay_label")}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  oncontextmenu={(event) => {
    event.preventDefault();
    void commands.cancelRegion();
  }}
>
  {#if rect}
    <div
      class="hole"
      style:left="{rect.x}px"
      style:top="{rect.y}px"
      style:width="{rect.width}px"
      style:height="{rect.height}px"
    >
      <span class="size">{Math.round(rect.width)} × {Math.round(rect.height)}</span>
    </div>
  {/if}
  {#if hint}
    <p class="hint">{$translator("ocr.overlay_hint")}</p>
  {/if}
</div>

<style>
  :global(html),
  :global(body),
  :global(#app) {
    margin: 0;
    height: 100%;
    background: transparent;
    overflow: hidden;
  }

  .surface {
    position: fixed;
    inset: 0;
    background: var(--capture-scrim);
    cursor: crosshair;
    user-select: none;
    -webkit-user-select: none;
    touch-action: none;
  }

  /* While dragging, the hole is cut out of the scrim by a huge shadow around the selection. */
  .surface.selecting {
    background: transparent;
  }

  .hole {
    position: absolute;
    box-sizing: border-box;
    border: 1px solid var(--accent);
    box-shadow: 0 0 0 100vmax var(--capture-scrim);
    pointer-events: none;
  }

  .size {
    position: absolute;
    left: 0;
    top: calc(100% + 4px);
    padding: 1px 6px;
    border-radius: var(--radius-small);
    background: var(--surface-opaque);
    color: var(--text);
    font-size: 11px;
    white-space: nowrap;
  }

  .hint {
    position: absolute;
    top: 16px;
    left: 50%;
    transform: translateX(-50%);
    margin: 0;
    padding: 4px 12px;
    border-radius: var(--radius-medium);
    background: var(--surface-opaque);
    color: var(--text);
    font-size: 12px;
    pointer-events: none;
  }
</style>
