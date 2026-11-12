<script lang="ts">
  import { t as translator } from "$lib/i18n";
  import type { NoticeItem } from "$lib/notices";

  let { items }: { items: NoticeItem[] } = $props();
</script>

{#if items.length > 0}
  <ul class="notices">
    {#each items as item (item.key + JSON.stringify(item.params ?? {}))}
      <li>{$translator(item.key, item.params)}</li>
    {/each}
  </ul>
{/if}

<style>
  .notices {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  li {
    font-size: 11px;
    line-height: 1.35;
    color: var(--text-secondary);
  }
</style>
