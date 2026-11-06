<script lang="ts">
  import { onMount } from "svelte";
  import { type Clip, keptNext } from "./clip";
  import { formatLength } from "./library";
  import { frameCentre, frameOf, isZoomed, type View, wholeView, withFrames, zoomLevel, zoomText } from "./timeline";

  type Props = {
    src: string;
    fps: number;
    duration: number;
    clip: Clip;
    view: View;
    time: number;
    onstart: (t: number) => void;
    onend: (t: number) => void;
  };

  let { src, fps, duration, clip, view = $bindable(), time = $bindable(), onstart, onend }: Props = $props();

  const RATES = [0.5, 1, 2];
  const REPEAT_AFTER = 350;
  const REPEAT_EVERY = 1000 / 24;

  let el: HTMLVideoElement;
  let playing = $state(false);
  let rate = $state(1);
  let volume = $state(1);
  let muted = $state(false);
  // The piece being played in "play the kept part", else null.
  let keptAt: number | null = null;
  // The frame a step asked for, until the video shows it: held steps build on it, not on the old frame.
  let stepped: number | null = null;
  let afterTimer: ReturnType<typeof setTimeout>;
  let repeatTimer: ReturnType<typeof setInterval>;

  const silent = $derived(muted || volume === 0);

  $effect(() => {
    el.playbackRate = rate;
    el.volume = volume;
    el.muted = muted;
  });

  onMount(() => {
    let id = 0;
    const frames = "requestVideoFrameCallback" in el;
    const watch = () => {
      id = el.requestVideoFrameCallback((_, meta) => {
        shown(meta.mediaTime);
        watch();
      });
    };
    const tick = () => shown(el.currentTime);
    if (frames) watch();
    else el.addEventListener("timeupdate", tick);

    return () => {
      if (frames) el.cancelVideoFrameCallback(id);
      else el.removeEventListener("timeupdate", tick);
      release();
    };
  });

  // The frame on screen: `currentTime` only says what was asked for, `mediaTime` what is painted.
  function shown(mediaTime: number) {
    if (el.seeking) return;

    stepped = null;
    time = mediaTime;
    if (keptAt === null) return;

    const next = keptNext(clip, keptAt, mediaTime, fps);
    if (next === "stop") {
      el.pause();
    } else if (next !== "carry on") {
      keptAt = next;
      el.currentTime = clip.pieces[next].start_s;
    }
  }

  const pause = () => {
    if (!el.paused) el.pause();
  };

  export function seek(t: number) {
    pause();
    keptAt = null;
    stepped = null;
    time = Math.max(0, Math.min(t, duration));
    el.currentTime = time;
  }

  function step(dir: number) {
    pause();
    keptAt = null;
    const last = Math.max(0, frameOf(duration, fps) - 1);
    stepped = Math.max(0, Math.min((stepped ?? frameOf(time, fps)) + dir, last));
    time = stepped / fps;
    el.currentTime = frameCentre(stepped, fps);
  }

  function hold(dir: number) {
    release();
    step(dir);
    afterTimer = setTimeout(() => {
      repeatTimer = setInterval(() => step(dir), REPEAT_EVERY);
    }, REPEAT_AFTER);
  }

  function release() {
    clearTimeout(afterTimer);
    clearInterval(repeatTimer);
  }

  // A key press has no mouse-down, so it arrives as a click with no pointer (detail 0).
  const stepByKey = (e: MouseEvent, dir: number) => {
    if (e.detail === 0) step(dir);
  };

  const play = () => el.play()?.catch(() => {});

  const toggle = () => (playing ? el.pause() : play());

  function playKept() {
    keptAt = 0;
    stepped = null;
    el.currentTime = clip.pieces[0].start_s;
    play();
  }

  function onkey(e: KeyboardEvent) {
    if ((e.target instanceof Element && e.target.closest("input, textarea")) || e.ctrlKey || e.metaKey || e.altKey)
      return;

    const actions: Record<string, () => void> = {
      " ": toggle,
      ArrowLeft: () => step(-1),
      ArrowRight: () => step(1),
      i: () => onstart(time),
      o: () => onend(time),
    };
    const action = actions[e.key.length === 1 ? e.key.toLowerCase() : e.key];
    if (!action) return;

    e.preventDefault();
    action();
  }

  const cycleRate = () => (rate = RATES[(RATES.indexOf(rate) + 1) % RATES.length]);

  function setVolume(e: Event) {
    volume = Number((e.currentTarget as HTMLInputElement).value) / 100;
    muted = false;
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="picture">
  <!-- svelte-ignore a11y_media_has_caption -->
  <video
    bind:this={el}
    {src}
    preload="auto"
    onplay={() => (playing = true)}
    onpause={() => {
      playing = false;
      keptAt = null;
    }}
  ></video>
</div>

<div class="controls">
  <button
    class="ctl"
    onmousedown={() => hold(-1)}
    onmouseup={release}
    onmouseleave={release}
    onclick={(e) => stepByKey(e, -1)}
    aria-label="Back one frame"
    title="Back one frame (Left, hold to repeat)"
  >
    <svg
      aria-hidden="true"
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.8"
      stroke-linecap="round"
      stroke-linejoin="round"><rect x="11" y="6" width="9" height="12" rx="1.5" /><path d="M8 9l-4 3 4 3" /></svg
    >
  </button>
  <button
    class="ctl"
    onclick={toggle}
    aria-label={playing ? "Pause" : "Play"}
    title="{playing ? 'Pause' : 'Play'} (Space)"
  >
    {#if playing}
      <svg aria-hidden="true" width="18" height="18" viewBox="0 0 24 24" fill="currentColor"
        ><rect x="6.5" y="5" width="3.5" height="14" rx="1" /><rect x="14" y="5" width="3.5" height="14" rx="1" /></svg
      >
    {:else}
      <svg aria-hidden="true" width="18" height="18" viewBox="0 0 24 24" fill="currentColor"
        ><path d="M7 5v14l11-7z" /></svg
      >
    {/if}
  </button>
  <button
    class="ctl"
    onmousedown={() => hold(1)}
    onmouseup={release}
    onmouseleave={release}
    onclick={(e) => stepByKey(e, 1)}
    aria-label="Forward one frame"
    title="Forward one frame (Right, hold to repeat)"
  >
    <svg
      aria-hidden="true"
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.8"
      stroke-linecap="round"
      stroke-linejoin="round"><rect x="4" y="6" width="9" height="12" rx="1.5" /><path d="M16 9l4 3-4 3" /></svg
    >
  </button>
  <span class="time" aria-label="Time"
    >{withFrames(time, fps)} <span class="total">/ {formatLength(duration)}</span></span
  >
  <span class="divider"></span>
  <button class="ctl" onclick={() => onstart(time)} aria-label="Start here" title="Start here (I)">
    <svg
      aria-hidden="true"
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"><path d="M9 4H6v16h3" /><path d="M13 12h7M17 9l3 3-3 3" /></svg
    >
  </button>
  <button class="ctl" onclick={() => onend(time)} aria-label="End here" title="End here (O)">
    <svg
      aria-hidden="true"
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"><path d="M15 4h3v16h-3" /><path d="M11 12H4M7 9l-3 3 3 3" /></svg
    >
  </button>
  <button class="ctl" onclick={playKept} aria-label="Play the kept part" title="Play the kept part">
    <svg
      aria-hidden="true"
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"><path d="M3 5v14M21 5v14" /><path d="M9 8v8l6-4z" fill="currentColor" /></svg
    >
  </button>
  <span class="gap"></span>
  {#if isZoomed(view, duration)}
    <button
      class="ctl wide"
      onclick={() => (view = wholeView(duration))}
      aria-label="Show the whole video"
      title="Show the whole video"
    >
      <svg
        aria-hidden="true"
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"><path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" /></svg
      >
      <span class="mono">{zoomText(zoomLevel(view, duration))}</span>
    </button>
  {/if}
  <button class="ctl wide mono" onclick={cycleRate} aria-label="Speed" title="Speed">{rate}×</button>
  <button
    class="ctl"
    onclick={() => (muted = !muted)}
    aria-label={silent ? "Unmute" : "Mute"}
    title={silent ? "Unmute" : "Mute"}
  >
    {#if silent}
      <svg
        aria-hidden="true"
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"><path d="M4 9v6h4l5 4V5L8 9z" /><path d="M17 9l5 6M22 9l-5 6" /></svg
      >
    {:else}
      <svg
        aria-hidden="true"
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"><path d="M4 9v6h4l5 4V5L8 9z" /><path d="M17 9a4 4 0 010 6" /></svg
      >
    {/if}
  </button>
  <input
    class="volume"
    type="range"
    min="0"
    max="100"
    value={silent ? 0 : volume * 100}
    oninput={setVolume}
    aria-label="Volume"
    title="Volume"
  />
</div>

<style>
  .picture {
    flex: 1;
    min-height: 0;
    display: grid;
    background: #000;
  }

  video {
    width: 100%;
    height: 100%;
    min-height: 0;
    object-fit: contain;
  }

  .controls {
    flex: none;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 6px 12px;
    border-top: 1px solid var(--line);
    background: var(--panel);
  }

  .ctl {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 8px;
    color: var(--dim);
  }

  .ctl:hover {
    background: var(--raised);
    color: var(--text);
    filter: none;
  }

  .wide {
    width: auto;
    padding: 0 10px;
    grid-auto-flow: column;
    gap: 6px;
    font-size: 12px;
  }

  .mono,
  .time {
    font-family: var(--mono);
  }

  .time {
    min-width: 130px;
    margin: 0 10px 0 8px;
    font-size: 13px;
  }

  .total {
    color: var(--dim);
  }

  .divider {
    width: 1px;
    height: 20px;
    margin: 0 6px;
    background: var(--line);
  }

  .gap {
    flex: 1;
  }

  .volume {
    width: 80px;
    margin: 0 6px 0 2px;
    accent-color: var(--green);
  }
</style>
