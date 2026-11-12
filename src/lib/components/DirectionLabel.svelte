<script lang="ts">
  let { source, target }: { source: string; target: string } = $props();
</script>

<span class="direction" aria-label="{source} to {target}">
  {source.toUpperCase()} → {target.toUpperCase()}
</span>

<style>
  .direction {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.03em;
    color: var(--text-secondary);
    white-space: nowrap;
  }
</style>
