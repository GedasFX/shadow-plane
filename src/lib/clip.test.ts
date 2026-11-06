import { expect, test } from "vitest";
import {
  type Clip,
  clipInput,
  freshClip,
  freshMeta,
  keptLength,
  keptNext,
  MIN_PIECE,
  remove,
  reorder,
  select,
  setEnd,
  setStart,
  split,
} from "./clip";

const two: Clip = {
  pieces: [
    { start_s: 10, end_s: 20 },
    { start_s: 30, end_s: 40 },
  ],
  selected: 0,
};

test("a fresh video's clip is one piece, start to end", () => {
  expect(freshClip(754)).toEqual({ pieces: [{ start_s: 0, end_s: 754 }], selected: 0 });
});

test("the kept length adds the pieces", () => {
  expect(keptLength(two)).toBe(20);
});

test("an end can't pass its neighbour or the video's edges", () => {
  expect(setEnd(two, 0, 35, 100).pieces[0].end_s).toBe(30);
  expect(setEnd(two, 1, 500, 100).pieces[1].end_s).toBe(100);
  expect(setStart(two, 1, 15).pieces[1].start_s).toBe(20);
  expect(setStart(two, 0, -5).pieces[0].start_s).toBe(0);
});

test("a piece keeps a minimum length", () => {
  expect(setStart(two, 0, 25).pieces[0].start_s).toBe(20 - MIN_PIECE);
  expect(setEnd(two, 0, 5, 100).pieces[0].end_s).toBe(10 + MIN_PIECE);
});

test("bounds follow time when the list is reordered", () => {
  const swapped = reorder(two, 0, 1);

  expect(setEnd(swapped, 1, 35, 100).pieces[1].end_s).toBe(30);
  expect(setStart(swapped, 0, 5).pieces[0].start_s).toBe(20);
});

test("splitting halves a piece with a gap and selects the second half", () => {
  const halves = split(freshClip(100), 0);

  expect(halves.pieces).toEqual([
    { start_s: 0, end_s: 49.5 },
    { start_s: 50.5, end_s: 100 },
  ]);
  expect(halves.selected).toBe(1);
  expect(split(freshClip(2), 0).pieces[0].end_s).toBeCloseTo(0.9);
});

test("removing needs two pieces and keeps the selection on its piece", () => {
  expect(remove(freshClip(10), 0)).toEqual(freshClip(10));
  expect(remove({ ...two, selected: 1 }, 0)).toEqual({ pieces: [two.pieces[1]], selected: 0 });
  expect(remove({ ...two, selected: 1 }, 1).selected).toBe(0);
});

test("reordering moves a piece and the selection follows it", () => {
  const moved = reorder({ ...two, selected: 0 }, 0, 1);

  expect(moved.pieces).toEqual([two.pieces[1], two.pieces[0]]);
  expect(moved.selected).toBe(1);
});

test("selecting a piece changes only the selection", () => {
  expect(select(two, 1)).toEqual({ ...two, selected: 1 });
});

test("the kept part carries on, then moves to the next piece, then stops", () => {
  expect(keptNext(two, 0, 15, 50)).toBe("carry on");
  expect(keptNext(two, 0, 19.99, 50)).toBe(1);
  expect(keptNext(two, 1, 39.99, 50)).toBe("stop");
});

test("a fresh clip has no title, no labels and the video's game", () => {
  expect(freshMeta("PEAK")).toEqual({ title: "", game: "PEAK", great: false, best: false });
  expect(clipInput(freshMeta("PEAK"), two)).toEqual({ ...freshMeta("PEAK"), pieces: two.pieces });
});
