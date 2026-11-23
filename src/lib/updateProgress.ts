/**
 * What the update window's bar shows (spec 004 FR-016). The bar must reach 100 % before the
 * "installing" wording appears, and a download that finishes in a blink still shows the bar filling
 * for `minFillMs`, so it never looks like it jumped.
 */
export interface ProgressDisplay {
  start(): void;
  /** `null` = the length is unknown: the bar is indeterminate. */
  update(percent: number | null): void;
  /** The download is done: the bar goes to 100 %. */
  finish(): void;
  readonly state: { percent: number; indeterminate: boolean; readyToInstall: boolean };
  /** Milliseconds until `readyToInstall` can become true (0 when it is). */
  remaining(): number;
}

export function createProgressDisplay(
  options: { minFillMs?: number; now?: () => number } = {},
): ProgressDisplay {
  const minFillMs = options.minFillMs ?? 800;
  const now = options.now ?? (() => Date.now());
  let startedAt = now();
  let percent = 0;
  let indeterminate = false;
  let done = false;

  return {
    start() {
      startedAt = now();
      percent = 0;
      indeterminate = false;
      done = false;
    },
    update(next) {
      if (done) return;
      if (next === null) {
        indeterminate = true;
        return;
      }
      indeterminate = false;
      // Never backwards, never past 100.
      percent = Math.max(percent, Math.min(100, Math.round(next)));
    },
    finish() {
      done = true;
      indeterminate = false;
      percent = 100;
    },
    get state() {
      return {
        percent,
        indeterminate: indeterminate && !done,
        readyToInstall: done && percent === 100 && now() - startedAt >= minFillMs,
      };
    },
    remaining() {
      return done ? Math.max(0, minFillMs - (now() - startedAt)) : minFillMs;
    },
  };
}
