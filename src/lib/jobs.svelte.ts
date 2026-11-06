import { listen } from "@tauri-apps/api/event";
import { JOB_PROGRESS, type JobProgress, QUEUE_CHANGED, type Queue, queue } from "./api";

const empty: Queue = { running: null, pending: [], done: [], paused: false };

// The queue shared by the header's chip and its popover; `seen` is the newest failed job the popover has shown.
export const store = $state<{ queue: Queue; progress: JobProgress | null; seen: number }>({
  queue: empty,
  progress: null,
  seen: 0,
});

export async function refresh() {
  store.queue = await queue();
  if (store.progress?.id !== store.queue.running?.id) store.progress = null;
}

// Loads the queue and follows its events; returns the stop.
export async function follow() {
  const stops = await Promise.all([
    listen(QUEUE_CHANGED, () => refresh().catch(console.error)),
    listen<JobProgress>(JOB_PROGRESS, (e) => (store.progress = e.payload)),
  ]);
  await refresh();

  return () => {
    for (const stop of stops) stop();
  };
}
