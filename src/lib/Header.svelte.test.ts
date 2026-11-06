import { clearMocks, mockConvertFileSrc, mockIPC } from "@tauri-apps/api/mocks";
import { cleanup, render } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test } from "vitest";
import Header from "./Header.svelte";
import { store } from "./jobs.svelte";

beforeEach(() => {
  mockConvertFileSrc("windows");
  mockIPC(() => undefined);
});
afterEach(() => {
  cleanup();
  clearMocks();
  store.queue = { running: null, pending: [], done: [], paused: false };
  store.progress = null;
});

const props = { screen: "library", version: "", video: null, nav: { total: 0, index: 0 }, onscreen() {}, onstep() {} };
const job = { id: 1, status: "running", key: "a.mp4", title: "t", game: "g", file_name: "t.mp4", kept_s: 60 };

test("a line along the bottom edge shows the running job's percentage, none when nothing runs", () => {
  // biome-ignore lint/suspicious/noExplicitAny: a partial header prop set
  const { container } = render(Header, props as any);
  expect(container.querySelector(".line")).toBeNull();

  // biome-ignore lint/suspicious/noExplicitAny: a partial job
  store.queue = { running: job as any, pending: [], done: [], paused: false };
  store.progress = { id: 1, phase: "encode", done_s: 15, kept_s: 60, left_s: 30 };
  return Promise.resolve().then(() =>
    expect((container.querySelector(".line") as HTMLElement).style.width).toBe("25%"),
  );
});
