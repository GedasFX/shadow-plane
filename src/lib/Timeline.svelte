<script lang="ts">
  import type { Piece } from "./api";
  import type { Clip } from "./clip";
  import { formatLength } from "./library";
  import {
    fracOf,
    frameOf,
    panTo,
    pan,
    ticks,
    tickStep,
    timeAt,
    type View,
    withFrames,
    zoomAround,
    zoomLevel,
  } from "./timeline";

  type Drag = { kind: "scrub" } | { kind: "pan" } | { kind: "start" | "end"; i: number };

  type Props = {
    duration: number;
    fps: number;
    clip: Clip;
    // Pieces of the video's already saved clips, drawn hatched.
    saved: Piece[];
    view: View;
    time: number;
    onseek: (t: number) => void;
    onselect: (i: number) => void;
    onedge: (i: number, edge: "start" | "end", t: number) => void;
  };

  let { duration, fps, clip, saved, view = $bindable(), time, onseek, onselect, onedge }: Props = $props();

  let track: HTMLElement;
  let overview: HTMLElement;
  let drag = $state<Drag | null>(null);

  const pct = (t: number) => fracOf(view, t) * 100;
  const overPct = (t: number) => (t / duration) * 100;
  const zoomed = $derived(zoomLevel(view, duration) > 1.01);
  const marks = $derived(ticks(view));
  const labelFrames = $derived(tickStep(view) < 1);
  const bubbleAt = $derived.by(() => {
    if (drag?.kind === "start" || drag?.kind === "end") return clip.pieces[drag.i][`${drag.kind}_s`];

    return time;
  });
  const bubbled = $derived(drag !== null && drag.kind !== "pan");

  // Where the pointer is across an element, 0 to 1.
  function across(e: MouseEvent, el: HTMLElement) {
    const rect = el.getBoundingClientRect();

    return rect.width ? Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width)) : 0;
  }

  // Edges sit on whole frames.
  const snap = (t: number) => frameOf(t, fps) / fps;

  function scrubStart(e: MouseEvent) {
    drag = { kind: "scrub" };
    onseek(timeAt(view, across(e, track)));
  }

  function grab(e: MouseEvent, i: number, kind: "start" | "end") {
    e.preventDefault();
    e.stopPropagation();
    drag = { kind, i };
    onselect(i);
  }

  function panStart(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    drag = { kind: "pan" };
  }

  function move(e: MouseEvent) {
    if (!drag) return;

    if (drag.kind === "pan") {
      view = panTo(view, duration, across(e, overview) * duration);
    } else if (drag.kind === "scrub") {
      onseek(timeAt(view, across(e, track)));
    } else {
      onedge(drag.i, drag.kind, snap(timeAt(view, across(e, track))));
    }
  }

  // Ctrl + scroll zooms around the mouse, scroll pans once zoomed. Not passive, or the page zooms instead.
  function wheel(node: HTMLElement) {
    const onwheel = (e: WheelEvent) => {
      const frac = across(e, track);
      if (e.ctrlKey) {
        e.preventDefault();
        view = zoomAround(view, duration, frac, e.deltaY < 0 ? 0.8 : 1.25);
      } else if (zoomed) {
        e.preventDefault();
        view = pan(view, duration, (e.deltaX || e.deltaY) / (track.getBoundingClientRect().width || 1));
      }
    };
    node.addEventListener("wheel", onwheel, { passive: false });

    return { destroy: () => node.removeEventListener("wheel", onwheel) };
  }
</script>

<svelte:window onmousemove={move} onmouseup={() => (drag = null)} />

