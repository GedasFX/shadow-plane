import { convertFileSrc, invoke } from "@tauri-apps/api/core";

export type Status = "review" | "saved" | "discarded";

export type Video = {
  key: string;
  game: string;
  // Local time, "2024-02-20T23:46:01".
  date: string;
  duration_s: number;
  size: number;
  status: Status;
  saved_clips: number;
};

export type Piece = { start_s: number; end_s: number };

export type SavedClip = {
  title: string;
  game: string;
  great: boolean;
  best: boolean;
  pieces: Piece[];
  file_name: string;
};

export type VideoDetail = Video & { width: number; height: number; fps: number; clips: SavedClip[] };

export type ClipInput = { title: string; game: string; great: boolean; best: boolean; pieces: Piece[] };

export type Scan = { sources: number; unreadable: number };

// Errors reject with a string: a house code (<entity>.<property>.<rule>) or a message.
export const update = () => invoke<void>("update");
export const library = () => invoke<string | null>("library");
// The native folder dialog; null when cancelled.
export const chooseLibrary = () => invoke<string | null>("choose_library");
export const scan = () => invoke<Scan>("scan");
export const videos = () => invoke<Video[]>("videos");
// null when the key is not in the index.
export const video = (key: string) => invoke<VideoDetail | null>("video", { key });
export const clipFileName = (key: string, clip: ClipInput) => invoke<string>("clip_file_name", { key, clip });
export const save = (key: string, clip: ClipInput) => invoke<void>("save", { key, clip });
export const discard = (key: string) => invoke<void>("discard", { key });

// The `media` protocol. The thumbnail is a 320 px JPEG made on first request, one ffmpeg at a time:
// load grids lazily.
export const videoUrl = (key: string) => convertFileSrc(`video/${key}`, "media");
export const thumbUrl = (key: string) => convertFileSrc(`thumb/${key}`, "media");
