import { clearMocks, mockConvertFileSrc, mockIPC } from "@tauri-apps/api/mocks";
import { fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { Video } from "./api";
import Library from "./Library.svelte";
import { store } from "./videos.svelte";

beforeEach(() => mockConvertFileSrc("windows"));
afterEach(() => {
  clearMocks();
  store.videos = [];
});

const video = (key: string, game: string, status: Video["status"], saved_clips = 0): Video => ({
  key,
  game,
  date: "2025-10-25T23:31:09",
  duration_s: 379,
  size: 9e8,
  status,
  saved_clips,
});

const all = [
  video("Make Way/a.mp4", "Make Way", "review"),
  video("Make Way/b.mp4", "Make Way", "saved", 2),
  video("PEAK/Climb.mp4", "PEAK", "review"),
  video("PEAK/d.mp4", "PEAK", "discarded"),
];

type Handlers = { saved?: string | null; picked?: string | null; scan?: () => unknown };

function app(onopen = vi.fn(), { saved = "Z:\\Shadowplays", picked = null, scan }: Handlers = {}) {
  const calls: string[] = [];
  mockIPC((cmd) => {
    calls.push(cmd);
    switch (cmd) {
      case "library":
        return saved;
      case "choose_library":
        return picked;
      case "scan":
        return scan ? scan() : { sources: all.length, unreadable: 0 };
      case "videos":
        return all;
    }
  });
  render(Library, { onopen });

  return calls;
}

test("with no folder chosen, shows only a choose-folder button", async () => {
  app(vi.fn(), { saved: null });
  expect(await screen.findByRole("button", { name: "Choose folder" })).toBeTruthy();
  expect(screen.queryByLabelText("Find a video")).toBeNull();
});

test("choosing a folder scans it and lists its videos", async () => {
  const calls = app(vi.fn(), { saved: null, picked: "Z:\\Clips" });
  await fireEvent.click(await screen.findByRole("button", { name: "Choose folder" }));
  expect(await screen.findByTitle("a.mp4")).toBeTruthy();
  expect(calls).toContain("scan");
  expect(screen.getByRole("button", { name: "Library folder" }).title).toBe("Z:\\Clips");
});

test("scans at launch when a folder is set, starting on To review", async () => {
  const calls = app();
  expect(await screen.findByTitle("a.mp4")).toBeTruthy();
  expect(calls).toContain("scan");
  expect(screen.getByRole("button", { name: "To review" }).getAttribute("aria-pressed")).toBe("true");
  expect(screen.getByTitle("Climb.mp4")).toBeTruthy();
  expect(screen.queryByTitle("b.mp4")).toBeNull();
});

test("the status filters switch what the grid shows", async () => {
  app();
  await screen.findByTitle("a.mp4");
  await fireEvent.click(screen.getByRole("button", { name: "Discarded" }));
  expect(screen.getByTitle("d.mp4")).toBeTruthy();
  expect(screen.getByRole("img", { name: "Discarded" })).toBeTruthy();
  expect(screen.queryByTitle("a.mp4")).toBeNull();
  await fireEvent.click(screen.getByRole("button", { name: "Saved" }));
  expect(screen.getByTitle("b.mp4").textContent).toContain("2");
});

test("search narrows by file name or game", async () => {
  app();
  await screen.findByTitle("a.mp4");
  await fireEvent.input(screen.getByLabelText("Find a video"), { target: { value: "climb" } });
  expect(screen.getByTitle("Climb.mp4")).toBeTruthy();
  expect(screen.queryByTitle("a.mp4")).toBeNull();
});

test("the games filter ticks games, partly ticks All games and names the choice", async () => {
  app();
  await screen.findByTitle("a.mp4");
  const trigger = screen.getByRole("button", { name: "All games" });
  await fireEvent.click(trigger);
  expect(screen.getByRole("option", { name: "Make Way 1" })).toBeTruthy();
  await fireEvent.click(screen.getByRole("option", { name: "PEAK 1" }));
  expect(trigger.textContent).toContain("PEAK");
  expect(screen.queryByTitle("a.mp4")).toBeNull();
  expect(screen.getByRole("option", { name: "All games" }).getAttribute("aria-selected")).toBe("false");
  await fireEvent.click(screen.getByRole("option", { name: "Make Way 1" }));
  expect(trigger.textContent).toContain("2 games");
  await fireEvent.click(screen.getByRole("option", { name: "All games" }));
  expect(trigger.textContent).toContain("All games");
});

test("the games popover has its own search", async () => {
  app();
  await screen.findByTitle("a.mp4");
  await fireEvent.click(screen.getByRole("button", { name: "All games" }));
  await fireEvent.input(screen.getByLabelText("Find a game"), { target: { value: "pea" } });
  expect(screen.queryByRole("option", { name: "Make Way 1" })).toBeNull();
  expect(screen.getByRole("option", { name: "PEAK 1" })).toBeTruthy();
  expect(screen.getByRole("option", { name: "All games" })).toBeTruthy();
});

test("clicking a video hands its key over", async () => {
  const onopen = vi.fn();
  app(onopen);
  await fireEvent.click(await screen.findByTitle("Climb.mp4"));
  expect(onopen).toHaveBeenCalledWith("PEAK/Climb.mp4");
});

test("the rescan button scans again", async () => {
  const calls = app();
  await screen.findByTitle("a.mp4");
  await fireEvent.click(screen.getByRole("button", { name: "Rescan" }));
  await vi.waitFor(() => expect(calls.filter((c) => c === "scan")).toHaveLength(2));
});

test("says why a scan failed", async () => {
  app(vi.fn(), {
    scan: () => {
      throw "no ffprobe";
    },
  });
  expect(await screen.findByText("Couldn't scan: no ffprobe")).toBeTruthy();
});
