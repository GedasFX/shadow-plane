<script lang="ts">
  let { open }: { open: string | null } = $props();
</script>

<main>
  {#if open}
    <p>{open}</p>
  {:else}
    <p>No video open</p>
  {/if}
</main>

<style>
  main {
    flex: 1;
    display: grid;
    place-items: center;
    color: var(--dim);
  }
</style>
