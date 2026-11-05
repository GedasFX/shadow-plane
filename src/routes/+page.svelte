<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { invoke } from "@tauri-apps/api/core";
  import LibraryFolder from "$lib/LibraryFolder.svelte";

  const version = getVersion();
  const update = invoke("update");
</script>

<main>
  <h1>Shadow Plane</h1>
  {#await version then v}
    <p>Version {v}</p>
  {/await}
  {#await update}
    <p>Checking for updates…</p>
  {:then}
    <p>Up to date</p>
  {:catch error}
    <p>Couldn't check for updates: {error}</p>
  {/await}
  <LibraryFolder />
</main>
