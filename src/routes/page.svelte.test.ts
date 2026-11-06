import { clearMocks, mockConvertFileSrc, mockIPC } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { store } from "$lib/videos.svelte";
import Page from "./+page.svelte";

beforeEach(() => mockConvertFileSrc("windows"));
// The page stops listening a tick after it unmounts, which needs the event mock still there.
afterEach(async () => {
  cleanup();
  await new Promise((done) => setTimeout(done));
  clearMocks();
  store.videos = [];
});

const video = {
  key: "PEAK/a.mp4",
  game: "PEAK",
  date: "2025-10-25T23:31:09",
  duration_s: 60,
  size: 1e9,
  status: "review",
  saved_clips: 0,
};

function app(handlers: Record<string, () => unknown> = {}) {
  const calls: string[] = [];
  mockIPC(
    (cmd) => {
      calls.push(cmd);
      if (cmd === "plugin:app|version") return "1.2.3";
      if (cmd === "library") return "Z:\\Shadowplays";
      if (cmd === "videos") return [video];
      if (cmd === "queue") return { running: null, pending: [], done: [], paused: false };
      if (cmd === "video") return { ...video, width: 1920, height: 1080, fps: 60, clips: [] };
      return handlers[cmd]?.();
    },
    { shouldMockEvents: true },
  );

  return calls;
}

test("the mascot's tooltip shows the app's version", async () => {
  app();
  render(Page);
  expect(await screen.findByTitle("Shadow Plane 1.2.3")).toBeTruthy();
  await screen.findByTitle("a.mp4");
});

test("checks for an update at launch and shows nothing of it", async () => {
  const calls = app();
  render(Page);
  await vi.waitFor(() => expect(calls).toContain("update"));
  await screen.findByTitle("a.mp4");
  expect(screen.queryByText(/update/i)).toBeNull();
});

test("a failed update check does not reach the screen", async () => {
  const log = vi.spyOn(console, "error").mockImplementation(() => {});
  app({
    update: () => {
      throw "offline";
    },
  });
  render(Page);
  await vi.waitFor(() => expect(log).toHaveBeenCalledWith("offline"));
  await screen.findByTitle("a.mp4");
  expect(screen.queryByText(/offline/)).toBeNull();
  log.mockRestore();
});

test("clicking a video opens it in Review", async () => {
  app();
  render(Page);
  await fireEvent.click(await screen.findByTitle("a.mp4"));
  expect(screen.getByRole("button", { name: "Review" }).getAttribute("aria-current")).toBe("page");
  expect(await screen.findByRole("button", { name: "Play" })).toBeTruthy();
});

test("Review with no video open says so", async () => {
  app();
  render(Page);
  await screen.findByTitle("a.mp4");
  await fireEvent.click(screen.getByRole("button", { name: "Review" }));
  expect(screen.getByText("No video open")).toBeTruthy();
});

test("on Review the header shows the video and walks the to-review videos", async () => {
  const list = [video, { ...video, key: "PEAK/b.mp4", date: "2025-10-26T10:00:00" }];
  mockIPC(
    (cmd) => {
      if (cmd === "library") return "Z:\\Shadowplays";
      if (cmd === "videos") return list;
      if (cmd === "queue") return { running: null, pending: [], done: [], paused: false };
      if (cmd === "video") return { ...list[0], width: 1920, height: 1080, fps: 60, clips: [] };
    },
    { shouldMockEvents: true },
  );
  render(Page);
  await fireEvent.click(await screen.findByTitle("a.mp4"));

  expect(await screen.findByText("1 of 2 to review")).toBeTruthy();
  expect(screen.getByText("25 Oct 23:31 · 1:00 · 1.0 GB")).toBeTruthy();
  expect((screen.getByRole("button", { name: "Previous video" }) as HTMLButtonElement).disabled).toBe(true);

  await fireEvent.click(screen.getByRole("button", { name: "Next video" }));
  expect(await screen.findByText("2 of 2 to review")).toBeTruthy();
  expect((screen.getByRole("button", { name: "Next video" }) as HTMLButtonElement).disabled).toBe(true);
});
