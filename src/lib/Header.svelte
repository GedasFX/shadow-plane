<script lang="ts" module>
  export type Screen = "library" | "review";
</script>

<script lang="ts">
  import type { Video } from "./api";
  import { formatDate, formatLength, formatSize, type reviewNav } from "./library";

  type Props = {
    screen: Screen;
    version: string;
    // The open video, shown on Review.
    video: Video | null;
    nav: ReturnType<typeof reviewNav>;
    onscreen: (screen: Screen) => void;
    onstep: (key: string) => void;
  };

  let { screen, version, video, nav, onscreen, onstep }: Props = $props();

  const tabs: { id: Screen; name: string }[] = [
    { id: "library", name: "Library" },
    { id: "review", name: "Review" },
  ];
</script>

<header>
  <svg role="img" aria-label="Shadow Plane" width="28" height="28" viewBox="0 0 32 32" style="flex: none">
    <title>{version ? `Shadow Plane ${version}` : "Shadow Plane"}</title>
    <path
      d="M6 27C5 18 8 6 16 6s11 12 10 21c-2-2-3 1-5-1s-3 1-5-1-3 1-5-1-3 1-5 3z"
      fill="var(--bg)"
      stroke="var(--green)"
      stroke-width="1.8"
      stroke-linejoin="round"
    />
    <ellipse cx="12.5" cy="15" rx="1.6" ry="2.2" fill="var(--green)" />
    <ellipse cx="19.5" cy="15" rx="1.6" ry="2.2" fill="var(--green)" />
  </svg>
  <nav aria-label="Screens">
    {#each tabs as tab (tab.id)}
      <button aria-current={tab.id === screen ? "page" : "false"} onclick={() => onscreen(tab.id)}>{tab.name}</button>
    {/each}
  </nav>
  {#if screen === "review" && video}
    <div class="steps">
      <button
        class="step"
        aria-label="Previous video"
        title="Previous video"
        disabled={!nav.prev}
        onclick={() => nav.prev && onstep(nav.prev)}
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
          stroke-linejoin="round"><path d="M15 6l-6 6 6 6" /></svg
        >
      </button>
      <button
        class="step"
        aria-label="Next video"
        title="Next video"
        disabled={!nav.next}
        onclick={() => nav.next && onstep(nav.next)}
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
          stroke-linejoin="round"><path d="M9 6l6 6-6 6" /></svg
        >
      </button>
    </div>
    <span class="game">{video.game}</span>
    <span class="meta">{formatDate(video.date)} · {formatLength(video.duration_s)} · {formatSize(video.size)}</span>
    <span class="grow"></span>
    <span class="queue">{nav.index ? `${nav.index} of ${nav.total}` : nav.total} to review</span>
  {/if}
</header>

<style>
  header {
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    height: 48px;
    padding: 0 14px;
    border-bottom: 1px solid var(--line);
    background: var(--panel);
  }

  nav {
    display: flex;
    padding: 2px;
    border-radius: 7px;
    background: var(--bg);
  }

  nav button {
    padding: 5px 12px;
    border-radius: 5px;
    color: var(--dim);
    font-size: 13px;
  }

  .steps {
    display: flex;
    gap: 2px;
    margin-left: 6px;
  }

  .step {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 6px;
  }

  .step:disabled {
    opacity: 0.35;
    cursor: default;
    filter: none;
  }

  .game {
    font-size: 15px;
    font-weight: 600;
    white-space: nowrap;
  }

  .meta {
    overflow: hidden;
    color: var(--dim);
    font-family: var(--mono);
    font-size: 13px;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .grow {
    flex: 1;
  }

  .queue {
    color: var(--dim);
    font-size: 13px;
    white-space: nowrap;
  }

  nav button[aria-current="page"] {
    background: var(--raised-2);
    color: var(--text);
    font-weight: 600;
  }
</style>
