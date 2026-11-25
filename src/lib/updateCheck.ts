// "Check for Updates…" in Settings › About (spec 011 US2): the check runs in place and its result,
// an offer with its notes included, is shown in About. No window opens.

import type { UpdateStatus } from "./ipc";

/** Runs one manual check, reporting `checking` and then the status; resolves with the final one. */
export async function runUpdateCheck(
  check: () => Promise<UpdateStatus>,
  report: (status: UpdateStatus) => void,
  current: UpdateStatus,
): Promise<UpdateStatus> {
  report({ ...current, phase: "checking", error: null });
  let next: UpdateStatus;
  try {
    next = await check();
  } catch {
    next = { ...current, phase: "check_failed", error: "offline" };
  }
  report(next);
  return next;
}
