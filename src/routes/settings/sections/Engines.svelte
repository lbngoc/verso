<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import { t } from "$lib/i18n";
  import type { EngineInfo } from "$lib/ipc";

  // Engine list: order and on/off inside the "Engines" card (moved from the 0.1.x page).

  let {
    engines,
    disabled = [],
    onupdate,
  }: {
    engines: EngineInfo[];
    disabled?: string[];
    onupdate: (patch: Record<string, unknown>) => Promise<void>;
  } = $props();

  let order = $state<string[]>([]);

  $effect(() => {
    const next = [...engines].sort((a, b) => a.priority - b.priority).map((engine) => engine.id);
    if (next.join(",") !== order.join(",")) order = next;
  });

  const byId = $derived(new Map(engines.map((engine) => [engine.id, engine])));

  function isOn(id: string): boolean {
    return !disabled.includes(id);
  }

  function toggle(id: string): void {
    const next = isOn(id) ? [...disabled, id] : disabled.filter((item) => item !== id);
    if (next.length >= order.length) return; // at least one engine stays on (FR-016)
    void onupdate({ engine_order: order, engines_disabled: next });
  }

  function move(id: string, direction: -1 | 1): void {
    const index = order.indexOf(id);
    const target = index + direction;
    if (index < 0 || target < 0 || target >= order.length) return;
    const next = [...order];
    [next[index], next[target]] = [next[target], next[index]];
    void onupdate({ engine_order: next, engines_disabled: disabled });
  }

  function serviceName(id: string): string {
    switch (id) {
      case "http-google":
        return $t("settings.engine_google");
      case "http-bing":
        return $t("settings.engine_bing");
      default:
        return id;
    }
  }

  function serviceProvider(id: string): string {
    return id === "http-bing" ? "Microsoft" : "Google";
  }
</script>

<Card title={$t("settings.card_engines")} helper={$t("settings.engines_hint")}>
  <div class="engines">
    {#each order as id, index (id)}
      {@const engine = byId.get(id)}
      {#if engine}
        <article class="engine" class:off={!isOn(id)}>
          <label class="switch">
            <input
              type="checkbox"
              checked={isOn(id)}
              onchange={() => toggle(id)}
              aria-label={serviceName(id)}
            />
            <span class="track" aria-hidden="true"></span>
          </label>
          <div class="meta">
            <strong>{serviceName(id)}</strong>
            <span class="receives">
              {$t("settings.engine_receives", { service: serviceProvider(id) })}
            </span>
            <span class="status">
              {#if !isOn(id)}
                {$t("settings.engine_disabled")}
              {:else if engine.cooldown_remaining_s > 0}
                {$t("settings.engine_cooldown", { seconds: engine.cooldown_remaining_s })}
              {:else if engine.availability.state === "available"}
                {$t("settings.engine_available")}
              {:else}
                {engine.availability.reason ?? engine.availability.state}
              {/if}
            </span>
          </div>
          <div class="reorder">
            <button
              type="button"
              title={$t("settings.move_up")}
              aria-label={$t("settings.move_up")}
              disabled={index === 0}
              onclick={() => move(id, -1)}
            >
              ↑
            </button>
            <button
              type="button"
              title={$t("settings.move_down")}
              aria-label={$t("settings.move_down")}
              disabled={index === order.length - 1}
              onclick={() => move(id, 1)}
            >
              ↓
            </button>
          </div>
        </article>
      {/if}
    {/each}
  </div>
</Card>

<style>
  .engines {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .engine {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--stroke);
    border-radius: var(--radius-small);
    background: var(--surface-raised);
  }

  .engine.off {
    opacity: 0.65;
  }

  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  .receives {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .status {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .reorder {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .reorder button {
    padding: 0 6px;
    font-size: 11px;
    line-height: 1.4;
  }

  .switch {
    position: relative;
    display: inline-flex;
    width: 32px;
    height: 18px;
    flex: none;
    margin-top: 2px;
  }

  .switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .track {
    position: absolute;
    inset: 0;
    border-radius: 9px;
    background: var(--separator);
    transition: background 0.15s ease;
  }

  .track::after {
    content: "";
    position: absolute;
    width: 14px;
    height: 14px;
    top: 2px;
    left: 2px;
    border-radius: 50%;
    background: var(--accent-contrast);
    box-shadow: var(--shadow-control);
    transition: transform 0.15s ease;
  }

  .switch input:checked + .track {
    background: var(--accent);
  }

  .switch input:checked + .track::after {
    transform: translateX(14px);
  }

  .engines {
    display: block;
    width: 100%;
  }
</style>
