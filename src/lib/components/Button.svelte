<script lang="ts">
  import type { Snippet } from "svelte";

  // The one button of Settings, Setup and About (spec 011 R4, contracts/settings-look.md): the
  // platform's secondary push button, the accent-filled primary, or a plain link. It never sets its
  // own alignment or width; where it sits is decided by its row or card footer.
  let {
    variant = "secondary",
    size = "regular",
    disabled = false,
    title,
    type = "button",
    onclick,
    children,
    element = $bindable(null),
  }: {
    variant?: "secondary" | "primary" | "link";
    size?: "regular" | "small";
    disabled?: boolean;
    title?: string;
    type?: "button" | "submit";
    onclick?: (event: MouseEvent) => void;
    children?: Snippet;
    /** The underlying element, for a page that moves focus to it. */
    element?: HTMLButtonElement | null;
  } = $props();
</script>

<button
  bind:this={element}
  class="button {variant}"
  class:small={size === "small"}
  {type}
  {disabled}
  {title}
  {onclick}
>
  {@render children?.()}
</button>

<style>
  .button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex: none;
    height: var(--button-height);
    padding: 0 var(--button-padding-x);
    border-radius: var(--button-radius);
    font-size: var(--type-row-label-size);
    font-weight: 400;
    line-height: 1;
    white-space: nowrap;
  }

  .small {
    height: var(--button-height-small);
  }

  .secondary {
    background: var(--button-fill);
    box-shadow: inset 0 0 0 0.5px var(--button-stroke);
    color: var(--text);
  }

  .secondary:hover:not(:disabled) {
    background: var(--button-fill-hover);
  }

  .primary {
    background: var(--accent);
    color: var(--accent-contrast);
  }

  .primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 88%, black);
  }

  .link {
    height: auto;
    padding: 0;
    background: transparent;
    color: var(--accent-text);
    font-size: inherit;
  }

  .link:hover:not(:disabled) {
    background: transparent;
    text-decoration: underline;
  }

  .button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .secondary:disabled {
    background: var(--button-fill);
    color: var(--text);
  }

  .primary:disabled {
    background: var(--accent);
    color: var(--accent-contrast);
  }

  /* macOS: the accent halo of a focused push button. */
  :global([data-os="macos"]) .button:focus-visible {
    box-shadow: var(--focus-ring);
  }

  /* Windows: Fluent standard button with the darker bottom edge and a 2 px accent focus outline. */
  :global([data-os="windows"]) .secondary {
    box-shadow:
      inset 0 0 0 1px var(--button-stroke),
      inset 0 -1px 0 var(--button-stroke-bottom, var(--button-stroke));
  }

  :global([data-os="windows"]) .button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
    box-shadow: none;
  }

  :global([data-high-contrast="true"]) .secondary {
    box-shadow: inset 0 0 0 1px var(--text);
  }
</style>
