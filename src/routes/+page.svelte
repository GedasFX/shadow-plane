<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { onMount } from "svelte";
  import { update } from "$lib/api";
  import Header, { type Screen } from "$lib/Header.svelte";
  import Library from "$lib/Library.svelte";
  import Review from "$lib/Review.svelte";

  let screen = $state<Screen>("library");
  let open = $state<string | null>(null);
  let version = $state("");

  onMount(() => {
    getVersion().then((v) => (version = v));
    update().catch(console.error);
  });
</script>

<div class="shell">
  <Header {screen} {version} onscreen={(s) => (screen = s)} />
  <!-- Library stays mounted behind Review, so its filters and scan survive a tab switch. -->
  <div class="screen" hidden={screen !== "library"}>
    <Library
      onopen={(key) => {
        open = key;
        screen = "review";
      }}
    />
  </div>
  {#if screen === "review"}
    <Review {open} />
  {/if}
</div>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .screen {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .screen[hidden] {
    display: none;
  }
</style>
