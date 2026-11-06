import { expect, test } from "vitest";
import type { Job, JobProgress, Queue } from "./api";
import { chipText, failures, newestFailure, nowText, percent, phase, timeLeft, waiting } from "./jobs";

const job = (id: number, status: Job["status"] = "pending"): Job => ({
  id,
  status,
  key: "PEAK/a.mp4",
  title: `clip ${id}`,
  game: "PEAK",
  file_name: `clip ${id}.mp4`,
  kept_s: 60,
  output: null,
  error: null,
});

const queue = (q: Partial<Queue> = {}): Queue => ({ running: null, pending: [], done: [], paused: false, ...q });
const progress = (p: Partial<JobProgress> = {}): JobProgress => ({
  id: 1,
  phase: "encode",
  done_s: 25,
  kept_s: 60,
  left_s: 72,
  ...p,
});
const running = queue({ running: job(1, "running"), pending: [job(2), job(3)] });

test("the phase follows what runs, and paused wins", () => {
  expect(phase(running, progress())).toBe("encode");
  expect(phase(running, progress({ phase: "check" }))).toBe("check");
  expect(phase(running, null)).toBe("encode");
  expect(phase({ ...running, paused: true }, progress())).toBe("paused");
  expect(phase(queue({ done: [job(4, "done")] }), null)).toBe("saved");
  expect(phase(queue({ done: [job(4, "failed")] }), null)).toBe("idle");
  expect(phase(queue(), null)).toBe("idle");
});

test("progress from another job is ignored", () => {
  expect(phase(running, progress({ id: 9, phase: "check" }))).toBe("encode");
  expect(percent(running, progress({ id: 9 }))).toBe(0);
  expect(timeLeft(running, progress({ id: 9 }))).toBeNull();
});

test("the percentage is a whole number, none when nothing runs", () => {
  expect(percent(running, progress())).toBe(42);
  expect(percent(running, progress({ done_s: 99 }))).toBe(100);
  expect(percent(queue({ pending: [job(2)] }), progress())).toBeNull();
});

test("the time left is m:ss, h:mm:ss past an hour, none without a rate", () => {
  expect(timeLeft(running, progress())).toBe("1:12");
  expect(timeLeft(running, progress({ left_s: 3725 }))).toBe("1:02:05");
  expect(timeLeft(running, progress({ left_s: null }))).toBeNull();
});

test("the waiting count is the pending jobs", () => {
  expect(waiting(running)).toBe(2);
});

test("failures count the failed jobs newer than the last one shown", () => {
  const done = [job(8, "failed"), job(7, "done"), job(5, "failed")];

  expect(failures(done, 0)).toBe(2);
  expect(failures(done, 5)).toBe(1);
  expect(failures(done, newestFailure(done))).toBe(0);
  expect(newestFailure([job(7, "done")])).toBe(0);
});

test("the chip says the same in words", () => {
  expect(chipText(running, progress(), 1)).toEqual({
    title: "Encoding · 1:12 left",
    label: "Jobs: encoding, 42%, 1:12 left, 2 waiting, 1 failed",
  });
  expect(chipText({ ...running, paused: true }, progress(), 0).title).toBe("Paused");
  expect(chipText(queue({ done: [job(4, "done")] }), null, 0)).toEqual({
    title: "All saved",
    label: "Jobs: all saved",
  });
  expect(chipText(queue(), null, 0).title).toBe("No jobs");
});

test("the popover's line names the phase and the time left", () => {
  expect(nowText(running, progress({ phase: "check" }))).toBe("Checking · 1:12 left");
  expect(nowText(running, progress({ left_s: null }))).toBe("Encoding");
  expect(nowText({ ...running, paused: true }, progress())).toBe("Paused");
});
