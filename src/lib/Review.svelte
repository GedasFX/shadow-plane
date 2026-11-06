<script lang="ts">
  import { clipFileName, discard, save, type VideoDetail, video, videoUrl } from "./api";
  import { type Clip, clipInput, freshClip, freshMeta, type Meta, select, setEnd, setStart } from "./clip";
  import ClipColumn from "./ClipColumn.svelte";
  import { formatDate, reviewNav } from "./library";
  import Player from "./Player.svelte";
  import Timeline from "./Timeline.svelte";
  import { type View, wholeView } from "./timeline";
  import { refresh, store } from "./videos.svelte";

  type Props = {
    open: string | null;
    // A key opens that video; null means none is left to review.
    onopen: (key: string | null) => void;
    onnotice: (kind: "saved" | "discarded" | "error", text: string) => void;
  };

  let { open, onopen, onnotice }: Props = $props();

  let detail = $state<VideoDetail | null>(null);
  let clip = $state<Clip>(freshClip(0));
  let view = $state<View>(wholeView(0));
  let time = $state(0);
  let player = $state<Player>();
  let meta = $state<Meta>(freshMeta(""));
  let fileName = $state("");
  let tried = $state(false);

  const saved = $derived(detail?.clips.flatMap((c) => c.pieces) ?? []);

  const missing = $derived(tried && !meta.title.trim());

  // A whole-video clip with nothing filled in: a video just opened, or one saved and cut again.
  function start(found: VideoDetail) {
    detail = found;
    clip = freshClip(found.duration_s);
    meta = freshMeta(found.game);
    fileName = "";
    tried = false;
  }

  $effect(() => {
    const key = open;
    detail = null;
    if (!key) return;

    let current = true;
    video(key).then((found) => {
      if (!current || !found) return;

      start(found);
      view = wholeView(found.duration_s);
      time = 0;
    });

    return () => {
      current = false;
    };
  });

  // The name follows the fields that make it up, once typing pauses.
  $effect(() => {
    if (!detail) return;

    const key = detail.key;
    const { title, game, great, best } = meta;
    const { start_s, end_s } = clip.pieces[0];
    const timer = setTimeout(() => {
      clipFileName(key, { title, game, great, best, pieces: [{ start_s, end_s }] })
        .then((name) => (fileName = name))
        .catch(() => (fileName = ""));
    }, 250);

    return () => clearTimeout(timer);
  });

  async function keep(another: boolean) {
    if (!detail) return;

    tried = true;
    if (!meta.title.trim()) return;

    const { key } = detail;
    const title = meta.title.trim();
    try {
      await save(key, clipInput(meta, clip));
      await refresh();
    } catch (e) {
      onnotice("error", `Couldn't save: ${e}`);

      return;
    }

    onnotice("saved", `Saved “${title}”`);
    if (!another) {
      onopen(reviewNav(store.videos, key).next);

      return;
    }

    // The saved range now shows hatched on the timeline.
    const found = await video(key);
    if (found) start(found);
  }

  async function bin() {
    if (!detail) return;

    const { key, date } = detail;
    try {
      await discard(key);
      await refresh();
    } catch (e) {
      onnotice("error", `Couldn't discard: ${e}`);

      return;
    }

    onnotice("discarded", `Discarded ${formatDate(date)}`);
    onopen(reviewNav(store.videos, key).next);
  }
</script>

<main>
  {#if open && detail}
    <div class="left">
      <Player
        bind:this={player}
        src={videoUrl(detail.key)}
        fps={detail.fps}
        duration={detail.duration_s}
        {clip}
        bind:view
        bind:time
        onstart={(t) => (clip = setStart(clip, clip.selected, t))}
        onend={(t) => (clip = setEnd(clip, clip.selected, t, detail!.duration_s))}
      />
      <Timeline
        duration={detail.duration_s}
        fps={detail.fps}
        {clip}
        {saved}
        bind:view
        {time}
        onseek={(t) => player?.seek(t)}
        onselect={(i) => (clip = select(clip, i))}
        onedge={(i, edge, t) =>
          (clip = edge === "start" ? setStart(clip, i, t) : setEnd(clip, i, t, detail!.duration_s))}
      />
    </div>
    <ClipColumn bind:clip bind:meta {fileName} {missing} onsave={keep} ondiscard={bin} />
  {:else}
    <p class="empty">{open ? "" : "No video open"}</p>
  {/if}
</main>

<style>
  main {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .left {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .empty {
    flex: 1;
    display: grid;
    place-items: center;
    margin: 0;
    color: var(--dim);
  }
</style>
