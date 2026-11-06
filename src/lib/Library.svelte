<script lang="ts">
  import { onMount } from "svelte";
  import { chooseLibrary, library, scan } from "./api";
  import GamesFilter from "./GamesFilter.svelte";
  import { type Filter, filters, gameCounts, groupByGame } from "./library";
  import VideoCard from "./VideoCard.svelte";
  import { refresh, store } from "./videos.svelte";

  let { onopen }: { onopen: (key: string) => void } = $props();

  // undefined until the saved folder is read; null when none is chosen.
  let folder = $state<string | null>();
  let scanning = $state(false);
  let error = $state<string>();
  let filter = $state<Filter>("review");
  let search = $state("");
  let games = $state<string[]>([]);

  const groups = $derived(groupByGame(store.videos, { filter, search, games }));

  async function rescan() {
    scanning = true;
    error = undefined;
    try {
      await scan();
      await refresh();
    } catch (e) {
      error = `Couldn't scan: ${e}`;
    } finally {
      scanning = false;
    }
  }

  async function choose() {
    const picked = await chooseLibrary();
    if (!picked) return;
    folder = picked;
    games = [];
    await rescan();
  }

  onMount(async () => {
    folder = await library();
    if (folder) await rescan();
  });
</script>

{#if folder === null}
  <main class="empty">
    <button class="choose" onclick={choose}>Choose folder</button>
  </main>
{:else if folder}
  <main>
    <div class="body">
      <div class="toolbar">
        <label class="search">
          <svg
            aria-hidden="true"
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="var(--dim)"
            stroke-width="2"
            stroke-linecap="round"><circle cx="11" cy="11" r="7" /><path d="M20 20l-3.5-3.5" /></svg
          >
          <input aria-label="Find a video" placeholder="Find a video" bind:value={search} />
        </label>
        <div class="filters" role="group" aria-label="Show">
          {#each filters as f (f.id)}
            <button aria-pressed={filter === f.id} onclick={() => (filter = f.id)}>{f.name}</button>
          {/each}
        </div>
        <GamesFilter games={gameCounts(store.videos, filter, search)} bind:selected={games} />
        <button class="icon-button" title={folder} aria-label="Library folder" onclick={choose}>
          <svg
            aria-hidden="true"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            ><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg
          >
        </button>
        <button
          class="icon-button"
          title="Rescan"
          aria-label="Rescan"
          aria-busy={scanning}
          disabled={scanning}
          onclick={rescan}
        >
          <svg
            class:spin={scanning}
            aria-hidden="true"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"><path d="M20 12a8 8 0 1 1-2.5-5.8M20 4v5h-5" /></svg
          >
        </button>
      </div>
      {#if error}
        <p class="note error">{error}</p>
      {/if}
      {#each groups as group (group.name)}
        <h2>{group.name} <span>{group.videos.length}</span></h2>
        <div class="grid">
          {#each group.videos as video (video.key)}
            <VideoCard {video} {onopen} />
          {/each}
        </div>
      {:else}
        {#if !scanning && !error}
          <p class="note">Nothing here</p>
        {/if}
      {/each}
    </div>
  </main>
{/if}

<style>
  main {
    flex: 1;
    min-width: 0;
    display: flex;
    justify-content: center;
    padding: 20px 24px 40px;
    overflow: auto;
  }

  .empty {
    align-items: center;
  }

  .choose {
    padding: 9px 16px;
    border-radius: 8px;
    background: var(--raised);
    font-size: 14px;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
    max-width: 1240px;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    margin-bottom: 6px;
  }

  .search {
    flex: 1;
    min-width: 240px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 12px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
  }

  .search input {
    flex: 1;
    min-width: 0;
    font-size: 14px;
  }

  .filters {
    display: flex;
    padding: 3px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
  }

  .filters button {
    padding: 6px 12px;
    border-radius: 6px;
    color: var(--dim);
    font-size: 13px;
  }

  .filters button[aria-pressed="true"] {
    background: var(--raised-2);
    color: var(--text);
    font-weight: 600;
  }

  .icon-button:disabled {
    cursor: default;
    filter: none;
  }

  .spin {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  h2 {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin: 0;
    padding: 16px 2px 8px;
    font-size: 15px;
    font-weight: 600;
  }

  h2 span {
    color: var(--dim);
    font-size: 12px;
    font-weight: 400;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 16px;
  }

  .note {
    color: var(--dim);
  }

  .error {
    color: var(--red-text);
  }
</style>
