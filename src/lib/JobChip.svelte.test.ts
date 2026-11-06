import { clearMocks, mockConvertFileSrc, mockIPC } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test } from "vitest";
import type { Job, Queue } from "./api";
import JobChip from "./JobChip.svelte";
import { store } from "./jobs.svelte";

beforeEach(() => mockConvertFileSrc("windows"));
afterEach(() => {
  cleanup();
  clearMocks();
  store.queue = { running: null, pending: [], done: [], paused: false };
  store.progress = null;
  store.seen = 0;
});

const job = (id: number, status: Job["status"] = "pending", extra: Partial<Job> = {}): Job => ({
  id,
  status,
  key: "PEAK/a.mp4",
  title: `clip ${id}`,
  game: "PEAK",
  file_name: `clip ${id}.mp4`,
  kept_s: 75,
  output: null,
  error: null,
  ...extra,
});

function app(queue: Partial<Queue>) {
  const calls: [string, unknown][] = [];
  store.queue = { running: null, pending: [], done: [], paused: false, ...queue };
  mockIPC((cmd, args) => {
    calls.push([cmd, args]);
  });

  return calls;
}

const chip = () => screen.getByRole("button", { name: /^Jobs:/ });

test("the chip tells the running job in words", () => {
  app({ running: job(1, "running"), pending: [job(2), job(3)], done: [job(9, "failed")] });
  store.progress = { id: 1, phase: "encode", done_s: 31.5, kept_s: 75, left_s: 72 };
  render(JobChip);

  expect(chip().getAttribute("aria-label")).toBe("Jobs: encoding, 42%, 1:12 left, 2 waiting, 1 failed");
  expect(chip().title).toBe("Encoding · 1:12 left");
  expect(chip().textContent).toContain("42%");
});

test("with no jobs the chip is a label, with no ring or numbers", () => {
  app({});
  const { container } = render(JobChip);

  expect(chip().title).toBe("No jobs");
  expect(chip().textContent?.trim()).toBe("No jobs");
  expect(container.querySelector(".ring")).toBeNull();
  expect(container.querySelector(".num")).toBeNull();
});

test("when all is saved the label says so and a failure count still shows", () => {
  app({ done: [job(8, "done"), job(9, "failed")] });
  const { container } = render(JobChip);

  expect(chip().getAttribute("aria-label")).toBe("Jobs: all saved, 1 failed");
  expect(chip().textContent).toContain("All saved");
  expect(container.querySelector(".fail")?.textContent).toBe("1");
  expect(container.querySelector(".ring")).toBeNull();
});

test("a running job brings the ring and the numbers", () => {
  app({ running: job(1, "running") });
  const { container } = render(JobChip);

  expect(container.querySelector(".ring")).not.toBeNull();
  expect(container.querySelector(".num")).not.toBeNull();
});

test("paused with a job keeps the ring", () => {
  app({ paused: true, pending: [job(2)] });
  const { container } = render(JobChip);

  expect(container.querySelector(".ring")).not.toBeNull();
});

test("opening the popover clears the failure count", async () => {
  app({ done: [job(9, "failed", { error: "disk full" }), job(8, "done", { output: "_Processed/PEAK/x.mp4" })] });
  render(JobChip);
  expect(chip().getAttribute("aria-label")).toContain("1 failed");

  await fireEvent.click(chip());
  await fireEvent.click(chip());

  expect(chip().getAttribute("aria-label")).not.toContain("failed");
});

test("the popover closes on Escape and on a click outside", async () => {
  app({ pending: [job(2)] });
  render(JobChip);

  await fireEvent.click(chip());
  expect(screen.getByRole("dialog")).toBeTruthy();
  await fireEvent.keyDown(window, { key: "Escape" });
  expect(screen.queryByRole("dialog")).toBeNull();

  await fireEvent.click(chip());
  await fireEvent.pointerDown(document.body);
  expect(screen.queryByRole("dialog")).toBeNull();
});

test("Now shows the running job, with pause and cancel", async () => {
  const calls = app({ running: job(1, "running", { title: "" }) });
  store.progress = { id: 1, phase: "check", done_s: 10, kept_s: 75, left_s: 5 };
  render(JobChip);
  await fireEvent.click(chip());

  expect(screen.getByText("clip 1.mp4")).toBeTruthy();
  expect(screen.getByText("Checking · 0:05 left")).toBeTruthy();
  await fireEvent.click(screen.getByRole("button", { name: "Pause" }));
  await fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

  expect(calls.map(([cmd]) => cmd)).toEqual(["pause", "cancel_job"]);
  expect(calls[1][1]).toEqual({ id: 1 });
});

test("paused, the popover offers Resume, even with nothing running", async () => {
  const calls = app({ paused: true, pending: [job(2)] });
  render(JobChip);
  await fireEvent.click(chip());

  expect(screen.queryByRole("button", { name: "Cancel" })).toBeNull();
  await fireEvent.click(screen.getByRole("button", { name: "Resume" }));

  expect(calls[0][0]).toBe("resume");
});

test("no Now with nothing running", async () => {
  app({ pending: [job(2)] });
  render(JobChip);
  await fireEvent.click(chip());

  expect(screen.queryByRole("button", { name: "Pause" })).toBeNull();
});

test("Up next removes a row", async () => {
  const calls = app({ pending: [job(2), job(3), job(4)] });
  render(JobChip);
  await fireEvent.click(chip());

  expect(screen.getAllByText("1:15")).toHaveLength(3);
  const rows = screen.getAllByText(/^clip/).map((n) => n.parentElement as HTMLElement);
  await fireEvent.click(within(rows[1]).getByRole("button", { name: "Remove from the queue" }));

  expect(calls).toEqual([["remove_job", { id: 3 }]]);
});

test("Done lists each job with play and its state", async () => {
  const calls = app({
    done: [job(9, "failed", { error: "disk full" }), job(8, "done", { output: "_Processed/PEAK/x.mp4" })],
  });
  render(JobChip);
  await fireEvent.click(chip());

  expect(screen.getByText("Failed").title).toBe("disk full");
  expect(screen.getByText("Saved")).toBeTruthy();
  const [failed, saved] = screen.getAllByRole("button", { name: "Play" }) as HTMLButtonElement[];
  expect(failed.disabled).toBe(true);
  await fireEvent.click(saved);

  expect(calls).toEqual([["play", { id: 8 }]]);
});

test("a command that fails shows its error", async () => {
  store.queue = { running: null, pending: [], done: [job(8, "done", { output: "x.mp4" })], paused: false };
  mockIPC(() => {
    throw "job.output.missing";
  });
  render(JobChip);
  await fireEvent.click(chip());
  await fireEvent.click(screen.getByRole("button", { name: "Play" }));

  expect((await screen.findByRole("alert")).textContent).toBe("job.output.missing");
});
