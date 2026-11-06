import { expect, test } from "vitest";
import type { Video } from "./api";
import { formatDate, formatLength, formatSize, gameCounts, gamesLabel, groupByGame, matching } from "./library";

const video = (key: string, game: string, status: Video["status"]): Video => ({
  key,
  game,
  date: "2025-10-25T23:31:09",
  duration_s: 60,
  size: 1e9,
  status,
  saved_clips: 0,
});

const videos = [
  video("Make Way/a.mp4", "Make Way", "review"),
  video("Make Way/b.mp4", "Make Way", "saved"),
  video("PEAK/Climb.mp4", "PEAK", "review"),
  video("PEAK/d.mp4", "PEAK", "discarded"),
];

const keys = (list: Video[]) => list.map((v) => v.key);

test("the status filter keeps one status, All keeps every one", () => {
  expect(keys(matching(videos, "saved", ""))).toEqual(["Make Way/b.mp4"]);
  expect(matching(videos, "all", "")).toHaveLength(4);
});

test("search matches the file name and the game, ignoring case", () => {
  expect(keys(matching(videos, "all", "CLIMB"))).toEqual(["PEAK/Climb.mp4"]);
  expect(keys(matching(videos, "review", "make way"))).toEqual(["Make Way/a.mp4"]);
});

test("a game's count follows the status filter and the search", () => {
  expect(gameCounts(videos, "review", "")).toEqual([
    { name: "Make Way", count: 1 },
    { name: "PEAK", count: 1 },
  ]);
  expect(gameCounts(videos, "discarded", "")).toEqual([
    { name: "Make Way", count: 0 },
    { name: "PEAK", count: 1 },
  ]);
});

test("videos are grouped by game, limited to the ticked games", () => {
  const groups = groupByGame(videos, { filter: "all", search: "", games: ["PEAK"] });
  expect(groups.map((g) => g.name)).toEqual(["PEAK"]);
  expect(keys(groups[0].videos)).toEqual(["PEAK/Climb.mp4", "PEAK/d.mp4"]);
  expect(groupByGame(videos, { filter: "review", search: "", games: [] }).map((g) => g.name)).toEqual([
    "Make Way",
    "PEAK",
  ]);
});

test("the games button reads All games, the game or a count", () => {
  expect(gamesLabel([])).toBe("All games");
  expect(gamesLabel(["PEAK"])).toBe("PEAK");
  expect(gamesLabel(["PEAK", "Make Way"])).toBe("2 games");
});

test("formats lengths, dates and sizes", () => {
  expect(formatLength(9.6)).toBe("0:09");
  expect(formatLength(379)).toBe("6:19");
  expect(formatLength(3725)).toBe("1:02:05");
  expect(formatDate("2025-10-25T23:31:09")).toBe("25 Oct 23:31");
  expect(formatSize(220e6)).toBe("220 MB");
  expect(formatSize(1.84e9)).toBe("1.8 GB");
});
