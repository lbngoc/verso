import { invoke } from "@tauri-apps/api/core";

// Diagnosis of input methods that edit by sending Backspace and new keys (VietTelex "Phím thật",
// EVKey, OpenKey), spec 004 T106. Runs only when Verso was started with `--trace-input`. Logs the
// order and timing of key and input events with the field's length and caret — never the
// characters typed: every printable key is reported as "char".
export async function traceInput(field: HTMLTextAreaElement): Promise<() => void> {
  const enabled = await invoke<boolean>("input_trace_enabled").catch(() => false);
  if (!enabled) return () => undefined;
  let last = performance.now();
  const log = (kind: string, detail: string) => {
    const now = performance.now();
    const line = `${kind} ${detail} len=${field.value.length} caret=${field.selectionStart}-${field.selectionEnd} +${Math.round(now - last)}ms`;
    last = now;
    void invoke("trace_input", { line }).catch(() => undefined);
  };
  const keyName = (key: string) => (key.length === 1 ? "char" : key);
  const onKeydown = (event: KeyboardEvent) =>
    log("keydown", `${keyName(event.key)} composing=${event.isComposing} code=${event.keyCode}`);
  const onKeyup = (event: KeyboardEvent) => log("keyup", keyName(event.key));
  const onBeforeInput = (event: InputEvent) =>
    log(
      "beforeinput",
      `${event.inputType} data=${event.data === null ? "null" : event.data.length}`,
    );
  const onInput = (event: Event) => log("input", (event as InputEvent).inputType ?? "?");
  const onCompositionStart = () => log("compositionstart", "");
  const onCompositionEnd = () => log("compositionend", "");
  const onSelect = () => log("selectionchange", "");
  field.addEventListener("keydown", onKeydown, true);
  field.addEventListener("keyup", onKeyup, true);
  field.addEventListener("beforeinput", onBeforeInput, true);
  field.addEventListener("input", onInput, true);
  field.addEventListener("compositionstart", onCompositionStart, true);
  field.addEventListener("compositionend", onCompositionEnd, true);
  document.addEventListener("selectionchange", onSelect);
  log("trace", "on");
  return () => {
    field.removeEventListener("keydown", onKeydown, true);
    field.removeEventListener("keyup", onKeyup, true);
    field.removeEventListener("beforeinput", onBeforeInput, true);
    field.removeEventListener("input", onInput, true);
    field.removeEventListener("compositionstart", onCompositionStart, true);
    field.removeEventListener("compositionend", onCompositionEnd, true);
    document.removeEventListener("selectionchange", onSelect);
  };
}
