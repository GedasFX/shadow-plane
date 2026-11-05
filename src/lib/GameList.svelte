<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  type Source = { path: string; name: string; duration_s: number; height: number; decision: string | null };
  type Game = { name: string; sources: Source[] };

  const decisions: Record<string, string> = { queue: "Queued", skip: "Skipped", reject: "Rejected" };

  const games = invoke<Game[]>("games");

  function length(seconds: number) {
    const s = Math.floor(seconds);
    const mmss = `${Math.floor((s % 3600) / 60)}:${String(s % 60).padStart(2, "0")}`;

    return s < 3600 ? mmss : `${Math.floor(s / 3600)}:${mmss.padStart(5, "0")}`;
  }

  function describe(source: Source) {
    const parts = [source.name, length(source.duration_s), `${source.height}p`];
    if (source.decision) {
      parts.push(decisions[source.decision]);
    }

    return parts.join(" · ");
  }
</script>

{#await games then games}
  {#each games as game (game.name)}
    <section>
      <h2>{game.name || "Outside game folders"}</h2>
      <ul>
        {#each game.sources as source (source.path)}
          <li>{describe(source)}</li>
        {/each}
      </ul>
    </section>
  {/each}
{:catch error}
  <p>Couldn't list the videos: {error}</p>
{/await}
