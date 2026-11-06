import type { Status, Video } from "./api";

export type Filter = Status | "all";

export const filters: { id: Filter; name: string }[] = [
  { id: "review", name: "To review" },
  { id: "saved", name: "Saved" },
  { id: "discarded", name: "Discarded" },
  { id: "all", name: "All" },
];

export type View = { filter: Filter; search: string; games: string[] };

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

export const fileName = (key: string) => key.slice(key.lastIndexOf("/") + 1);

// The status filter and the search, without the games filter: the games popover counts with these.
export function matching(videos: Video[], filter: Filter, search: string) {
  const needle = search.trim().toLowerCase();

  return videos.filter(
    (v) =>
      (filter === "all" || v.status === filter) &&
      (fileName(v.key).toLowerCase().includes(needle) || v.game.toLowerCase().includes(needle)),
  );
}

export function gameCounts(videos: Video[], filter: Filter, search: string) {
  const counts = new Map<string, number>();
  for (const v of videos) counts.set(v.game, 0);
  for (const v of matching(videos, filter, search)) counts.set(v.game, (counts.get(v.game) ?? 0) + 1);

  return [...counts].map(([name, count]) => ({ name, count })).sort((a, b) => a.name.localeCompare(b.name));
}

export function groupByGame(videos: Video[], view: View) {
  const groups = new Map<string, Video[]>();
  for (const v of matching(videos, view.filter, view.search)) {
    if (view.games.length && !view.games.includes(v.game)) continue;
    groups.set(v.game, [...(groups.get(v.game) ?? []), v]);
  }

  return [...groups].map(([name, list]) => ({ name, videos: list })).sort((a, b) => a.name.localeCompare(b.name));
}

export function gamesLabel(games: string[]) {
  if (games.length === 0) return "All games";

  return games.length === 1 ? games[0] : `${games.length} games`;
}

export function formatLength(seconds: number) {
  const s = Math.floor(seconds);
  const mmss = `${Math.floor((s % 3600) / 60)}:${String(s % 60).padStart(2, "0")}`;

  return s < 3600 ? mmss : `${Math.floor(s / 3600)}:${mmss.padStart(5, "0")}`;
}

// "2024-02-20T23:46:01" as "20 Feb 23:46".
export function formatDate(date: string) {
  const [day, time] = date.split("T");
  const [, month, d] = day.split("-").map(Number);

  return `${d} ${MONTHS[month - 1]} ${time.slice(0, 5)}`;
}

export function formatSize(bytes: number) {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;

  return `${Math.round(bytes / 1e6)} MB`;
}
