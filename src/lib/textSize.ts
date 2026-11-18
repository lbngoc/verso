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

export function label(step: number): string {
  return `${percentFor(step)} %`;
}

export function canShrink(step: number): boolean {
  return clampStep(step) > MIN_STEP;
}

export function canEnlarge(step: number): boolean {
  return clampStep(step) < MAX_STEP;
}
