import type { ClipInput, Piece } from "./api";

export type Clip = { pieces: Piece[]; selected: number };

// What a clip carries besides its pieces; the game starts as the video's folder.
export type Meta = Omit<ClipInput, "pieces">;

export const MIN_PIECE = 0.5;

const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));

// A fresh video's clip is the whole video.
export const freshClip = (duration: number): Clip => ({ pieces: [{ start_s: 0, end_s: duration }], selected: 0 });

export const keptLength = (clip: Clip) => clip.pieces.reduce((sum, p) => sum + p.end_s - p.start_s, 0);

export const select = (clip: Clip, selected: number): Clip => ({ ...clip, selected });

// The other pieces bound a piece by where they sit in time, not by their order in the list.
export function setStart(clip: Clip, i: number, t: number): Clip {
  const piece = clip.pieces[i];
  const floor = Math.max(
    0,
    ...others(clip, i)
      .filter((p) => p.end_s <= piece.start_s)
      .map((p) => p.end_s),
  );

  return replace(clip, i, { ...piece, start_s: clamp(t, floor, piece.end_s - MIN_PIECE) });
}

export function setEnd(clip: Clip, i: number, t: number, duration: number): Clip {
  const piece = clip.pieces[i];
  const ceiling = Math.min(
    duration,
    ...others(clip, i)
      .filter((p) => p.start_s >= piece.end_s)
      .map((p) => p.start_s),
  );

  return replace(clip, i, { ...piece, end_s: clamp(t, piece.start_s + MIN_PIECE, ceiling) });
}

// Two halves with a small gap between them; the second becomes the selected piece.
export function split(clip: Clip, i: number): Clip {
  const { start_s, end_s } = clip.pieces[i];
  const mid = (start_s + end_s) / 2;
  const gap = Math.min(1, (end_s - start_s) / 10);
  const pieces = clip.pieces.slice();
  pieces.splice(i, 1, { start_s, end_s: mid - gap / 2 }, { start_s: mid + gap / 2, end_s });

  return { pieces, selected: i + 1 };
}

// The last piece can't be removed.
export function remove(clip: Clip, i: number): Clip {
  if (clip.pieces.length < 2) return clip;

  const pieces = clip.pieces.filter((_, j) => j !== i);
  const selected = clip.selected > i ? clip.selected - 1 : Math.min(clip.selected, pieces.length - 1);

  return { pieces, selected };
}

// Move a piece to a new place in the list; the selection stays on the same piece.
export function reorder(clip: Clip, from: number, to: number): Clip {
  const pieces = clip.pieces.slice();
  const [moved] = pieces.splice(from, 1);
  pieces.splice(to, 0, moved);
  const chosen = clip.pieces[clip.selected];

  return { pieces, selected: pieces.indexOf(chosen) };
}

// Playing the kept part, piece `at` shows a frame at `mediaTime`: carry on, move to the next piece or stop.
export function keptNext(clip: Clip, at: number, mediaTime: number, fps: number): "carry on" | "stop" | number {
  if (mediaTime < clip.pieces[at].end_s - 0.5 / fps) return "carry on";

  return at + 1 < clip.pieces.length ? at + 1 : "stop";
}

const others = (clip: Clip, i: number) => clip.pieces.filter((_, j) => j !== i);

function replace(clip: Clip, i: number, piece: Piece): Clip {
  const pieces = clip.pieces.slice();
  pieces[i] = piece;

  return { ...clip, pieces };
}

export const freshMeta = (game: string): Meta => ({ title: "", game, great: false, best: false });

export const clipInput = (meta: Meta, clip: Clip): ClipInput => ({ ...meta, pieces: clip.pieces });
