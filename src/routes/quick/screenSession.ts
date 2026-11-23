import type { CaptureFailure, EngineError, QuickScreenEvent, TranslationResult } from "$lib/ipc";
import type { NoticeItem } from "$lib/notices";

// One capture's state in the quick panel (spec 004 Revision 2, data-model PanelScreenSession).
// Pure, so the reset rules (FR-021: a capture never shows the previous text, result or note, and
// a late answer from a previous capture is dropped) are tested without the page.

export interface SessionState {
  mode: "typed" | "screen";
  phase: "idle" | QuickScreenEvent["phase"];
  /** Bumped by every request and every capture; an answer carrying an older value is dropped. */
  request_seq: number;
  source_lang: string;
  target_lang: string;
  text: string;
  result: TranslationResult | null;
  error: EngineError | null;
  notices: NoticeItem[];
  detected: string | null;
  capture_note: CaptureFailure | null;
}

export type SessionEvent =
  | QuickScreenEvent
  | { phase: "request" }
  | { phase: "answer"; seq: number; result: TranslationResult }
  | { phase: "answer_error"; seq: number; error: EngineError };

export function initialSession(source_lang: string, target_lang: string): SessionState {
  return {
    mode: "typed",
    phase: "idle",
    request_seq: 0,
    source_lang,
    target_lang,
    text: "",
    result: null,
    error: null,
    notices: [],
    detected: null,
    capture_note: null,
  };
}

export function screenSession(state: SessionState, event: SessionEvent): SessionState {
  switch (event.phase) {
    case "reading":
      return {
        ...state,
        mode: "screen",
        phase: "reading",
        request_seq: state.request_seq + 1,
        source_lang: event.source_lang ?? state.source_lang,
        target_lang: event.target_lang ?? state.target_lang,
        text: "",
        result: null,
        error: null,
        notices: [],
        detected: null,
        capture_note: null,
      };
    case "text":
      return { ...state, phase: "text", text: event.text ?? "" };
    case "no_text":
    case "read_failed":
      return { ...state, phase: event.phase, text: "" };
    case "request":
      return { ...state, request_seq: state.request_seq + 1, error: null };
    case "answer":
      return event.seq === state.request_seq ? { ...state, result: event.result } : state;
    case "answer_error":
      return event.seq === state.request_seq
        ? { ...state, error: event.error, result: null }
        : state;
  }
}
