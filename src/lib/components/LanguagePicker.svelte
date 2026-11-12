<script lang="ts">
  import Icon from "./Icon.svelte";
  import { t as translator } from "$lib/i18n";
  import type { Language } from "$lib/ipc";

  let {
    languages,
    value,
    label,
    allowAuto = false,
    autoLabel,
    onChange,
  }: {
    languages: Language[];
    value: string;
    label: string;
    allowAuto?: boolean;
    autoLabel?: string;
    onChange: (code: string) => void;
  } = $props();

  const RECENTS_KEY = "verso.recent-languages";

  let open = $state(false);
  let filter = $state("");

  function recents(): string[] {
    try {
      return JSON.parse(localStorage.getItem(RECENTS_KEY) ?? "[]") as string[];
    } catch {
      return [];
    }
  }

  function remember(code: string): void {
    if (!code || code === "auto") return;
    const next = [code, ...recents().filter((item) => item !== code)].slice(0, 5);
    localStorage.setItem(RECENTS_KEY, JSON.stringify(next));
  }

  function choose(code: string): void {
    remember(code);
    onChange(code);
    open = false;
    filter = "";
  }

  const selectedLabel = $derived(
    value === "auto"
      ? (autoLabel ?? $translator("quick.auto_detect"))
      : (languages.find((item) => item.code === value)?.name_en ?? value),
  );

  const recentLanguages = $derived(
    recents()
      .map((code) => languages.find((item) => item.code === code))
      .filter((item): item is Language => Boolean(item)),
  );

  const filtered = $derived(
    (filter.trim()
      ? languages.filter(
          (item) =>
            item.name_en.toLowerCase().includes(filter.toLowerCase()) ||
            item.name_native.toLowerCase().includes(filter.toLowerCase()) ||
            item.code.toLowerCase().includes(filter.toLowerCase()),
        )
      : languages
    ).slice(0, 80),
  );

  function onFilterKey(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      open = false;
      event.stopPropagation();
    }
    if (event.key === "Enter" && filtered.length > 0) {
      choose(filtered[0].code);
      event.stopPropagation();
    }
  }
</script>

<div class="picker">
  <button
    class="picker-button"
    type="button"
    title={label}
    aria-expanded={open}
    onclick={() => (open = !open)}
  >
    <span class="visually-hidden">{label}:</span>
    <span class="value" dir="auto">{selectedLabel}</span>
    <Icon name="chevron" />
  </button>

  {#if open}
    <div class="dropdown">
      <label class="search">
        <Icon name="search" size={12} />
        <input
          bind:value={filter}
          onkeydown={onFilterKey}
          placeholder={$translator("quick.search_language", { defaultValue: "Search languages" })}
          aria-label={$translator("quick.search_language", { defaultValue: "Search languages" })}
        />
      </label>
      <ul>
        {#if allowAuto}
          <li>
            <button
              type="button"
              class:selected={value === "auto"}
              onclick={() => choose("auto")}
            >
              <span dir="auto">{$translator("quick.auto_detect")}</span>
            </button>
          </li>
        {/if}
        {#if !filter.trim() && recentLanguages.length > 0}
          <li class="group">{$translator("quick.recent", { defaultValue: "Recent" })}</li>
          {#each recentLanguages as language (language.code)}
            <li>
              <button
                type="button"
                class:selected={value === language.code}
                onclick={() => choose(language.code)}
              >
                <span dir="auto">{language.name_native}</span>
                <span class="muted">{language.name_en}</span>
              </button>
            </li>
          {/each}
          <li class="group" role="presentation"></li>
        {/if}
        {#each filtered as language (language.code)}
          <li>
            <button
              type="button"
              class:selected={value === language.code}
              onclick={() => choose(language.code)}
            >
              <span dir="auto">{language.name_native}</span>
              <span class="muted">{language.name_en}</span>
            </button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>

<style>
  .picker {
    position: relative;
    display: inline-block;
  }

  .picker-button {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 6px;
    border-radius: var(--radius-small);
    font-size: 12px;
    color: var(--text-secondary);
  }

  .value {
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dropdown {
    position: absolute;
    z-index: 20;
    top: calc(100% + 4px);
    left: 0;
    width: 260px;
    max-height: 300px;
    display: flex;
    flex-direction: column;
    background: var(--surface-opaque);
    border: 1px solid var(--stroke);
    border-radius: var(--radius-small);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.22);
    padding: 6px;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 4px 6px;
    color: var(--text-tertiary);
  }

  .search input {
    flex: 1;
    border: 0;
    background: transparent;
    padding: 2px;
    font-size: 12px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow: auto;
  }

  li button {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    text-align: left;
    font-size: 12px;
    padding: 5px 6px;
  }

  li button.selected {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
  }

  li.group {
    padding: 6px 6px 2px;
    font-size: 10px;
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
