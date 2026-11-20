<script lang="ts">
  import Icon from "./Icon.svelte";

  // A status line that cannot be missed: an icon and a tinted pill. A missing permission is a warning
  // (amber), not an error, so it never takes the error colour.
  let { kind, text }: { kind: "ok" | "warn"; text: string } = $props();
</script>

<p class="badge" class:ok={kind === "ok"} class:warn={kind === "warn"} role="status">
  <Icon name={kind === "ok" ? "check" : "warning"} size={14} />
  <span>{text}</span>
</p>

<style>
  .badge {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 8px 12px;
    border-radius: 8px;
    font-size: 12px;
    font-weight: 600;
    line-height: 1.3;
  }

  .ok {
    color: var(--success);
    background: color-mix(in srgb, var(--success) 14%, transparent);
  }

  .warn {
    color: var(--warning);
    background: color-mix(in srgb, var(--warning) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--warning) 45%, transparent);
  }
</style>
