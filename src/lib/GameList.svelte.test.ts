import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { render, screen } from "@testing-library/svelte";
import { afterEach, expect, test } from "vitest";
import GameList from "./GameList.svelte";

afterEach(() => clearMocks());

const source = (name: string, duration_s: number, height: number, decision: string | null) => ({
  path: `PEAK/${name}`,
  name,
  duration_s,
  height,
  decision,
});

test("lists each game's videos with their length, height and decision", async () => {
  mockIPC((cmd) =>
    cmd === "games"
      ? [
          { name: "", sources: [source("loose.mp4", 9.6, 720, null)] },
          {
            name: "PEAK",
            sources: [source("a.mp4", 150.27, 1440, "queue"), source("b.mp4", 3725, 1080, "reject")],
          },
        ]
      : undefined,
  );
  render(GameList);
  expect(await screen.findByRole("heading", { name: "PEAK" })).toBeTruthy();
  expect(screen.getByRole("heading", { name: "Outside game folders" })).toBeTruthy();
  expect(screen.getByText("loose.mp4 · 0:09 · 720p")).toBeTruthy();
  expect(screen.getByText("a.mp4 · 2:30 · 1440p · Queued")).toBeTruthy();
  expect(screen.getByText("b.mp4 · 1:02:05 · 1080p · Rejected")).toBeTruthy();
});

test("says why the list couldn't load", async () => {
  mockIPC(() => {
    throw "database is locked";
  });
  render(GameList);
  expect(await screen.findByText("Couldn't list the videos: database is locked")).toBeTruthy();
});
