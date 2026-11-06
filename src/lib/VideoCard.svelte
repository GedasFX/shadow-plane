<script lang="ts">
  import { thumbUrl, type Video } from "./api";
  import { fileName, formatDate, formatLength, formatSize } from "./library";

  let { video, onopen }: { video: Video; onopen: (key: string) => void } = $props();
</script>

<button class="card" title={fileName(video.key)} onclick={() => onopen(video.key)}>
  <span class="shot" class:discarded={video.status === "discarded"}>
    <img src={thumbUrl(video.key)} alt="" loading="lazy" />
    <span class="length">{formatLength(video.duration_s)}</span>
    {#if video.status === "saved"}
      <span class="saved">
        <svg
          aria-hidden="true"
          width="10"
          height="10"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="3"
          stroke-linecap="round"
          stroke-linejoin="round"><path d="M5 12.5l4.5 4.5L19 7" /></svg
        >
        {video.saved_clips}
      </span>
    {:else if video.status === "discarded"}
      <span class="bin" role="img" aria-label="Discarded" title="Discarded">
        <svg
          aria-hidden="true"
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.4"
          stroke-linecap="round"
          stroke-linejoin="round"><path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13" /></svg
        >
      </span>
    {/if}
  </span>
  <span class="meta">
    <span class="when">{formatDate(video.date)}</span>
    <span class="size">{formatSize(video.size)}</span>
  </span>
</button>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    text-align: left;
  }

  .card:hover {
    filter: none;
  }

  .shot {
    position: relative;
    display: block;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--panel);
  }

  .card:hover .shot {
    box-shadow: 0 0 0 2px var(--green);
  }

  .discarded img {
    opacity: 0.45;
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .length {
    position: absolute;
    right: 6px;
    bottom: 6px;
    padding: 1px 5px;
    border-radius: 3px;
    background: rgba(0, 0, 0, 0.7);
    font-family: var(--mono);
    font-size: 11px;
  }

  .saved {
    position: absolute;
    left: 6px;
    top: 6px;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 7px;
    border-radius: 999px;
    background: var(--green-dark);
    color: var(--green-light);
    font-size: 11px;
    font-weight: 600;
  }

  .bin {
    position: absolute;
    left: 6px;
    top: 6px;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--red-dark);
    color: var(--red-text);
  }

  .meta {
    display: flex;
    justify-content: space-between;
    padding: 0 2px;
    font-size: 12px;
  }

  .when {
    font-family: var(--mono);
  }

  .size {
    color: var(--dim);
  }
</style>
