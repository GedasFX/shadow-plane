import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { refresh, store } from "./videos.svelte";

afterEach(() => clearMocks());

test("refresh loads the index into the shared list", async () => {
  mockIPC((cmd) => (cmd === "videos" ? [{ key: "PEAK/a.mp4" }] : undefined));
  await refresh();
  expect(store.videos.map((v) => v.key)).toEqual(["PEAK/a.mp4"]);

  mockIPC((cmd) => (cmd === "videos" ? [] : undefined));
  await refresh();
  expect(store.videos).toEqual([]);
});
