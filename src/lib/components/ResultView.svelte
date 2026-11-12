<script lang="ts">
  import DirectionLabel from "./DirectionLabel.svelte";
  import EngineLabel from "./EngineLabel.svelte";
  import Notices from "./Notices.svelte";
  import type { NoticeItem } from "$lib/notices";
  import type { TranslationResult } from "$lib/ipc";

  let {
    result,
    sourceText = "",
    expanded = false,
    notices = [],
    rtl = false,
  }: {
    result: TranslationResult;
    sourceText?: string;
    expanded?: boolean;
    notices?: NoticeItem[];
    rtl?: boolean;
  } = $props();
</script>

<section class="result" aria-live="polite">
  <header class="result-header">
    <DirectionLabel source={result.direction_used.source} target={result.direction_used.target} />
    <span class="spacer"></span>
    <EngineLabel engineId={result.engine_id} />
  </header>

  {#if expanded && sourceText}
    <p class="source-text" dir={rtl ? "auto" : "ltr"}>{sourceText}</p>
    <hr class="hairline" />
  {/if}

  <p class="result-text" dir={rtl ? "rtl" : "auto"}>{result.text}</p>

  <Notices items={notices} />
</section>

<style>
  .result {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
    overflow: auto;
    user-select: text;
  }

  .result-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .spacer {
    flex: 1;
  }

  .result-text {
    margin: 0;
    font-size: var(--font-size-result);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .source-text {
    margin: 0;
    font-size: 12px;
    color: var(--text-secondary);
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
