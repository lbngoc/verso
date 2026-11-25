<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Button from "$lib/components/Button.svelte";
  import Card from "$lib/components/Card.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import PermissionGuide from "$lib/components/PermissionGuide.svelte";
  import SettingRow from "$lib/components/SettingRow.svelte";
  import Switch from "$lib/components/Switch.svelte";
  import { setUiLanguage, t as translator } from "$lib/i18n";
  import { initAppearance } from "$lib/stores/appearance";
  import { applyTheme, initTheme } from "$lib/stores/theme";
  import {
    commands,
    events,
    type PermissionKind,
    type PermissionReason,
    type SettingsProfile,
    type Theme,
  } from "$lib/ipc";
  import { displayAccelerator } from "$lib/keys";

  const THEMES: Array<{ id: Theme; label: string }> = [
    { id: "system", label: "settings.theme_system" },
    { id: "light", label: "settings.theme_light" },
    { id: "dark", label: "settings.theme_dark" },
  ];

  const SERVICES = [
    { id: "http-google", name: "Google", receives: "onboarding.engine_google_receives" },
    { id: "http-bing", name: "Microsoft", receives: "onboarding.engine_bing_receives" },
  ];

  let step = $state(0);
  let order = $state(["http-google", "http-bing"]);
  let disabled = $state<string[]>([]);
  let settings = $state<SettingsProfile | null>(null);
  let permission = $state<"granted" | "denied" | "not_determined">("denied");
  let permissionReason = $state<PermissionReason>("missing");
  // The kind the permission step was opened for (spec 004): Screen Recording shows a short step of
  // its own that closes itself on grant instead of continuing the Setup flow.
  let stepKind = $state<PermissionKind>("accessibility");
  const screenOnly = $derived(step === 1 && stepKind === "screen_recording");

  async function closeWindow(): Promise<void> {
    try {
      await getCurrentWindow().close();
    } catch {
      /* browser preview */
    }
  }
  let isMac = $state(true);
  let warning = $state(false);
  // Pre-checked consent (FR-016): saved when onboarding finishes, changeable in Settings.
  let checkUpdates = $state(true);
  let startButton = $state<HTMLButtonElement | null>(null);
  let theme = $state<Theme>("system");

  // Applies at once and is saved with the rest of the profile; Settings › Appearance shows the same value.
  async function chooseTheme(next: Theme): Promise<void> {
    theme = next;
    applyTheme(next);
    await commands.updateSettings({ theme: next }).catch(() => undefined);
  }

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
      // Closed, not hidden: the last main window going away hides the Dock icon (FR-053).
      await getCurrentWindow().close();
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
      theme = settings.theme;
    }
    step = (await commands.onboardingStep().catch(() => 0)) === 1 ? 1 : 0;
    stepKind = await commands.permissionStepKind().catch(() => "accessibility" as const);
    if (isMac) {
      permissionReason = await commands.permissionReason(stepKind).catch(() => "missing" as const);
      permission = await commands.checkPermissions(stepKind).catch(() => "denied");
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
  {#if !screenOnly}
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
  {/if}

  {#if screenOnly}
    <section class="step-body">
      <h2>{$translator("permission.screen_recording_name")}</h2>
      <p class="muted">{$translator("permission.screen_recording_body")}</p>
      <PermissionGuide
        kind="screen_recording"
        reason={permissionReason}
        bind:status={permission}
        ongranted={() => void closeWindow()}
      />
    </section>
  {:else if step === 0}
    <section class="step-body">
      <h2>{$translator("onboarding.engines_title")}</h2>
      <p class="muted">{$translator("onboarding.engines_body")}</p>

      <Card>
        {#each order as id, index (id)}
          {@const service = SERVICES.find((item) => item.id === id)}
          {#if service}
            <SettingRow label={service.name} description={$translator(service.receives)}>
              <Button
                size="small"
                title={$translator("settings.move_up")}
                disabled={index === 0}
                onclick={() => move(id, -1)}
              >
                <span aria-label={$translator("settings.move_up")}>↑</span>
              </Button>
              <Button
                size="small"
                title={$translator("settings.move_down")}
                disabled={index === order.length - 1}
                onclick={() => move(id, 1)}
              >
                <span aria-label={$translator("settings.move_down")}>↓</span>
              </Button>
              <Switch checked={isOn(id)} label={service.name} onchange={() => toggle(id)} />
            </SettingRow>
          {/if}
        {/each}
      </Card>

      <Card>
        <SettingRow label={$translator("onboarding.appearance_title")}>
          <div class="segmented" role="radiogroup" aria-label={$translator("settings.theme")}>
            {#each THEMES as option (option.id)}
              <label class:on={theme === option.id}>
                <input
                  class="sr-only"
                  type="radio"
                  name="theme"
                  value={option.id}
                  checked={theme === option.id}
                  onchange={() => void chooseTheme(option.id)}
                />
                {$translator(option.label)}
              </label>
            {/each}
          </div>
        </SettingRow>
      </Card>

      {#if warning}
        <p class="warning" role="alert">{$translator("onboarding.at_least_one")}</p>
      {/if}

      <div class="actions go">
        <Button variant="primary" onclick={continueFromEngines}>
          <span>{$translator("onboarding.continue")}</span>
          <Icon name="arrow-right" size={14} />
        </Button>
      </div>
    </section>
  {:else}
    <section class="step-body">
      <h2>{$translator("onboarding.permission_title")}</h2>
      {#if isMac}
        {#if permissionReason !== "after_update"}
          <p class="muted">{$translator("onboarding.permission_body")}</p>
        {/if}
        <PermissionGuide kind="accessibility" reason={permissionReason} bind:status={permission} />
      {:else}
        <p class="muted">{$translator("onboarding.permission_windows")}</p>
      {/if}

      <Card>
        <SettingRow
          label={$translator("onboarding.updates_title")}
          description={$translator("onboarding.updates_body")}
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
          shortcut: displayAccelerator(settings?.shortcuts.translate ?? "Alt+Shift+Z", {
            mac: isMac,
          }),
        })}
      </p>

      <div class="actions go">
        <Button variant="primary" bind:element={startButton} onclick={finish}>
          <span>{$translator("onboarding.finish")}</span>
          <Icon name="arrow-right" size={14} />
        </Button>
      </div>
    </section>
  {/if}
</main>

<style>
  /* The window is a fixed size: the page fills it and pins the action to the bottom, so no step needs a
     scrollbar (FR-042). */
  .onboarding {
    box-sizing: border-box;
    height: 100vh;
    max-width: 480px;
    margin: 0 auto;
    padding: 20px 24px 22px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  h1 {
    margin: 0 0 4px;
    font-size: var(--type-page-title-size);
    font-weight: var(--type-page-title-weight);
  }

  h2 {
    margin: 0;
    font-size: var(--type-group-heading-size);
    font-weight: var(--type-group-heading-weight);
  }

  .step-body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
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
    font-size: var(--type-caption-size);
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
    font-size: var(--type-caption-size);
    line-height: 1;
  }

  .steps li.current {
    color: var(--accent-text);
    font-weight: 600;
  }

  .hint {
    font-size: var(--type-caption-size);
    color: var(--text-tertiary);
  }

  .warning {
    font-size: var(--type-row-description-size);
    color: var(--danger);
  }

  /* The step's action sits bottom right, like a macOS assistant (contracts/settings-look.md). */
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: auto;
  }

  /* The arrow slides right on hover and focus; reduced motion keeps it still. */
  .go :global(.icon) {
    transition: transform 160ms ease;
  }

  .go :global(button:hover .icon),
  .go :global(button:focus-visible .icon) {
    transform: translateX(4px);
  }

  @media (prefers-reduced-motion: reduce) {
    .go :global(.icon) {
      transition: none;
    }

    .go :global(button:hover .icon),
    .go :global(button:focus-visible .icon) {
      transform: none;
    }
  }

  .segmented {
    display: inline-flex;
    padding: 2px;
    border-radius: 8px;
    background: var(--hover);
  }

  .segmented label {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 10px;
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: var(--type-row-value-size);
    cursor: pointer;
  }

  .segmented label.on {
    background: var(--surface-opaque);
    color: var(--accent-text);
    font-weight: 600;
    box-shadow: inset 0 0 0 1px var(--stroke);
  }

  .segmented label:has(input:focus-visible) {
    box-shadow: var(--focus-ring);
  }
</style>
