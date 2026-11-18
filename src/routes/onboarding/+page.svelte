<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Card from "$lib/components/Card.svelte";
  import SettingRow from "$lib/components/SettingRow.svelte";
  import Switch from "$lib/components/Switch.svelte";
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
  let startButton = $state<HTMLButtonElement | null>(null);

  // Once Accessibility is granted the next action is the only thing left: focus it (FR-041).
  $effect(() => {
    if (step === 1 && permission === "granted") startButton?.focus();
  });

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
    // The window is created hidden and shown once this page has painted (R13).
    requestAnimationFrame(() => requestAnimationFrame(() => void events.emitPageReady()));
  });
</script>

<main class="onboarding">
  <header>
    <h1>{$translator("onboarding.title")}</h1>
    <p class="muted">{$translator("onboarding.subtitle")}</p>
    <ol class="steps" aria-label={$translator("onboarding.step_engines")}>
      <li class:current={step === 0} aria-current={step === 0 ? "step" : undefined}>
        <span class="num">1</span>
        {$translator("onboarding.indicator_services")}
      </li>
      <li class="sep" aria-hidden="true">·</li>
      <li class:current={step === 1} aria-current={step === 1 ? "step" : undefined}>
        <span class="num">2</span>
        {$translator("onboarding.indicator_permission")}
      </li>
    </ol>
  </header>

  {#if step === 0}
    <section class="step-body">
      <h2>{$translator("onboarding.engines_title")}</h2>
      <p class="muted">{$translator("onboarding.engines_body")}</p>

      <Card>
        {#each order as id, index (id)}
          {@const service = SERVICES.find((item) => item.id === id)}
          {#if service}
            <SettingRow label={service.name} helper={$translator(service.receives)}>
              <div class="reorder">
                <button
                  type="button"
                  aria-label={$translator("settings.move_up")}
                  disabled={index === 0}
                  onclick={() => move(id, -1)}>↑</button
                >
                <button
                  type="button"
                  aria-label={$translator("settings.move_down")}
                  disabled={index === order.length - 1}
                  onclick={() => move(id, 1)}>↓</button
                >
              </div>
              <Switch checked={isOn(id)} label={service.name} onchange={() => toggle(id)} />
            </SettingRow>
          {/if}
        {/each}
      </Card>

      {#if warning}
        <p class="warning" role="alert">{$translator("onboarding.at_least_one")}</p>
      {/if}

      <button class="primary" type="button" onclick={continueFromEngines}>
        {$translator("onboarding.continue")}
      </button>
    </section>
  {:else}
    <section class="step-body">
      <h2>{$translator("onboarding.permission_title")}</h2>
      {#if isMac}
        <p class="muted">{$translator("onboarding.permission_body")}</p>
        <Card>
          <SettingRow
            label={permission === "granted"
              ? $translator("settings.permission_granted")
              : $translator("settings.permission_missing")}
          >
            <button
              class="secondary"
              type="button"
              onclick={async () => {
                await commands.requestPermission("accessibility");
                permission = await commands.checkPermissions("accessibility");
              }}
            >
              {$translator("capture.grant")}
            </button>
            <button
              class="secondary"
              type="button"
              onclick={async () => (permission = await commands.checkPermissions("accessibility"))}
            >
              {$translator("capture.recheck")}
            </button>
          </SettingRow>
        </Card>
        <p class="status" class:ok={permission === "granted"}>
          {permission === "granted"
            ? $translator("settings.permission_granted")
            : $translator("capture.stale_hint")}
        </p>
      {:else}
        <p class="muted">{$translator("onboarding.permission_windows")}</p>
      {/if}

      <Card>
        <SettingRow
          label={$translator("onboarding.updates_title")}
          helper={$translator("onboarding.updates_body")}
        >
          <Switch
            checked={checkUpdates}
            label={$translator("onboarding.updates_title")}
            onchange={(checked) => (checkUpdates = checked)}
          />
        </SettingRow>
      </Card>

      <p class="hint">
        {$translator("onboarding.shortcut_hint", {
          shortcut: settings?.shortcuts.translate ?? "Alt+Shift+T",
        })}
      </p>

      <button class="primary wide" type="button" bind:this={startButton} onclick={finish}>
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
    margin: 0;
  }

  .step-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .step-body p {
    margin: 0;
  }

  .steps {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 10px 0 0;
    padding: 0;
    list-style: none;
    font-size: 12px;
    color: var(--text-tertiary);
  }

  .steps li {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .steps .num {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 1px solid currentColor;
    font-size: 11px;
    line-height: 1;
  }

  .steps li.current {
    color: var(--accent-text);
    font-weight: 600;
  }

  .reorder {
    display: flex;
    gap: 4px;
  }

  .reorder button {
    width: var(--control-height);
    height: var(--control-height);
    padding: 0;
    font-size: 12px;
  }

  .hint {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .status {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .status.ok {
    color: var(--success);
  }

  .warning {
    font-size: 12px;
    color: var(--danger);
  }

  .primary {
    align-self: flex-start;
    height: 36px;
    border: none;
    border-radius: 8px;
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
    padding: 0 18px;
  }

  .primary.wide {
    align-self: stretch;
    width: 100%;
  }

  .secondary {
    height: var(--control-height);
    padding: 0 12px;
    border: none;
    border-radius: 6px;
    background: var(--surface-opaque);
    box-shadow: inset 0 0 0 1px var(--stroke);
    color: var(--text);
    font-size: 12px;
  }
</style>
