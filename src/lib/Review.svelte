<script lang="ts">
  import { type VideoDetail, video, videoUrl } from "./api";
  import { type Clip, freshClip, select, setEnd, setStart } from "./clip";
  import Player from "./Player.svelte";
  import Timeline from "./Timeline.svelte";
  import { type View, wholeView } from "./timeline";

  let { open }: { open: string | null } = $props();

  let detail = $state<VideoDetail | null>(null);
  let clip = $state<Clip>(freshClip(0));
  let view = $state<View>(wholeView(0));
  let time = $state(0);
  let player = $state<Player>();

  const saved = $derived(detail?.clips.flatMap((c) => c.pieces) ?? []);

  $effect(() => {
    const key = open;
    detail = null;
    if (!key) return;

    let current = true;
    video(key).then((found) => {
      if (!current || !found) return;

      detail = found;
      clip = freshClip(found.duration_s);
      view = wholeView(found.duration_s);
      time = 0;
    });

    return () => {
      current = false;
    };
  });
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
    <aside aria-label="Clip"></aside>
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

  aside {
    flex: none;
    width: 320px;
    border-left: 1px solid var(--line);
    background: var(--panel);
  }

  .empty {
    flex: 1;
    display: grid;
    place-items: center;
    margin: 0;
    color: var(--dim);
  }
</style>
