import { invoke } from "@tauri-apps/api/core";

// Spec 004 FR-028: each page reports its content scale and CSS size, so the log shows whether the
// webview matches the window and the display it is on (Revision 3, mixed-scale displays). No text.
export function reportViewport(): () => void {
  let timer: ReturnType<typeof setTimeout> | null = null;
  const send = () =>
    void invoke("report_viewport", {
      dpr: window.devicePixelRatio,
      width: window.innerWidth,
      height: window.innerHeight,
    }).catch(() => undefined);
  const schedule = () => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(send, 150);
  };
  window.addEventListener("resize", schedule);
  window.addEventListener("focus", schedule);
  send();
  return () => {
    window.removeEventListener("resize", schedule);
    window.removeEventListener("focus", schedule);
    if (timer) clearTimeout(timer);
  };
}
