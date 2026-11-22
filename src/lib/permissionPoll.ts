import type { PermissionStatus } from "./ipc";

/**
 * Re-checks a permission while the permission step shows it missing (spec 010 R3, FR-006): the
 * step notices the grant without a restart and never claims success before the check says so.
 */
export interface PermissionPoll {
  start(): void;
  stop(): void;
  /** Checks at once (window focus); resolves after the check. */
  recheckNow(): Promise<void>;
  readonly running: boolean;
}

export function createPermissionPoll(
  check: () => Promise<PermissionStatus>,
  onGranted: () => void,
  intervalMs = 1000,
): PermissionPoll {
  let timer: ReturnType<typeof setInterval> | undefined;
  let done = false;

  const stop = () => {
    if (timer !== undefined) clearInterval(timer);
    timer = undefined;
  };

  const tick = async () => {
    if (done) return;
    const status = await check().catch(() => "denied" as const);
    if (status === "granted" && !done) {
      done = true;
      stop();
      onGranted();
    }
  };

  return {
    start() {
      if (timer !== undefined || done) return;
      timer = setInterval(() => void tick(), intervalMs);
    },
    stop,
    recheckNow: tick,
    get running() {
      return timer !== undefined;
    },
  };
}
