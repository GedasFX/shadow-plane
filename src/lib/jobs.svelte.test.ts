import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test, vi } from "vitest";
import type { Queue } from "./api";
import { follow, store } from "./jobs.svelte";

afterEach(() => {
  clearMocks();
  store.queue = { running: null, pending: [], done: [], paused: false };
  store.progress = null;
});

const job = (id: number) => ({ id, status: "running", key: "a.mp4", title: "t", kept_s: 60 });

test("follow loads the queue, reloads on queue-changed and keeps the latest progress", async () => {
  let current = { running: null, pending: [], done: [], paused: false } as unknown as Queue;
  mockIPC((cmd) => (cmd === "queue" ? current : undefined), { shouldMockEvents: true });
  const stop = await follow();
  expect(store.queue.running).toBeNull();

  current = { ...current, running: job(1) } as unknown as Queue;
  await emit("queue-changed");
  await vi.waitFor(() => expect(store.queue.running?.id).toBe(1));

  await emit("job-progress", { id: 1, phase: "encode", done_s: 10, kept_s: 60, left_s: 5 });
  await vi.waitFor(() => expect(store.progress?.done_s).toBe(10));

  current = { ...current, running: job(2) } as unknown as Queue;
  await emit("queue-changed");
  await vi.waitFor(() => expect(store.progress).toBeNull());
  stop();
});
