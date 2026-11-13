<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import { t } from "$lib/i18n";
  import type { ModuleInfo } from "$lib/ipc";

  let { modules }: { modules: ModuleInfo[] } = $props();
</script>

<Card title={$t("settings.section_modules")}>
  {#each modules as module (module.id)}
    <div class="row">
      <span>{module.name}</span>
      <span class="hint">
        {$t("settings.module_unavailable", { spec: module.planned_in })}
      </span>
    </div>
  {/each}
</Card>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 13px;
  }

  .hint {
    font-size: 11px;
    color: var(--text-secondary);
  }
</style>
