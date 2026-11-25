/** One text-size scale for the popup, the quick panel and Settings (spec 009 R23). */
export const STEPS = [80, 90, 100, 110, 125, 140, 150] as const;
export const MIN_STEP = -2;
export const MAX_STEP = 4;

export function clampStep(step: number): number {
  return Math.min(MAX_STEP, Math.max(MIN_STEP, Math.round(step)));
}

export function percentFor(step: number): number {
  return STEPS[clampStep(step) - MIN_STEP];
}

export function fontPx(step: number, mac: boolean): number {
  const base = mac ? 17 : 16;
  return Math.round((base * percentFor(step)) / 100);
}

/**
 * The panel's text (source field and result alike, spec 004 FR-031): the system body size at the
 * default step — 13 px on macOS, 14 px (Segoe UI) on Windows — scaled by the same step.
 */
export function panelFontPx(step: number, mac: boolean): number {
  const base = mac ? 13 : 14;
  return Math.round((base * percentFor(step)) / 100);
}

export function label(step: number): string {
  return `${percentFor(step)} %`;
}

export function canShrink(step: number): boolean {
  return clampStep(step) > MIN_STEP;
}

export function canEnlarge(step: number): boolean {
  return clampStep(step) < MAX_STEP;
}
