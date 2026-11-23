import { describe, expect, it } from "vitest";
import { createProgressDisplay } from "./updateProgress";

describe("update progress display (spec 004 FR-016)", () => {
  it("an unknown length is indeterminate until a percent arrives", () => {
    const display = createProgressDisplay();
    display.start();
    display.update(null);
    expect(display.state.indeterminate).toBe(true);
    display.update(40);
    expect(display.state.indeterminate).toBe(false);
    expect(display.state.percent).toBe(40);
  });

  it("a fast download is ready only after the minimum fill time, at 100 %", () => {
    let time = 1000;
    const display = createProgressDisplay({ minFillMs: 800, now: () => time });
    display.start();
    time += 100;
    display.finish();
    expect(display.state.percent).toBe(100);
    expect(display.state.readyToInstall).toBe(false);
    expect(display.remaining()).toBe(700);
    time += 700;
    expect(display.state.readyToInstall).toBe(true);
    expect(display.remaining()).toBe(0);
  });

  it("a slow download is ready as soon as it is done", () => {
    let time = 0;
    const display = createProgressDisplay({ minFillMs: 800, now: () => time });
    display.start();
    time += 5000;
    display.update(90);
    expect(display.state.readyToInstall).toBe(false);
    display.finish();
    expect(display.state.readyToInstall).toBe(true);
  });

  it("the percent never goes back or past 100", () => {
    const display = createProgressDisplay();
    display.start();
    display.update(60);
    display.update(30);
    expect(display.state.percent).toBe(60);
    display.update(150);
    expect(display.state.percent).toBe(100);
  });
});
