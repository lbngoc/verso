import { describe, expect, it, vi } from "vitest";
import { runUpdateCheck, type CheckState } from "./updateCheck";

async function run(check: () => Promise<{ version: string } | null>) {
  const states: CheckState[] = [];
  const openOffer = vi.fn(async () => undefined);
  const final = await runUpdateCheck({ check, openOffer }, (state) => states.push(state));
  return { states, final, openOffer };
}

describe("runUpdateCheck (FR-050)", () => {
  it("up to date: a line in the section, no window", async () => {
    const { states, final, openOffer } = await run(async () => null);
    expect(states).toEqual(["checking", "up_to_date"]);
    expect(final).toBe("up_to_date");
    expect(openOffer).not.toHaveBeenCalled();
  });

  it("offer: opens the update window and returns to rest", async () => {
    const { states, final, openOffer } = await run(async () => ({ version: "9.9.9" }));
    expect(states).toEqual(["checking", "idle"]);
    expect(final).toBe("idle");
    expect(openOffer).toHaveBeenCalledOnce();
  });

  it("failure: a warning line, no window", async () => {
    const { states, final, openOffer } = await run(async () => {
      throw new Error("offline");
    });
    expect(states).toEqual(["checking", "failed"]);
    expect(final).toBe("failed");
    expect(openOffer).not.toHaveBeenCalled();
  });

  it("a window that fails to open does not break the section", async () => {
    const states: CheckState[] = [];
    const final = await runUpdateCheck(
      {
        check: async () => ({ version: "9.9.9" }),
        openOffer: async () => Promise.reject(new Error("x")),
      },
      (state) => states.push(state),
    );
    expect(final).toBe("idle");
  });
});
