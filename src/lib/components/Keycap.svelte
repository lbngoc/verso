<script lang="ts">
  // A small key label: the popup footer ("c", "s"), the panel hint and the settings key reference.
  let { label, muted = false }: { label: string; muted?: boolean } = $props();
</script>

<kbd class:muted>{label}</kbd>

<style>
  kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 5px;
    background: var(--surface-opaque);
    box-shadow: inset 0 0 0 1px var(--stroke);
    font: 500 11px/1 var(--font-ui);
    color: var(--text-secondary);
  }

  kbd.muted {
    opacity: 0.7;
  }
</style>
