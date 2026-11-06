import type { Job, JobProgress, Queue } from "./api";
import { formatLength } from "./library";

export type Phase = "encode" | "check" | "paused" | "saved" | "idle";

// What the running job has done, once its first `job-progress` has come.
const progressOf = (queue: Queue, progress: JobProgress | null) =>
  queue.running && progress?.id === queue.running.id ? progress : null;

export function phase(queue: Queue, progress: JobProgress | null): Phase {
  if (queue.paused) return "paused";
  if (queue.running || queue.pending.length) return progressOf(queue, progress)?.phase ?? "encode";

  return queue.done.some((j) => j.status === "done") ? "saved" : "idle";
}

// Whole numbers; null when nothing runs.
export function percent(queue: Queue, progress: JobProgress | null) {
  if (!queue.running) return null;
  const p = progressOf(queue, progress);
  if (!p || p.kept_s <= 0) return 0;

  return Math.min(100, Math.max(0, Math.round((p.done_s / p.kept_s) * 100)));
}

// `m:ss`; null until there is a rate.
export function timeLeft(queue: Queue, progress: JobProgress | null) {
  const left = progressOf(queue, progress)?.left_s;

  return left == null ? null : formatLength(Math.ceil(left));
}

// The jobs behind the one running.
export const waiting = (queue: Queue) => queue.pending.length;

// Failed jobs among the done that the popover has not shown yet; `seen` is the newest failed id it showed.
export const failures = (done: Job[], seen: number) => done.filter((j) => j.status === "failed" && j.id > seen).length;

export const newestFailure = (done: Job[]) =>
  Math.max(0, ...done.filter((j) => j.status === "failed").map((j) => j.id));

const NAMES: Record<Phase, string> = {
  encode: "Encoding",
  check: "Checking",
  paused: "Paused",
  saved: "All saved",
  idle: "No jobs",
};

// The chip's words: the tooltip, and the longer label for assistive tech.
export function chipText(queue: Queue, progress: JobProgress | null, failed: number) {
  const p = phase(queue, progress);
  const pct = percent(queue, progress);
  const left = timeLeft(queue, progress);
  const title = left && (p === "encode" || p === "check") ? `${NAMES[p]} · ${left} left` : NAMES[p];
  const parts = [
    NAMES[p].toLowerCase(),
    pct === null ? null : `${pct}%`,
    left && p !== "paused" ? `${left} left` : null,
    waiting(queue) ? `${waiting(queue)} waiting` : null,
    failed ? `${failed} failed` : null,
  ];

  return { title, label: `Jobs: ${parts.filter(Boolean).join(", ")}` };
}

// The popover's line under the running job's title.
export function nowText(queue: Queue, progress: JobProgress | null) {
  const p = phase(queue, progress);
  const left = timeLeft(queue, progress);

  return left && (p === "encode" || p === "check") ? `${NAMES[p]} · ${left} left` : NAMES[p];
}
