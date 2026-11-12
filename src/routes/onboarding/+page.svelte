<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { setUiLanguage, t as translator } from "$lib/i18n";
  import { initAppearance } from "$lib/stores/appearance";
  import { initTheme } from "$lib/stores/theme";
  import { commands, events, type SettingsProfile } from "$lib/ipc";

  const SERVICES = [
    { id: "http-google", name: "Google", receives: "onboarding.engine_google_receives" },
    { id: "http-bing", name: "Microsoft", receives: "onboarding.engine_bing_receives" },
  ];

  let step = $state(0);
  let order = $state(["http-google", "http-bing"]);
  let disabled = $state<string[]>([]);
  let settings = $state<SettingsProfile | null>(null);
  let permission = $state<"granted" | "denied" | "not_determined">("denied");
  let isMac = $state(true);
  let warning = $state(false);
  // Pre-checked consent (FR-016): saved when onboarding finishes, changeable in Settings.
  let checkUpdates = $state(true);

  function isOn(id: string): boolean {
    return !disabled.includes(id);
  }

  function toggle(id: string): void {
    warning = false;
    disabled = isOn(id) ? [...disabled, id] : disabled.filter((item) => item !== id);
  }

  function move(id: string, direction: -1 | 1): void {
    const index = order.indexOf(id);
    const target = index + direction;
    if (index < 0 || target < 0 || target >= order.length) return;
    const next = [...order];
    [next[index], next[target]] = [next[target], next[index]];
    order = next;
  }

  async function continueFromEngines(): Promise<void> {
    if (disabled.length >= order.length) {
      warning = true;
      return;
    }
    step = 1;
  }

  async function finish(): Promise<void> {
    // The choice is saved before the step is marked complete, so no check can run before it.
    await commands.updateSettings({ check_updates: checkUpdates }).catch(() => undefined);
    await commands.completeEngineChoice(disabled, order).catch(() => undefined);
    try {
      await getCurrentWindow().hide();
    } catch {
      /* browser preview */
    }
  }

  onMount(async () => {
    const appearance = await initAppearance();
    isMac = appearance.os === "macos";
    await initTheme();
    settings = await commands.getSettings().catch(() => null);
    if (settings) {
      setUiLanguage(settings.ui_language);
      disabled = settings.engines_disabled;
      order = settings.engine_order;
    }
    step = (await commands.onboardingStep().catch(() => 0)) === 1 ? 1 : 0;
    if (isMac) {
      permission = await commands.checkPermissions("accessibility").catch(() => "denied");
      await events.onPermissionChanged((payload) => {
        if (payload.status === "granted" || payload.status === "denied") {
          permission = payload.status;
        }
      });
    }
  });
</script>

<main class="onboarding">
  <header>
    <h1>{$translator("onboarding.title")}</h1>
    <p class="muted">{$translator("onboarding.subtitle")}</p>
    <span class="step">
      {step === 0 ? $translator("onboarding.step_engines") : $translator("onboarding.step_permission")}
    </span>
  </header>

  {#if step === 0}
    <section class="card">
      <h2>{$translator("onboarding.engines_title")}</h2>
      <p class="muted">{$translator("onboarding.engines_body")}</p>

      {#each order as id, index (id)}
        {@const service = SERVICES.find((item) => item.id === id)}
        {#if service}
          <article class="engine" class:off={!isOn(id)}>
            <label class="row">
              <input type="checkbox" checked={isOn(id)} onchange={() => toggle(id)} />
              <strong>{service.name}</strong>
            </label>
            <p class="hint">{$translator(service.receives)}</p>
            <div class="reorder">
              <button
                type="button"
                aria-label={$translator("settings.move_up")}
                disabled={index === 0}
                onclick={() => move(id, -1)}
              >↑</button>
              <button
                type="button"
                aria-label={$translator("settings.move_down")}
                disabled={index === order.length - 1}
                onclick={() => move(id, 1)}
              >↓</button>
            </div>
          </article>
        {/if}
      {/each}

      {#if warning}
        <p class="warning" role="alert">{$translator("onboarding.at_least_one")}</p>
      {/if}

      <button class="primary" type="button" onclick={continueFromEngines}>
        {$translator("onboarding.continue")}
      </button>
    </section>
  {:else}
    <section class="card">
      <h2>{$translator("onboarding.permission_title")}</h2>
      {#if isMac}
        <p class="muted">{$translator("onboarding.permission_body")}</p>
        <p class="status" class:ok={permission === "granted"}>
          {permission === "granted"
            ? $translator("settings.permission_granted")
            : $translator("settings.permission_missing")}
        </p>
        <div class="row">
          <button
            class="primary"
            type="button"
            onclick={async () => {
              await commands.requestPermission("accessibility");
              permission = await commands.checkPermissions("accessibility");
            }}
          >
            {$translator("capture.grant")}
          </button>
          <button
            type="button"
            onclick={async () => (permission = await commands.checkPermissions("accessibility"))}
          >
            {$translator("capture.recheck")}
          </button>
        </div>
        <p class="hint">{$translator("capture.stale_hint")}</p>
      {:else}
        <p class="muted">{$translator("onboarding.permission_windows")}</p>
      {/if}

      <p class="hint">
        {$translator("onboarding.shortcut_hint", {
          shortcut: settings?.shortcuts.translate ?? "Alt+Shift+T",
        })}
      </p>

      <label class="row">
        <input type="checkbox" bind:checked={checkUpdates} />
        <span>{$translator("onboarding.updates_title")}</span>
      </label>
      <p class="hint">{$translator("onboarding.updates_body")}</p>

      <button class="primary" type="button" onclick={finish}>
        {$translator("onboarding.finish")}
      </button>
    </section>
  {/if}
</main>

<style>
  .onboarding {
    max-width: 480px;
    margin: 0 auto;
    padding: 28px 24px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  h1 {
    font-size: 22px;
    margin: 0 0 6px;
  }

  h2 {
    font-size: 15px;
    margin: 0 0 8px;
  }

  .step {
    font-size: 11px;
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .card {
    border: 1px solid var(--stroke);
    border-radius: var(--radius-medium);
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .engine {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 12px;
    border: 1px solid var(--stroke);
    border-radius: var(--radius-small);
    background: var(--surface-raised);
  }

  .engine.off {
    opacity: 0.65;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .reorder {
    display: flex;
    gap: 4px;
  }

  .reorder button {
    padding: 0 8px;
    font-size: 11px;
  }

  .hint {
    margin: 0;
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .status {
    margin: 0;
    font-size: 12px;
    color: var(--danger);
  }

  .status.ok {
    color: var(--success);
  }

  .warning {
    margin: 0;
    font-size: 12px;
    color: var(--danger);
  }

  .primary {
    align-self: flex-start;
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
    padding: 8px 16px;
  }
</style>
