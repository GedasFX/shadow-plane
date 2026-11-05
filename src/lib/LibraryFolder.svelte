<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import GameList from "./GameList.svelte";

  type Scan = { sources: number; unreadable: number };

  let folder = $state<string | null>(null);
  let scan = $state<Promise<Scan>>();

  function show(path: string | null) {
    folder = path;
    if (path) {
      scan = invoke<Scan>("scan");
    }
  }

  invoke<string | null>("library").then(show);

  async function choose() {
    const picked = await invoke<string | null>("choose_library");
    if (picked) {
      show(picked);
    }
  }
</script>

<section>
  <p>{folder ? `Library: ${folder}` : "No library folder chosen yet"}</p>
  <button onclick={choose}>{folder ? "Change folder" : "Choose folder"}</button>
  {#if scan}
    {#await scan}
      <p>Scanning…</p>
    {:then found}
      <p>Videos: {found.sources}{found.unreadable ? ` · Unreadable: ${found.unreadable}` : ""}</p>
      <GameList />
    {:catch error}
      <p>Couldn't scan: {error}</p>
    {/await}
  {/if}
</section>
