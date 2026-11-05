<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let folder = $state<string | null>(null);
  invoke<string | null>("library").then((saved) => {
    folder = saved;
  });

  async function choose() {
    const picked = await invoke<string | null>("choose_library");
    if (picked) folder = picked;
  }
</script>

<section>
  <p>{folder ? `Library: ${folder}` : "No library folder chosen yet"}</p>
  <button onclick={choose}>{folder ? "Change folder" : "Choose folder"}</button>
</section>
