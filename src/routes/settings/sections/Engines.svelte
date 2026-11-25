<script lang="ts">
  import Button from "$lib/components/Button.svelte";
  import Card from "$lib/components/Card.svelte";
  import SettingRow from "$lib/components/SettingRow.svelte";
  import Switch from "$lib/components/Switch.svelte";
  import { t } from "$lib/i18n";
  import type { EngineInfo } from "$lib/ipc";

  // Engine list: one row per engine with its order buttons and switch on the right (spec 011).

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

  function status(engine: EngineInfo): string {
    if (!isOn(engine.id)) return $t("settings.engine_disabled");
    if (engine.cooldown_remaining_s > 0) {
      return $t("settings.engine_cooldown", { seconds: engine.cooldown_remaining_s });
    }
    if (engine.availability.state === "available") return $t("settings.engine_available");
    return engine.availability.reason ?? engine.availability.state;
  }

  function serviceProvider(id: string): string {
    return id === "http-bing" ? "Microsoft" : "Google";
  }
</script>

<Card title={$t("settings.card_engines")} helper={$t("settings.engines_hint")}>
  {#each order as id, index (id)}
    {@const engine = byId.get(id)}
    {#if engine}
      <SettingRow
        label={serviceName(id)}
        description={`${$t("settings.engine_receives", { service: serviceProvider(id) })} · ${status(engine)}`}
      >
        <Button
          size="small"
          title={$t("settings.move_up")}
          disabled={index === 0}
          onclick={() => move(id, -1)}
        >
          <span aria-label={$t("settings.move_up")}>↑</span>
        </Button>
        <Button
          size="small"
          title={$t("settings.move_down")}
          disabled={index === order.length - 1}
          onclick={() => move(id, 1)}
        >
          <span aria-label={$t("settings.move_down")}>↓</span>
        </Button>
        <Switch checked={isOn(id)} label={serviceName(id)} onchange={() => toggle(id)} />
      </SettingRow>
    {/if}
  {/each}
</Card>
