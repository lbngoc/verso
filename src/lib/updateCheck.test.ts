import { describe, expect, it } from "vitest";
import type { UpdateStatus } from "./ipc";
import { runUpdateCheck } from "./updateCheck";

const idle: UpdateStatus = {
  phase: "idle",
  offer: null,
  last_checked: "2026-11-25T08:00:00Z",
  progress: null,
  error: null,
};

async function run(check: () => Promise<UpdateStatus>) {
  const reported: UpdateStatus[] = [];
  const final = await runUpdateCheck(check, (status) => reported.push(status), idle);
  return { phases: reported.map((status) => status.phase), final };
}

describe("runUpdateCheck (spec 011 US2)", () => {
  it("up to date: checking, then the status the backend returned", async () => {
    const { phases, final } = await run(async () => ({
      ...idle,
      phase: "up_to_date",
      last_checked: "2026-11-25T12:00:00Z",
    }));
    expect(phases).toEqual(["checking", "up_to_date"]);
    expect(final.last_checked).toBe("2026-11-25T12:00:00Z");
  });

  it("offer: the status carries the offer and its notes, shown in place", async () => {
    const offer = { version: "9.9.9", current: "0.1.11", notes: "- Faster", skipped: false };
    const { phases, final } = await run(async () => ({ ...idle, phase: "available", offer }));
    expect(phases).toEqual(["checking", "available"]);
    expect(final.offer?.notes).toBe("- Faster");
  });

  it("a failed call reports check_failed and keeps the last successful time", async () => {
    const { phases, final } = await run(async () => {
      throw new Error("ipc");
    });
    expect(phases).toEqual(["checking", "check_failed"]);
    expect(final.error).toBe("offline");
    expect(final.last_checked).toBe(idle.last_checked);
  });

  it("takes no window opener: the check never opens a window", () => {
    expect(runUpdateCheck.length).toBe(3);
  });
});
