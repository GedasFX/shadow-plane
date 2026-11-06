export type View = { start: number; end: number };

export const MIN_SPAN = 2;

const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));

export const wholeView = (length: number): View => ({ start: 0, end: length });

// A position across the visible window (0 to 1) as a time, and back.
export const timeAt = (view: View, frac: number) => view.start + clamp(frac, 0, 1) * (view.end - view.start);
export const fracOf = (view: View, t: number) => (t - view.start) / (view.end - view.start);

// Zoom by `factor` (below 1 zooms in), keeping the time under `frac` where it is.
export function zoomAround(view: View, length: number, frac: number, factor: number): View {
  const span = view.end - view.start;
  const next = clamp(span * factor, Math.min(MIN_SPAN, length), length);
  const start = clamp(view.start + frac * span - frac * next, 0, length - next);

  return { start, end: start + next };
}

// Move the window by a fraction of its width.
export function pan(view: View, length: number, byFrac: number): View {
  const span = view.end - view.start;
  const start = clamp(view.start + byFrac * span, 0, length - span);

  return { start, end: start + span };
}

// Centre the window on a time, as dragging the overview strip does.
export function panTo(view: View, length: number, centre: number): View {
  const span = view.end - view.start;
  const start = clamp(centre - span / 2, 0, length - span);

  return { start, end: start + span };
}

export const zoomLevel = (view: View, length: number) => length / (view.end - view.start);

export const isZoomed = (view: View, length: number) => zoomLevel(view, length) > 1.01;

export const zoomText = (level: number) => `${level.toFixed(level < 10 ? 1 : 0)}×`;

const STEPS = [0.5, 1, 2, 5, 10, 15, 30, 60, 120, 180, 300];

// The gap between ruler marks: the smallest that leaves about eight across the window.
export const tickStep = (view: View) => STEPS.find((s) => (view.end - view.start) / s <= 8) ?? 300;

export function ticks(view: View): number[] {
  const step = tickStep(view);
  const marks: number[] = [];
  for (let t = Math.ceil(view.start / step) * step; t <= view.end; t += step) marks.push(t);

  return marks;
}

export const frameOf = (t: number, fps: number) => Math.round(t * fps);

// The middle of a frame: a seek there lands on it however the timestamps jitter.
export const frameCentre = (frame: number, fps: number) => (frame + 0.5) / fps;

// "2:31.18": minutes, seconds and the frame within the second.
export function withFrames(t: number, fps: number) {
  const frame = frameOf(t, fps);
  const s = Math.floor(frame / fps);
  const ff = frame - Math.round(s * fps);

  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}.${String(ff).padStart(2, "0")}`;
}
