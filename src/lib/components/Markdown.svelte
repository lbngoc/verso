<script lang="ts">
  import { onMount } from "svelte";
  import { renderMarkdown } from "$lib/markdown";
  import { commands } from "$lib/ipc";

  // Renders release notes through the in-house Markdown subset; links open in the browser.
  let { source }: { source: string } = $props();
  let host = $state<HTMLElement | null>(null);

  onMount(() => {
    if (!host) return;
    host.replaceChildren(
      renderMarkdown(source, document, (href) => {
        void commands.openUrl(href).catch(() => undefined);
      }),
    );
  });

  $effect(() => {
    // Re-render when the notes change (a second check in the same window).
    if (!host) return;
    host.replaceChildren(
      renderMarkdown(source, document, (href) => {
        void commands.openUrl(href).catch(() => undefined);
      }),
    );
  });
</script>

<div class="markdown" bind:this={host}></div>

<style>
  .markdown {
    font-size: 12px;
    line-height: 1.5;
    color: var(--text);
    user-select: text;
  }

  .markdown :global(h3) {
    margin: 12px 0 4px;
    font-size: 12px;
    font-weight: 600;
  }

  .markdown :global(h4) {
    margin: 10px 0 3px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .markdown :global(p) {
    margin: 6px 0;
  }

  .markdown :global(ul),
  .markdown :global(ol) {
    margin: 4px 0;
    padding-left: 18px;
  }

  .markdown :global(li) {
    margin: 2px 0;
  }

  .markdown :global(li.nested) {
    margin-left: 14px;
    list-style: circle;
  }

  .markdown :global(code) {
    padding: 1px 4px;
    border-radius: 4px;
    background: var(--surface-opaque);
    font-size: 11px;
  }

  .markdown :global(a) {
    color: var(--accent-text);
    text-decoration: underline;
  }
</style>
