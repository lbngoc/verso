<script lang="ts">
  import Button from "./Button.svelte";
  import Card from "./Card.svelte";
  import Markdown from "./Markdown.svelte";
  import { t } from "$lib/i18n";
  import type { UpdatePhase, UpdateStatus } from "$lib/ipc";
  import { createProgressDisplay } from "$lib/updateProgress";

  // The offer in Settings › About (spec 011 US2, contracts/update-in-settings.md § About layout):
  // what is new, then Skip This Version on the left and Install on Quit / Install Now on the right;
  // while busy, the buttons give way to the download bar or a line saying what happens next.
  let {
    status,
    windows = false,
    oninstall,
    oninstallonquit,
    onskip,
  }: {
    status: UpdateStatus;
    /** Windows: the installer closes and restarts Verso, so the wording differs. */
    windows?: boolean;
    oninstall: () => void;
    oninstallonquit: () => void;
    onskip: () => void;
  } = $props();

  const display = createProgressDisplay();
  let percent = $state(0);
  let indeterminate = $state(false);
  // After the download the bar fills to 100 % and stays long enough to be seen (spec 004 FR-016).
  let holdingBar = $state(false);
  let previous: UpdatePhase | null = null;
  let holdTimer: ReturnType<typeof setTimeout> | undefined;

  function sync(): void {
    percent = display.state.percent;
    indeterminate = display.state.indeterminate;
  }

  $effect(() => {
    const phase = status.phase;
    const progress = status.progress;
    if (phase === "downloading") {
      if (previous !== "downloading") display.start();
      if (progress) display.update(progress.percent);
      sync();
    } else if (
      previous === "downloading" &&
      (phase === "installing" || phase === "ready_on_quit")
    ) {
      display.finish();
      sync();
      holdingBar = true;
      clearTimeout(holdTimer);
      holdTimer = setTimeout(() => (holdingBar = false), display.remaining());
    } else {
      holdingBar = false;
    }
    previous = phase;
  });

  $effect(() => () => clearTimeout(holdTimer));

  const offer = $derived(status.offer);
  const showBar = $derived(status.phase === "downloading" || holdingBar);
  const failure = $derived(
    status.phase === "install_failed"
      ? status.error === "verify"
        ? $t("about.update_failed_verify")
        : status.error === "offline"
          ? $t("about.update_failed_offline")
          : $t("about.update_failed_download")
      : "",
  );
</script>

{#if offer}
  <Card>
    <div class="offer">
      <p class="title">
        {$t("about.update_title", { version: offer.version, current: offer.current })}
      </p>
      {#if offer.skipped}
        <p class="note">{$t("about.update_skipped_note")}</p>
      {/if}
      {#if offer.notes.trim()}
        <p class="notes-heading">{$t("about.update_whats_new")}</p>
        <div class="notes">
          <Markdown source={offer.notes} />
        </div>
      {/if}

      {#if showBar}
        <p class="state" role="status">
          {$t("about.update_downloading", { version: offer.version })}
          {#if !indeterminate}<span class="percent">{percent}%</span>{/if}
        </p>
        <div
          class="progress"
          class:indeterminate
          role="progressbar"
          aria-valuenow={indeterminate ? undefined : percent}
          aria-valuemin="0"
          aria-valuemax="100"
        >
          <span style:width={indeterminate ? undefined : `${percent}%`}></span>
        </div>
      {:else if status.phase === "installing"}
        <p class="state" role="status">
          {windows ? $t("about.update_will_restart") : $t("about.update_installing")}
        </p>
      {:else if status.phase === "ready_on_quit"}
        <p class="state" role="status">
          {$t("about.update_ready_on_quit", { version: offer.version })}
        </p>
      {:else if failure}
        <p class="state failure" role="alert">{failure}</p>
      {/if}
    </div>

    {#snippet footer()}
      {#if !showBar && status.phase === "ready_on_quit"}
        <Button variant="primary" onclick={oninstall}>{$t("about.update_install_now")}</Button>
      {:else if !showBar && status.phase === "install_failed"}
        <span class="lead"><Button onclick={onskip}>{$t("about.update_skip")}</Button></span>
        <Button variant="primary" onclick={oninstall}>{$t("about.update_retry")}</Button>
      {:else if !showBar && status.phase !== "installing"}
        <span class="lead"><Button onclick={onskip}>{$t("about.update_skip")}</Button></span>
        <!-- Platform order (contracts/settings-look.md): macOS puts the default button rightmost,
             Windows puts it first in the right-aligned group. -->
        {#if windows}
          <Button variant="primary" onclick={oninstall}>{$t("about.update_install_now")}</Button>
          <Button onclick={oninstallonquit}>{$t("about.update_install_on_quit")}</Button>
        {:else}
          <Button onclick={oninstallonquit}>{$t("about.update_install_on_quit")}</Button>
          <Button variant="primary" onclick={oninstall}>{$t("about.update_install_now")}</Button>
        {/if}
      {/if}
    {/snippet}
  </Card>
{/if}

<style>
  /* "div." outranks the Card's one-line layout for plain children (which centres them). */
  div.offer {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
    padding: 12px 0 4px;
  }

  .title {
    margin: 0;
    font-size: var(--type-row-label-size);
    font-weight: 600;
    color: var(--text);
  }

  .note,
  .notes-heading {
    margin: 0;
    font-size: var(--type-row-description-size);
    color: var(--text-secondary);
  }

  .notes-heading {
    margin-top: 4px;
    font-weight: 600;
  }

  /* The notes sit in the box itself, as in Software Update, and scroll inside it; the buttons below
     always stay visible. */
  .notes {
    max-height: 240px;
    overflow-y: auto;
    padding-bottom: 4px;
    border-bottom: 1px solid var(--separator);
    font-size: var(--type-row-label-size);
    user-select: text;
  }

  .state {
    display: flex;
    gap: 8px;
    margin: 6px 0 0;
    font-size: var(--type-row-description-size);
    color: var(--text-secondary);
  }

  .state.failure {
    color: var(--warning);
  }

  .percent {
    margin-left: auto;
  }

  .progress {
    height: 6px;
    border-radius: 3px;
    background: var(--separator);
    overflow: hidden;
    margin-bottom: 8px;
  }

  .progress span {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 200ms ease-out;
  }

  /* Unknown length: a segment sweeps along the bar; a static stripe with reduced motion. */
  .progress.indeterminate span {
    width: 35%;
    transition: none;
    animation: sweep 1.2s ease-in-out infinite;
  }

  @keyframes sweep {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(290%);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .progress.indeterminate span {
      width: 100%;
      animation: none;
      opacity: 0.5;
    }
  }

  /* Skip This Version sits apart, at the left end of the footer (macOS dialog order). */
  .lead {
    margin-right: auto;
  }
</style>
