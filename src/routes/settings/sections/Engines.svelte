<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import Engines from "../Engines.svelte";
  import { t } from "$lib/i18n";
  import type { EngineInfo } from "$lib/ipc";

  let {
    engines,
    disabled,
    onupdate,
  }: {
    engines: EngineInfo[];
    disabled: string[];
    onupdate: (patch: Record<string, unknown>) => Promise<void>;
  } = $props();
</script>

<Card title={$t("settings.card_engines")} helper={$t("settings.engines_hint")}>
  <div class="engines">
    <Engines
      {engines}
      {disabled}
      onchange={(order, off) =>
        void onupdate({ engine_order: order, engines_disabled: off })}
    />
  </div>
</Card>

<style>
  .engines {
    display: block;
    width: 100%;
    padding: 6px 0;
  }
</style>
