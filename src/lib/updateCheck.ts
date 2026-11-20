// "Check for Updates…" in Settings → About (spec 009 FR-050): the check runs in place; the update
// window opens only when a newer release is offered.

export type CheckState = "idle" | "checking" | "up_to_date" | "failed";

export interface CheckDeps<Offer> {
  check: () => Promise<Offer | null>;
  openOffer: () => Promise<void>;
}

/** Runs one check, reporting each state; resolves with the final one. */
export async function runUpdateCheck<Offer>(
  deps: CheckDeps<Offer>,
  report: (state: CheckState) => void,
): Promise<CheckState> {
  report("checking");
  let offer: Offer | null;
  try {
    offer = await deps.check();
  } catch {
    report("failed");
    return "failed";
  }
  if (!offer) {
    report("up_to_date");
    return "up_to_date";
  }
  // The window shows the notes and the choices; the section goes back to rest.
  report("idle");
  await deps.openOffer().catch(() => undefined);
  return "idle";
}
