import { describe, expect, it } from "vitest";
import type { EngineError, TranslationResult } from "$lib/ipc";
import { initialSession, screenSession, type SessionState } from "./screenSession";

// Spec 004 Revision 2 (T070, data-model PanelScreenSession): one capture's state in the panel.

const result = { text: "xin chào", engine_id: "google" } as unknown as TranslationResult;
const failure = { kind: "network" } as unknown as EngineError;

function typed(): SessionState {
  return {
    ...initialSession("auto", "vi"),
    text: "old typed text",
    result,
    error: failure,
    detected: "en",
    capture_note: "nothing_selected",
    notices: [{ key: "notice.truncated" }],
    request_seq: 4,
  };
}

describe("screenSession", () => {
  it("a reading event resets the whole page state and starts a new request sequence", () => {
    const next = screenSession(typed(), {
      phase: "reading",
      source_lang: "auto",
      target_lang: "ja",
    });
    expect(next.mode).toBe("screen");
    expect(next.phase).toBe("reading");
    expect(next.text).toBe("");
    expect(next.result).toBeNull();
    expect(next.error).toBeNull();
    expect(next.notices).toEqual([]);
    expect(next.detected).toBeNull();
    expect(next.capture_note).toBeNull();
    expect(next.request_seq).toBe(5);
    expect(next.source_lang).toBe("auto");
    expect(next.target_lang).toBe("ja");
  });

  it("a text event sets the source and the phase", () => {
    const reading = screenSession(typed(), {
      phase: "reading",
      source_lang: "auto",
      target_lang: "vi",
    });
    const next = screenSession(reading, { phase: "text", text: "hello" });
    expect(next.phase).toBe("text");
    expect(next.text).toBe("hello");
    expect(next.mode).toBe("screen");
  });

  it("no_text and read_failed keep the source empty", () => {
    const reading = screenSession(typed(), {
      phase: "reading",
      source_lang: "auto",
      target_lang: "vi",
    });
    for (const phase of ["no_text", "read_failed"] as const) {
      const next = screenSession(reading, { phase });
      expect(next.phase).toBe(phase);
      expect(next.text).toBe("");
    }
  });

  it("an answer from a previous capture is ignored", () => {
    const first = screenSession(typed(), {
      phase: "reading",
      source_lang: "auto",
      target_lang: "vi",
    });
    const seq = first.request_seq;
    const second = screenSession(first, {
      phase: "reading",
      source_lang: "auto",
      target_lang: "vi",
    });
    const late = screenSession(second, { phase: "answer", seq, result });
    expect(late.result).toBeNull();
    const fresh = screenSession(second, { phase: "answer", seq: second.request_seq, result });
    expect(fresh.result).toBe(result);
    const lateError = screenSession(second, { phase: "answer_error", seq, error: failure });
    expect(lateError.error).toBeNull();
  });

  it("a request bumps the sequence and clears the error", () => {
    const next = screenSession(typed(), { phase: "request" });
    expect(next.request_seq).toBe(5);
    expect(next.error).toBeNull();
  });

  it("a reset returns to a clean typed session and drops answers still in flight", () => {
    const next = screenSession(typed(), { phase: "reset" });
    expect(next.mode).toBe("typed");
    expect(next.phase).toBe("idle");
    expect(next.text).toBe("");
    expect(next.result).toBeNull();
    expect(next.capture_note).toBeNull();
    expect(next.request_seq).toBe(5);
  });
});
