import { expect, test } from "vitest";
import {
  fracOf,
  frameCentre,
  frameOf,
  isZoomed,
  pan,
  panTo,
  ticks,
  timeAt,
  wholeView,
  withFrames,
  zoomAround,
  zoomLevel,
  zoomText,
} from "./timeline";

test("a position across the window maps to a time and back", () => {
  const view = { start: 100, end: 200 };

  expect(timeAt(view, 0.25)).toBe(125);
  expect(timeAt(view, 2)).toBe(200);
  expect(fracOf(view, 150)).toBe(0.5);
});

test("zooming keeps the time under the mouse in place", () => {
  const view = zoomAround(wholeView(100), 100, 0.5, 0.5);

  expect(view).toEqual({ start: 25, end: 75 });
  expect(zoomLevel(view, 100)).toBe(2);
  expect(isZoomed(view, 100)).toBe(true);
});

test("zooming in stops at 2 s across and zooming out at the whole video", () => {
  expect(zoomAround({ start: 0, end: 3 }, 100, 0, 0.1)).toEqual({ start: 0, end: 2 });
  expect(zoomAround({ start: 40, end: 60 }, 100, 0.5, 10)).toEqual({ start: 0, end: 100 });
  expect(isZoomed(wholeView(100), 100)).toBe(false);
});

test("a video shorter than 2 s never zooms in", () => {
  expect(zoomAround(wholeView(1), 1, 0.5, 0.1)).toEqual({ start: 0, end: 1 });
});

test("panning moves by a fraction of the window and stays inside the video", () => {
  const view = { start: 10, end: 30 };

  expect(pan(view, 100, 0.5)).toEqual({ start: 20, end: 40 });
  expect(pan(view, 100, -5)).toEqual({ start: 0, end: 20 });
  expect(pan(view, 100, 9)).toEqual({ start: 80, end: 100 });
  expect(panTo(view, 100, 50)).toEqual({ start: 40, end: 60 });
  expect(panTo(view, 100, 99)).toEqual({ start: 80, end: 100 });
});

test("the ruler spaces marks by zoom", () => {
  expect(ticks({ start: 0, end: 754 })).toEqual([0, 120, 240, 360, 480, 600, 720]);
  expect(ticks({ start: 0, end: 8 })).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8]);
  expect(ticks({ start: 3, end: 11 })).toEqual([3, 4, 5, 6, 7, 8, 9, 10, 11]);
  expect(ticks({ start: 0, end: 2 })).toEqual([0, 0.5, 1, 1.5, 2]);
});

test("a zoom reads as 1.5× below ten and whole above", () => {
  expect(zoomText(1.5)).toBe("1.5×");
  expect(zoomText(12.4)).toBe("12×");
});

test("time and frame convert for an fps", () => {
  expect(frameOf(1.0167, 60)).toBe(61);
  expect(frameCentre(61, 60)).toBeCloseTo(1.025, 4);
  expect(frameOf(frameCentre(61, 60), 60)).toBe(61);
});

test("time shows as minutes, seconds and the frame in the second", () => {
  expect(withFrames(151.3, 60)).toBe("2:31.18");
  expect(withFrames(0, 60)).toBe("0:00.00");
  expect(withFrames(59.99, 30)).toBe("1:00.00");
  expect(withFrames(1.001, 59.94)).toBe("0:01.00");
});