<section class="timeline" aria-label="Timeline">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="wrap" use:wheel onmousedown={scrubStart}>
    <div class="ruler">
      {#each marks as t (t)}
        <span class="mark" style:left="{pct(t)}%" class:first={t === view.start}
          >{labelFrames ? withFrames(t, fps) : formatLength(t)}</span
        >
      {/each}
    </div>
    <div class="track" bind:this={track}>
      {#each saved as p}
        <div
          class="saved"
          style:left="{pct(p.start_s)}%"
          style:width="{((p.end_s - p.start_s) / (view.end - view.start)) * 100}%"
        ></div>
      {/each}
      {#each clip.pieces as p, i}
        <div
          class="piece"
          class:selected={i === clip.selected}
          style:left="{pct(p.start_s)}%"
          style:width="{((p.end_s - p.start_s) / (view.end - view.start)) * 100}%"
        >
          <span class="grip start" role="presentation" title="Drag the start" onmousedown={(e) => grab(e, i, "start")}
          ></span>
          <span class="number">{clip.pieces.length > 1 ? i + 1 : ""}</span>
          <span class="grip end" role="presentation" title="Drag the end" onmousedown={(e) => grab(e, i, "end")}></span>
        </div>
      {/each}
    </div>
    {#if time >= view.start && time <= view.end}
      <div class="head" style:left="{pct(time)}%"><span class="flag"></span></div>
    {/if}
    {#if bubbled}
      <span class="bubble" style:left="{Math.max(0, Math.min(100, pct(bubbleAt)))}%">{withFrames(bubbleAt, fps)}</span>
    {/if}
    <div class="overview" class:hidden={!zoomed} bind:this={overview}>
      {#each clip.pieces as p}
        <span class="over-piece" style:left="{overPct(p.start_s)}%" style:width="{overPct(p.end_s - p.start_s)}%"
        ></span>
      {/each}
      <span
        class="window"
        role="presentation"
        title="Drag to pan"
        style:left="{overPct(view.start)}%"
        style:width="{overPct(view.end - view.start)}%"
        onmousedown={panStart}
      ></span>
    </div>
  </div>
</section>

<style>
  .timeline {
    flex: none;
    padding: 0 16px 12px;
    background: var(--panel);
  }

  .wrap {
    position: relative;
    padding-top: 24px;
    cursor: pointer;
  }

  .ruler {
    position: absolute;
    inset: 4px 0 auto;
    height: 16px;
    overflow: hidden;
    font-family: var(--mono);
    font-size: 11px;
    color: var(--dim);
    pointer-events: none;
  }

  .mark {
    position: absolute;
    top: 0;
    transform: translateX(-50%);
  }

  .mark.first {
    transform: none;
  }

  .track {
    position: relative;
    height: 30px;
    overflow: hidden;
    border: 1px solid var(--line);
    border-radius: 5px;
    background: var(--bg);
  }

  .saved {
    position: absolute;
    inset: 2px auto 2px;
    border-radius: 3px;
    background: repeating-linear-gradient(135deg, var(--faint) 0 2px, transparent 2px 6px);
    opacity: 0.6;
    pointer-events: none;
  }

  .piece {
    position: absolute;
    inset: 2px auto 2px;
    box-sizing: border-box;
    display: flex;
    min-width: 18px;
    border: 1.5px solid var(--green-line);
    border-radius: 4px;
    background: var(--green-dark);
  }

  .piece.selected {
    border-color: var(--green);
  }

  .number {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 700;
    pointer-events: none;
  }

  .grip {
    width: 8px;
    background: var(--green);
    cursor: ew-resize;
  }

  .grip.start {
    border-radius: 3px 0 0 3px;
  }

  .grip.end {
    border-radius: 0 3px 3px 0;
  }

  .head {
    position: absolute;
    top: 14px;
    height: 44px;
    width: 2px;
    margin-left: -1px;
    background: #fff;
    pointer-events: none;
  }

  .flag {
    position: absolute;
    top: 0;
    left: -6px;
    width: 14px;
    height: 10px;
    background: #fff;
    clip-path: polygon(0 0, 100% 0, 50% 100%);
  }

  .bubble {
    position: absolute;
    top: -6px;
    transform: translate(-50%, -100%);
    padding: 3px 6px;
    border-radius: 4px;
    background: var(--text);
    color: var(--bg);
    font-family: var(--mono);
    font-size: 11px;
    white-space: nowrap;
    pointer-events: none;
  }

  .overview {
    position: relative;
    height: 8px;
    margin-top: 6px;
    border-radius: 4px;
    background: var(--bg);
  }

  .overview.hidden {
    opacity: 0;
    pointer-events: none;
  }

  .over-piece {
    position: absolute;
    inset: 2px auto 2px;
    border-radius: 2px;
    background: var(--green-line);
  }

  .window {
    position: absolute;
    inset: -2px auto -2px;
    box-sizing: border-box;
    border: 1.5px solid var(--text);
    border-radius: 4px;
    cursor: grab;
  }
</style>
