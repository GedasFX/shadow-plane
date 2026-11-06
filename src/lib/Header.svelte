<script lang="ts" module>
  export type Screen = "library" | "review";
</script>

<script lang="ts">
  let { screen, version, onscreen }: { screen: Screen; version: string; onscreen: (screen: Screen) => void } = $props();

  const tabs: { id: Screen; name: string }[] = [
    { id: "library", name: "Library" },
    { id: "review", name: "Review" },
  ];
</script>

<header>
  <svg role="img" aria-label="Shadow Plane" width="28" height="28" viewBox="0 0 32 32" style="flex: none">
    <title>{version ? `Shadow Plane ${version}` : "Shadow Plane"}</title>
    <path
      d="M6 27C5 18 8 6 16 6s11 12 10 21c-2-2-3 1-5-1s-3 1-5-1-3 1-5-1-3 1-5 3z"
      fill="var(--bg)"
      stroke="var(--green)"
      stroke-width="1.8"
      stroke-linejoin="round"
    />
    <ellipse cx="12.5" cy="15" rx="1.6" ry="2.2" fill="var(--green)" />
    <ellipse cx="19.5" cy="15" rx="1.6" ry="2.2" fill="var(--green)" />
  </svg>
  <nav aria-label="Screens">
    {#each tabs as tab (tab.id)}
      <button aria-current={tab.id === screen ? "page" : "false"} onclick={() => onscreen(tab.id)}>{tab.name}</button>
    {/each}
  </nav>
</header>

<style>
  header {
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    height: 48px;
    padding: 0 14px;
    border-bottom: 1px solid var(--line);
    background: var(--panel);
  }

  nav {
    display: flex;
    padding: 2px;
    border-radius: 7px;
    background: var(--bg);
  }

  nav button {
    padding: 5px 12px;
    border-radius: 5px;
    color: var(--dim);
    font-size: 13px;
  }

  nav button[aria-current="page"] {
    background: var(--raised-2);
    color: var(--text);
    font-weight: 600;
  }
</style>
