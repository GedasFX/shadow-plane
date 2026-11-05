<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";

  const version = getVersion();
</script>

<main>
  <h1>Shadow Plane</h1>
  {#await version then v}
    <p>Version {v}</p>
  {/await}
</main>
