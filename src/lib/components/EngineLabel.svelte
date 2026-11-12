<script lang="ts">
  let { engineId, service }: { engineId: string; service?: string } = $props();

  const names: Record<string, string> = {
    "http-google": "Google",
    "http-bing": "Microsoft",
  };
  const label = $derived(service ?? names[engineId] ?? engineId);
</script>

<span class="engine" title={engineId}>{label}</span>

<style>
  .engine {
    font-size: 11px;
    color: var(--text-tertiary);
    white-space: nowrap;
  }
</style>
