<script lang="ts">
  import { gamesLabel } from "./library";

  let { games, selected = $bindable() }: { games: { name: string; count: number }[]; selected: string[] } = $props();

  let open = $state(false);
  let search = $state("");
  let root = $state<HTMLElement>();

  const shown = $derived(games.filter((g) => g.name.toLowerCase().includes(search.trim().toLowerCase())));

  function toggle(name: string) {
    selected = selected.includes(name) ? selected.filter((g) => g !== name) : [...selected, name];
  }

  function close() {
    open = false;
    search = "";
  }
</script>

<svelte:window
  onclick={(e) => {
    if (open && !root?.contains(e.target as Node)) close();
  }}
  onkeydown={(e) => {
    if (open && e.key === "Escape") close();
  }}
/>

{#snippet box(on: boolean, some: boolean)}
  <span class="box" class:filled={on || some}>
    {#if on}
      <svg
        aria-hidden="true"
        width="11"
        height="11"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="3.4"
        stroke-linecap="round"
        stroke-linejoin="round"><path d="M5 12.5l4.5 4.5L19 7" /></svg
      >
    {:else if some}
      <span class="dash"></span>
    {/if}
  </span>
{/snippet}

<div class="games" bind:this={root}>
  <button
    class="trigger"
    class:active={selected.length > 0}
    aria-haspopup="listbox"
    aria-expanded={open}
    onclick={() => (open ? close() : (open = true))}
  >
    {gamesLabel(selected)}
    <svg
      aria-hidden="true"
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="var(--dim)"
      stroke-width="2.2"
      stroke-linecap="round"
      stroke-linejoin="round"><path d="M6 9l6 6 6-6" /></svg
    >
  </button>
  {#if open}
    <div class="popover" role="listbox" aria-multiselectable="true" aria-label="Games">
      <label class="find">
        <svg
          aria-hidden="true"
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="var(--dim)"
          stroke-width="2"
          stroke-linecap="round"><circle cx="11" cy="11" r="7" /><path d="M20 20l-3.5-3.5" /></svg
        >
        <input aria-label="Find a game" placeholder="Find a game" bind:value={search} />
      </label>
      <button class="option all" role="option" aria-selected={selected.length === 0} onclick={() => (selected = [])}>
        {@render box(selected.length === 0, selected.length > 0)}
        <span class="name">All games</span>
      </button>
      {#each shown as game (game.name)}
        <button
          class="option"
          role="option"
          aria-selected={selected.includes(game.name)}
          onclick={() => toggle(game.name)}
        >
          {@render box(selected.includes(game.name), false)}
          <span class="name">{game.name}</span>
          <span class="count">{game.count}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .games {
    position: relative;
  }

  .trigger {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 12px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    font-size: 13px;
  }

  .trigger.active {
    border-color: var(--green-line);
    background: var(--green-dark);
    color: var(--green-light);
  }

  .popover {
    position: absolute;
    right: 0;
    top: 46px;
    z-index: 20;
    box-sizing: border-box;
    width: 300px;
    padding: 6px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--raised);
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6);
  }

  .find {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 2px 2px 6px;
    padding: 7px 10px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--bg);
  }

  .find input {
    flex: 1;
    min-width: 0;
    font-size: 13px;
  }

  .option {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    border-radius: 6px;
    font-size: 13px;
  }

  .option:hover {
    background: var(--raised-2);
    filter: none;
  }

  .all {
    margin-bottom: 4px;
    border-bottom: 1px solid var(--line);
    border-radius: 6px 6px 0 0;
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    color: var(--dim);
    font-family: var(--mono);
    font-size: 12px;
  }

  .box {
    flex: none;
    box-sizing: border-box;
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border: 1.5px solid var(--faint);
    border-radius: 4px;
    color: var(--bg);
  }

  .box.filled {
    border-color: var(--green);
    background: var(--green);
  }

  .dash {
    width: 8px;
    height: 2px;
    border-radius: 1px;
    background: var(--bg);
  }
</style>
