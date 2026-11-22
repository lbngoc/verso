import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPermissionPoll } from "./permissionPoll";

describe("createPermissionPoll (spec 010 R3)", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("does not poll before start", async () => {
    const check = vi.fn().mockResolvedValue("denied");
    createPermissionPoll(check, vi.fn());
    await vi.advanceTimersByTimeAsync(3000);
    expect(check).not.toHaveBeenCalled();
  });

  it("polls every interval after start and stops on stop", async () => {
    const check = vi.fn().mockResolvedValue("denied");
    const poll = createPermissionPoll(check, vi.fn(), 1000);
    poll.start();
    await vi.advanceTimersByTimeAsync(3000);
    expect(check).toHaveBeenCalledTimes(3);
    poll.stop();
    await vi.advanceTimersByTimeAsync(3000);
    expect(check).toHaveBeenCalledTimes(3);
  });

  it("calls onGranted once and stops when the grant becomes effective", async () => {
    const check = vi
      .fn()
      .mockResolvedValueOnce("denied")
      .mockResolvedValueOnce("granted")
      .mockResolvedValue("granted");
    const onGranted = vi.fn();
    const poll = createPermissionPoll(check, onGranted, 1000);
    poll.start();
    await vi.advanceTimersByTimeAsync(5000);
    expect(onGranted).toHaveBeenCalledTimes(1);
    expect(check).toHaveBeenCalledTimes(2);
    expect(poll.running).toBe(false);
  });

  it("re-checks at once on recheckNow", async () => {
    const check = vi.fn().mockResolvedValue("granted");
    const onGranted = vi.fn();
    const poll = createPermissionPoll(check, onGranted, 1000);
    poll.start();
    await poll.recheckNow();
    expect(check).toHaveBeenCalledTimes(1);
    expect(onGranted).toHaveBeenCalledTimes(1);
  });

  it("a second start does not double the polling", async () => {
    const check = vi.fn().mockResolvedValue("denied");
    const poll = createPermissionPoll(check, vi.fn(), 1000);
    poll.start();
    poll.start();
    await vi.advanceTimersByTimeAsync(2000);
    expect(check).toHaveBeenCalledTimes(2);
    poll.stop();
  });
});
