import { clearMocks, mockConvertFileSrc, mockIPC } from "@tauri-apps/api/mocks";
import { fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { ClipInput, VideoDetail } from "./api";
import Review from "./Review.svelte";
import { store } from "./videos.svelte";

beforeEach(() => mockConvertFileSrc("windows"));
afterEach(() => {
  clearMocks();
  onopen.mockReset();
  onnotice.mockReset();
  store.videos = [];
  vi.useRealTimers();
  vi.restoreAllMocks();
});

const detail: VideoDetail = {
  key: "PEAK/a.mp4",
  game: "PEAK",
  date: "2025-10-25T23:31:09",
  duration_s: 100,
  size: 1e9,
  status: "review",
  saved_clips: 1,
  width: 1920,
  height: 1080,
  fps: 50,
  clips: [
    {
      title: "Earlier",
      game: "PEAK",
      great: false,
      best: false,
      pieces: [{ start_s: 5, end_s: 10 }],
      file_name: "earlier.mp4",
    },
  ],
};

type Args = { key: string; clip: ClipInput };

const onopen = vi.fn();
const onnotice = vi.fn();
const other = { ...detail, key: "PEAK/b.mp4", status: "review" as const, clips: [], saved_clips: 0 };

async function open(handlers: Record<string, (args: Args) => unknown> = {}) {
  mockIPC((cmd, args) => (cmd === "video" ? detail : handlers[cmd]?.(args as Args)));
  const view = render(Review, { open: "PEAK/a.mp4", onopen, onnotice });
  await screen.findByRole("button", { name: "Play" });
  const el = view.container.querySelector("video") as HTMLVideoElement;
  const track = view.container.querySelector(".track") as HTMLElement;
  track.getBoundingClientRect = () => ({ left: 0, width: 1000 }) as DOMRect;
  const pieces = () => [...view.container.querySelectorAll<HTMLElement>(".piece")];

  return { ...view, el, track, pieces };
}

// The video shows a frame: with no frame callback in happy-dom, the page follows timeupdate.
async function showFrame(el: HTMLVideoElement, t: number) {
  el.currentTime = t;
  await fireEvent(el, new Event("timeupdate"));
}

// happy-dom's WheelEvent drops the modifier keys, so the event is built by hand.
const wheelEvent = (init: { ctrlKey?: boolean; deltaY: number; clientX: number }) =>
  Object.assign(new Event("wheel", { cancelable: true }), { ctrlKey: false, deltaX: 0, ...init });

test("opens a video: its file, one green piece end to end and the saved clip hatched", async () => {
  const { el, pieces, container } = await open();

  expect(el.src).toContain("video%2FPEAK%2Fa.mp4");
  expect(pieces()).toHaveLength(1);
  expect(pieces()[0].style.width).toBe("100%");
  expect(container.querySelectorAll(".saved")).toHaveLength(1);
  expect(screen.getByLabelText("Time").textContent).toContain("0:00.00");
  expect(screen.getByLabelText("Time").textContent).toContain("1:40");
});

test("with no video open it says so", () => {
  render(Review, { open: null, onopen: vi.fn(), onnotice: vi.fn() });
  expect(screen.getByText("No video open")).toBeTruthy();
});

test("play starts the video", async () => {
  const { el } = await open();
  const play = vi.spyOn(el, "play").mockResolvedValue();

  await fireEvent.click(screen.getByRole("button", { name: "Play" }));
  expect(play).toHaveBeenCalled();

  await fireEvent(el, new Event("play"));
  expect(screen.getByRole("button", { name: "Pause" })).toBeTruthy();
});

test("a frame step asks for the middle of the next frame and shows its time", async () => {
  const { el } = await open();
  await showFrame(el, 2);

  await fireEvent.keyDown(window, { key: "ArrowRight" });

  expect(el.currentTime).toBeCloseTo((101 + 0.5) / 50);
  expect(screen.getByLabelText("Time").textContent).toContain("0:02.01");
});

test("holding a frame button repeats at 24 steps a second after a short wait", async () => {
  vi.useFakeTimers();
  const { el } = await open();
  const forward = screen.getByRole("button", { name: "Forward one frame" });

  await fireEvent.mouseDown(forward);
  expect(el.currentTime).toBeCloseTo(1.5 / 50);

  vi.advanceTimersByTime(350 + (1000 / 24) * 3 + 5);
  await fireEvent.mouseUp(forward);
  expect(el.currentTime).toBeCloseTo(4.5 / 50);

  vi.advanceTimersByTime(1000);
  expect(el.currentTime).toBeCloseTo(4.5 / 50);
});

test("start here and end here move the selected piece's ends to the playhead", async () => {
  const { el, pieces } = await open();

  await showFrame(el, 20);
  await fireEvent.click(screen.getByRole("button", { name: "Start here" }));
  await showFrame(el, 60);
  await fireEvent.click(screen.getByRole("button", { name: "End here" }));

  expect(pieces()[0].style.left).toBe("20%");
  expect(pieces()[0].style.width).toBe("40%");
});

test("the keys I and O do the same", async () => {
  const { el, pieces } = await open();

  await showFrame(el, 10);
  await fireEvent.keyDown(window, { key: "i" });
  await showFrame(el, 30);
  await fireEvent.keyDown(window, { key: "o" });

  expect(pieces()[0].style.left).toBe("10%");
  expect(pieces()[0].style.width).toBe("20%");
});

test("dragging a green end moves it, on a whole frame, with the time bubble", async () => {
  const { pieces, container } = await open();
  const end = container.querySelector(".grip.end") as HTMLElement;

  await fireEvent.mouseDown(end);
  await fireEvent.mouseMove(window, { clientX: 500 });

  expect(pieces()[0].style.width).toBe("50%");
  expect(container.querySelector(".bubble")?.textContent).toBe("0:50.00");

  await fireEvent.mouseUp(window);
  expect(container.querySelector(".bubble")).toBeNull();
});

test("dragging anywhere on the timeline moves the playhead", async () => {
  const { el, track } = await open();

  await fireEvent.mouseDown(track, { clientX: 250 });
  expect(el.currentTime).toBe(25);

  await fireEvent.mouseMove(window, { clientX: 750 });
  expect(el.currentTime).toBe(75);
  expect(screen.getByLabelText("Time").textContent).toContain("1:15.00");
});

test("Ctrl + scroll zooms around the mouse, Fit shows the whole video again", async () => {
  const { track, pieces } = await open();
  expect(screen.queryByRole("button", { name: "Show the whole video" })).toBeNull();

  const wheel = wheelEvent({ ctrlKey: true, deltaY: -100, clientX: 500 });
  track.parentElement?.dispatchEvent(wheel);
  await vi.waitFor(() => expect(screen.getByRole("button", { name: "Show the whole video" })).toBeTruthy());
  expect(wheel.defaultPrevented).toBe(true);
  expect(pieces()[0].style.width).toBe("125%");

  await fireEvent.click(screen.getByRole("button", { name: "Show the whole video" }));
  expect(pieces()[0].style.width).toBe("100%");
});

test("scroll pans only when zoomed", async () => {
  const { track, pieces } = await open();
  const wrap = track.parentElement as HTMLElement;

  const flat = wheelEvent({ deltaY: 100, clientX: 500 });
  wrap.dispatchEvent(flat);
  expect(flat.defaultPrevented).toBe(false);

  wrap.dispatchEvent(wheelEvent({ ctrlKey: true, deltaY: -100, clientX: 0 }));
  await vi.waitFor(() => expect(pieces()[0].style.width).toBe("125%"));
  wrap.dispatchEvent(wheelEvent({ deltaY: 100, clientX: 500 }));
  await vi.waitFor(() => expect(pieces()[0].style.left).not.toBe("0%"));
});

test("play the kept part starts at the first piece and pauses at its end", async () => {
  const { el } = await open();
  const play = vi.spyOn(el, "play").mockResolvedValue();
  const pause = vi.spyOn(el, "pause");
  await showFrame(el, 20);
  await fireEvent.click(screen.getByRole("button", { name: "Start here" }));

  await fireEvent.click(screen.getByRole("button", { name: "Play the kept part" }));
  expect(el.currentTime).toBe(20);
  expect(play).toHaveBeenCalled();

  await showFrame(el, 99.99);
  expect(pause).toHaveBeenCalled();
});

test("speed cycles through 0.5, 1 and 2", async () => {
  const { el } = await open();
  const speed = screen.getByRole("button", { name: "Speed" });

  await fireEvent.click(speed);
  expect(el.playbackRate).toBe(2);
  await fireEvent.click(speed);
  expect(el.playbackRate).toBe(0.5);
  expect(speed.textContent).toBe("0.5×");
});

test("mute silences the video", async () => {
  const { el } = await open();

  await fireEvent.click(screen.getByRole("button", { name: "Mute" }));
  expect(el.muted).toBe(true);
  expect(screen.getByRole("button", { name: "Unmute" })).toBeTruthy();
});

const queue = [detail, other].map(({ clips: _, width, height, fps, ...v }) => v);

async function fill(title = "Ramp") {
  await fireEvent.input(screen.getByLabelText("Title"), { target: { value: title } });
}

test("the game starts as the video's folder, amber for Base Profile", async () => {
  await open();
  expect((screen.getByLabelText("Game") as HTMLInputElement).value).toBe("PEAK");
  expect(screen.getByLabelText("Game").classList.contains("amber")).toBe(false);

  await fireEvent.input(screen.getByLabelText("Game"), { target: { value: "Base Profile" } });
  expect(screen.getByLabelText("Game").classList.contains("amber")).toBe(true);
});

test("the file name follows the fields, after a pause", async () => {
  const calls: Args[] = [];
  await open({
    clip_file_name: (args) => {
      calls.push(args);

      return `${args.clip.title}.mp4`;
    },
  });
  await fill("Ramp");
  await fireEvent.click(screen.getByRole("button", { name: "Best" }));
  await vi.waitFor(() => expect(screen.getByLabelText("File name").textContent).toBe("Ramp.mp4"));
  expect(calls.at(-1)?.clip).toMatchObject({ title: "Ramp", best: true, pieces: [{ start_s: 0, end_s: 100 }] });
});

test("Save without a title marks it red and saves nothing", async () => {
  const save = vi.fn();
  await open({ save });
  await fireEvent.click(screen.getByRole("button", { name: "Save" }));

  expect(screen.getByLabelText("Title").getAttribute("aria-invalid")).toBe("true");
  expect(save).not.toHaveBeenCalled();
});

test("Save keeps the clip and opens the next video to review", async () => {
  const save = vi.fn();
  store.videos = queue;
  await open({ save, videos: () => [{ ...queue[0], status: "saved" }, queue[1]] });
  await fill("  Ramp ");
  await fireEvent.click(screen.getByRole("button", { name: "Save" }));

  await vi.waitFor(() => expect(onopen).toHaveBeenCalledWith("PEAK/b.mp4"));
  expect(save).toHaveBeenCalledWith({
    key: "PEAK/a.mp4",
    clip: { title: "  Ramp ", game: "PEAK", great: false, best: false, pieces: [{ start_s: 0, end_s: 100 }] },
  });
  expect(onnotice).toHaveBeenCalledWith("saved", "Saved “Ramp”");
});

test("a failed Save says so and stays", async () => {
  await open({
    save: () => {
      throw "clip.pieces.range";
    },
  });
  await fill();
  await fireEvent.click(screen.getByRole("button", { name: "Save" }));

  await vi.waitFor(() => expect(onnotice).toHaveBeenCalledWith("error", "Couldn't save: clip.pieces.range"));
  expect(onopen).not.toHaveBeenCalled();
});

test("with no video left to review, Save opens none", async () => {
  await open({ save: () => undefined, videos: () => [{ ...queue[0], status: "saved" }] });
  await fill();
  await fireEvent.click(screen.getByRole("button", { name: "Save" }));

  await vi.waitFor(() => expect(onopen).toHaveBeenCalledWith(null));
});

test("Save and make another stays, with a fresh clip and the saved range hatched", async () => {
  let saved = false;
  const grown = { ...detail, clips: [...detail.clips, { ...detail.clips[0], pieces: [{ start_s: 20, end_s: 30 }] }] };
  const { container } = await open();
  mockIPC((cmd) => {
    if (cmd === "save") saved = true;
    if (cmd === "video") return saved ? grown : detail;
    if (cmd === "videos") return [];
  });
  await fill();
  await fireEvent.click(screen.getByRole("button", { name: "More ways to save" }));
  await fireEvent.click(screen.getByRole("menuitem", { name: "Save and make another" }));

  await vi.waitFor(() => expect(container.querySelectorAll(".saved")).toHaveLength(2));
  expect((screen.getByLabelText("Title") as HTMLInputElement).value).toBe("");
  expect(onopen).not.toHaveBeenCalled();
  expect(onnotice).toHaveBeenCalledWith("saved", "Saved “Ramp”");
});

test("Discard asks first, then bins the video and opens the next", async () => {
  const discard = vi.fn();
  store.videos = queue;
  await open({ discard, videos: () => [{ ...queue[0], status: "discarded" }, queue[1]] });

  await fireEvent.click(screen.getByRole("button", { name: "Discard this video" }));
  expect(screen.getByText("It moves to the _Bin folder.")).toBeTruthy();
  await fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(discard).not.toHaveBeenCalled();

  await fireEvent.click(screen.getByRole("button", { name: "Discard this video" }));
  await fireEvent.click(screen.getByRole("button", { name: "Discard" }));
  await vi.waitFor(() => expect(onopen).toHaveBeenCalledWith("PEAK/b.mp4"));
  expect(discard).toHaveBeenCalledWith({ key: "PEAK/a.mp4" });
  expect(onnotice).toHaveBeenCalledWith("discarded", "Discarded 25 Oct 23:31");
});

test("a piece's menu splits it, and removes one of several", async () => {
  const { pieces } = await open();
  await fireEvent.click(screen.getByRole("button", { name: "More" }));
  expect(screen.queryByRole("menuitem", { name: "Remove" })).toBeNull();
  await fireEvent.click(screen.getByRole("menuitem", { name: "Split" }));
  expect(pieces()).toHaveLength(2);

  await fireEvent.click(screen.getAllByRole("button", { name: "More" })[0]);
  await fireEvent.click(screen.getByRole("menuitem", { name: "Remove" }));
  expect(pieces()).toHaveLength(1);
});

test("clicking a piece row selects it; dragging a row reorders", async () => {
  await open();
  await fireEvent.click(screen.getByRole("button", { name: "More" }));
  await fireEvent.click(screen.getByRole("menuitem", { name: "Split" }));
  const rows = () => [...document.querySelectorAll<HTMLElement>("aside .row")];
  const ranges = () => rows().map((r) => r.querySelector(".range")?.textContent);
  const before = ranges();

  await fireEvent.click(screen.getByRole("button", { name: "Piece 1" }));
  expect(rows()[0].classList.contains("selected")).toBe(true);

  await fireEvent.dragStart(rows()[0]);
  await fireEvent.drop(rows()[1]);
  expect(ranges()).toEqual([before[1], before[0]]);
});
