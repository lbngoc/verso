<script lang="ts">
  // A key label: the popup footer ("c", "s"), the panel hints ("↵", "Esc", "⌘,") and the settings key
  // reference. `active` marks a key that works right now; `muted` one that does not.
  let {
    label,
    muted = false,
    active = false,
  }: { label: string; muted?: boolean; active?: boolean } = $props();
</script>

<kbd class:muted class:active>{label}</kbd>

<style>
  kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 20px;
    padding: 0 6px;
    border-radius: 5px;
    background: var(--surface-opaque);
    /* A thin edge and a heavier bottom edge read as a key cap. */
    box-shadow:
      inset 0 0 0 1px var(--stroke),
      0 1px 0 var(--stroke);
    font: 600 11px/1 var(--font-ui);
    color: var(--text-secondary);
    transition:
      background 120ms ease,
      color 120ms ease;
  }

  kbd.active {
    background: var(--accent-soft);
    color: var(--accent-text);
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--accent) 55%, transparent),
      0 1px 0 color-mix(in srgb, var(--accent) 45%, transparent);
  }

  kbd.muted {
    opacity: 0.55;
    box-shadow: inset 0 0 0 1px var(--stroke);
  }

  @media (prefers-reduced-motion: reduce) {
    kbd {
      transition: none;
    }
  }
</style>
