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

export type JobStatus = "pending" | "running" | "done" | "failed" | "cancelled";

export type Job = {
  id: number;
  status: JobStatus;
  // The video the clip is cut from.
  key: string;
  title: string;
  game: string;
  file_name: string;
  kept_s: number;
  // The finished file below the library folder.
  output: string | null;
  error: string | null;
};

// `done` holds the last 5 finished or failed, newest first.
export type Queue = { running: Job | null; pending: Job[]; done: Job[]; paused: boolean };

export type JobPhase = "encode" | "check";

// Events the page listens to: `queue-changed` has no payload (call `queue()`); `job-progress` comes at
// most 4 a second while a job runs. `left_s` is null until there is a rate.
export type JobProgress = { id: number; phase: JobPhase; done_s: number; kept_s: number; left_s: number | null };
export const QUEUE_CHANGED = "queue-changed";
export const JOB_PROGRESS = "job-progress";

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
export const queue = () => invoke<Queue>("queue");
// Every pending job's id, in the new order.
export const reorderJobs = (ids: number[]) => invoke<void>("reorder_jobs", { ids });
// Cancels a pending job.
export const removeJob = (id: number) => invoke<void>("remove_job", { id });
// Pause freezes the running encode and holds the queue; resume lets both go on. Neither is remembered across a restart.
export const pause = () => invoke<void>("pause");
export const resume = () => invoke<void>("resume");
// Kills the running job, or removes a pending one.
export const cancelJob = (id: number) => invoke<void>("cancel_job", { id });

// The `media` protocol. The thumbnail is a 320 px JPEG made on first request, one ffmpeg at a time:
// load grids lazily.
export const videoUrl = (key: string) => convertFileSrc(`video/${key}`, "media");
export const thumbUrl = (key: string) => convertFileSrc(`thumb/${key}`, "media");
