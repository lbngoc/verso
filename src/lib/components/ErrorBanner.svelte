<script lang="ts">
  import { t as translator } from "$lib/i18n";
  import { errorTextKey, type EngineError } from "$lib/ipc";

  let {
    error,
    onsettings,
    onreport,
  }: { error: EngineError; onsettings?: () => void; onreport?: () => void } = $props();
  const key = $derived(errorTextKey(error));
</script>

<div class="error-banner" role="alert">
  <strong>{$translator(key)}</strong>
  {#if (error.kind === "rate_limited" || error.kind === "blocked") && onsettings}
    <p class="hint">
      <button type="button" onclick={onsettings}>{$translator("error.change_engine_order")}</button>
    </p>
  {/if}
  {#if error.kind === "blocked" && onreport}
    <p class="hint">
      <button type="button" onclick={onreport}>{$translator("error.report_problem")}</button>
    </p>
  {/if}
</div>

<style>
  .error-banner {
    border-radius: var(--radius-small);
    border: 1px solid var(--stroke);
    background: color-mix(in srgb, var(--danger) 10%, transparent);
    color: var(--text);
    padding: 10px 12px;
    font-size: 12px;
  }

  strong {
    font-weight: 600;
  }

  .hint {
    margin: 4px 0 0;
    color: var(--text-secondary);
  }

  .hint button {
    padding: 0;
    color: var(--accent);
    text-decoration: underline;
  }
</style>
