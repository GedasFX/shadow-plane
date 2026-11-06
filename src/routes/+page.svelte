<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { onMount } from "svelte";
  import { update } from "$lib/api";
  import Header, { type Screen } from "$lib/Header.svelte";
  import Library from "$lib/Library.svelte";
  import { follow } from "$lib/jobs.svelte";
  import { reviewNav } from "$lib/library";
  import Review from "$lib/Review.svelte";
  import { store } from "$lib/videos.svelte";

  let screen = $state<Screen>("library");
  let open = $state<string | null>(null);
  let version = $state("");
  let notice = $state<{ kind: "saved" | "discarded" | "error"; text: string } | null>(null);
  let noticeTimer: ReturnType<typeof setTimeout>;

  const video = $derived(store.videos.find((v) => v.key === open) ?? null);
  const nav = $derived(reviewNav(store.videos, open ?? ""));

  // With no key, nothing is left to review: the Library shows.
  function openVideo(key: string | null) {
    open = key;
    screen = key ? "review" : "library";
  }

  function notify(kind: "saved" | "discarded" | "error", text: string) {
    notice = { kind, text };
    clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => (notice = null), 2600);
  }

  onMount(() => {
    getVersion().then((v) => (version = v));
    update().catch(console.error);
    const following = follow();
    following.catch(console.error);

    return () => {
      following.then((stop) => stop()).catch(() => {});
    };
  });
</script>

<div class="shell">
  <Header {screen} {version} {video} {nav} onscreen={(s) => (screen = s)} onstep={openVideo} />
  <!-- Library stays mounted behind Review, so its filters and scan survive a tab switch. -->
  <div class="screen" hidden={screen !== "library"}>
    <Library onopen={openVideo} />
  </div>
  {#if screen === "review"}
    <Review {open} onopen={openVideo} onnotice={notify} />
  {/if}
  {#if notice}
    <div class="toast {notice.kind}" role="status">
      <svg
        aria-hidden="true"
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2.4"
        stroke-linecap="round"
        stroke-linejoin="round"
        ><path d={notice.kind === "saved" ? "M5 12.5l4.5 4.5L19 7" : "M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13"} /></svg
      >
      {notice.text}
    </div>
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

  .toast {
    position: fixed;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--raised);
    font-size: 13px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
  }

  .toast.saved svg {
    color: var(--green);
  }

  .toast.discarded svg,
  .toast.error {
    color: var(--red-text);
  }

  .screen[hidden] {
    display: none;
  }
</style>
