import { clearMocks, mockConvertFileSrc, mockIPC } from "@tauri-apps/api/mocks";
import { fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import Page from "./+page.svelte";

beforeEach(() => mockConvertFileSrc("windows"));
afterEach(() => clearMocks());

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
  mockIPC((cmd) => {
    calls.push(cmd);
    if (cmd === "plugin:app|version") return "1.2.3";
    if (cmd === "library") return "Z:\\Shadowplays";
    if (cmd === "videos") return [video];
    if (cmd === "video") return { ...video, width: 1920, height: 1080, fps: 60, clips: [] };
    return handlers[cmd]?.();
  });

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
