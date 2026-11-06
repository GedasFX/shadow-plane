<script lang="ts">
  import { cancelJob, type Job, pause, play, removeJob, reorderJobs, resume, thumbUrl } from "./api";
  import { chipText, failures, newestFailure, nowText, percent, phase, reordered, timeLeft, waiting } from "./jobs";
  import { store } from "./jobs.svelte";
  import { formatLength } from "./library";

  const RING = 2 * Math.PI * 9;
  const ICONS = {
    encode: "M3 5h18v14H3zM3 9h18M3 15h18M8 5v4M16 5v4M8 15v4M16 15v4",
    check: "M12 3l8 3v6c0 4.5-3.2 8-8 9-4.8-1-8-4.5-8-9V6zM8.5 12l2.5 2.5 4.5-5",
    paused: "M9 5v14M15 5v14",
    saved: "M5 12.5l4.5 4.5L19 7",
    idle: "M6 12h12",
  };

  let open = $state(false);
  let dragging = $state(-1);
  let error = $state("");
  let root: HTMLElement;

  const q = $derived(store.queue);
  const now = $derived(phase(q, store.progress));
  const pct = $derived(percent(q, store.progress));
  const left = $derived(timeLeft(q, store.progress));
  const failed = $derived(failures(q.done, store.seen));
  const text = $derived(chipText(q, store.progress, failed));
  const running = $derived(now === "encode" || now === "check");
  const title = (job: Job) => job.title || job.file_name;

  function toggle() {
    open = !open;
    error = "";
    if (open) store.seen = newestFailure(q.done);
  }

  async function run(action: () => Promise<void>) {
    error = "";
    try {
      await action();
    } catch (e) {
      error = String(e);
    }
  }

  function away(e: Event) {
    if (open && e.target instanceof Node && !root.contains(e.target)) open = false;
  }

  function drop(to: number) {
    if (dragging >= 0 && dragging !== to) run(() => reorderJobs(reordered(q.pending, dragging, to)));
    dragging = -1;
  }
</script>

<svelte:window onpointerdown={away} onkeydown={(e) => e.key === "Escape" && (open = false)} />

<div class="jobs" bind:this={root}>
  <button class="chip" class:open aria-label={text.label} title={text.title} aria-expanded={open} onclick={toggle}>
    <svg aria-hidden="true" width="22" height="22" viewBox="0 0 22 22" class="ring">
      <circle cx="11" cy="11" r="9" fill="none" stroke="var(--line)" stroke-width="2.5" />
      <circle
        cx="11"
        cy="11"
        r="9"
        fill="none"
        stroke="var(--green)"
        stroke-width="2.5"
        stroke-linecap="round"
        stroke-dasharray="{(RING * (now === 'saved' ? 100 : (pct ?? 0))) / 100} {RING}"
      />
    </svg>
    <svg
      aria-hidden="true"
      width="22"
      height="22"
      viewBox="0 0 24 24"
      fill="none"
      stroke={now === "saved" ? "var(--green)" : "var(--dim)"}
      stroke-width="1.8"
      stroke-linecap="round"
      stroke-linejoin="round"
      style="flex: none"><path d={ICONS[now]} /></svg
    >
    <span class="num">{pct === null ? "" : `${pct}%`}</span>
    <span class="num dim">{now === "paused" ? "" : (left ?? "")}</span>
    <span class="badges">
      {#if waiting(q)}<span class="badge">+{waiting(q)}</span>{/if}
      {#if failed}<span class="badge fail">{failed}</span>{/if}
    </span>
  </button>

  {#if open}
    <div class="pop" role="dialog" aria-label="Jobs">
      {#if q.running || q.paused}
        <div class="now">
          <div class="head">
            {#if q.running}
              <img src={thumbUrl(q.running.key)} alt="" />
              <span class="what">
                <span class="name">{title(q.running)}</span>
                <span class="sub">{nowText(q, store.progress)}</span>
              </span>
            {:else}
              <span class="what"><span class="name">Paused</span></span>
            {/if}
            {#if q.paused}
              <button class="icon" aria-label="Resume" title="Resume" onclick={() => run(resume)}>
                <svg aria-hidden="true" width="14" height="14" viewBox="0 0 24 24" fill="currentColor"
                  ><path d="M7 4.5v15l12-7.5z" /></svg
                >
              </button>
            {:else}
              <button class="icon" aria-label="Pause" title="Pause" onclick={() => run(pause)}>
                <svg aria-hidden="true" width="14" height="14" viewBox="0 0 24 24" fill="currentColor"
                  ><rect x="6" y="5" width="4" height="14" rx="1" /><rect
                    x="14"
                    y="5"
                    width="4"
                    height="14"
                    rx="1"
                  /></svg
                >
              </button>
            {/if}
            {#if q.running}
              {@const id = q.running.id}
              <button class="icon" aria-label="Cancel" title="Cancel" onclick={() => run(() => cancelJob(id))}>
                <svg
                  aria-hidden="true"
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2.2"
                  stroke-linecap="round"><path d="M6 6l12 12M18 6L6 18" /></svg
                >
              </button>
            {/if}
          </div>
          {#if q.running}
            <span class="bar"><span style="width: {pct ?? 0}%"></span></span>
          {/if}
        </div>
      {/if}
      <div class="lists">
        {#if q.pending.length}
          <h3>Up next</h3>
          <ul>
            {#each q.pending as job, i (job.id)}
              <li
                class="row"
                draggable="true"
                ondragstart={() => (dragging = i)}
                ondragover={(e) => e.preventDefault()}
                ondrop={() => drop(i)}
              >
                <svg
                  aria-hidden="true"
                  width="10"
                  height="14"
                  viewBox="0 0 12 16"
                  fill="var(--faint)"
                  style="flex: none"
                >
                  <circle cx="3" cy="3" r="1.4" /><circle cx="9" cy="3" r="1.4" />
                  <circle cx="3" cy="8" r="1.4" /><circle cx="9" cy="8" r="1.4" />
                  <circle cx="3" cy="13" r="1.4" /><circle cx="9" cy="13" r="1.4" />
                </svg>
                <span class="name">{title(job)}</span>
                <span class="mono">{formatLength(job.kept_s)}</span>
                <button
                  class="small"
                  aria-label="Remove from the queue"
                  title="Remove from the queue"
                  onclick={() => run(() => removeJob(job.id))}
                >
                  <svg
                    aria-hidden="true"
                    width="13"
                    height="13"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2.2"
                    stroke-linecap="round"><path d="M6 6l12 12M18 6L6 18" /></svg
                  >
                </button>
              </li>
            {/each}
          </ul>
        {/if}
        {#if q.done.length}
          <h3>Done</h3>
          <ul>
            {#each q.done as job (job.id)}
              <li class="row">
                <button
                  class="play"
                  aria-label="Play"
                  title="Play"
                  disabled={!job.output}
                  onclick={() => run(() => play(job.id))}
                >
                  <svg aria-hidden="true" width="10" height="10" viewBox="0 0 24 24" fill="currentColor"
                    ><path d="M7 4.5v15l12-7.5z" /></svg
                  >
                </button>
                <span class="name">{title(job)}</span>
                {#if job.status === "failed"}
                  <span class="state fail" title={job.error ?? ""}>Failed</span>
                {:else}
                  <span class="state">Saved</span>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
        {#if !q.pending.length && !q.done.length && !q.running}
          <p class="empty">No jobs yet</p>
        {/if}
      </div>
      {#if error}<p class="error" role="alert">{error}</p>{/if}
    </div>
  {/if}
</div>

<style>
  .jobs {
    margin-left: auto;
  }

  .chip {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 4px;
    width: 176px;
    height: 34px;
    padding: 0 6px 0 5px;
    border-radius: 8px;
    background: var(--raised);
  }

  .chip.open {
    background: var(--raised-2);
  }

  .ring {
    flex: none;
    transform: rotate(-90deg);
  }

  .num {
    flex: none;
    width: 2.7em;
    font-family: var(--mono);
    font-size: 12px;
    text-align: right;
  }

  .num.dim {
    width: 3.1em;
    color: var(--dim);
  }

  .badges {
    display: flex;
    flex: 1;
    justify-content: flex-end;
    gap: 3px;
    min-width: 0;
    overflow: hidden;
  }

  .badge {
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--raised-2);
    font-size: 11px;
    font-weight: 700;
  }

  .badge.fail {
    background: var(--red-dark);
    color: var(--red-text);
  }

  .pop {
    position: absolute;
    top: 54px;
    right: 14px;
    z-index: 20;
    display: flex;
    flex-direction: column;
    width: 400px;
    overflow: hidden;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--raised);
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6);
  }

  .now {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    background: var(--green-dark);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  img {
    flex: none;
    width: 80px;
    height: 45px;
    border: 1px solid var(--line);
    border-radius: 4px;
    background: var(--bg);
    object-fit: cover;
  }

  .what {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .what .name {
    font-size: 14px;
    font-weight: 600;
  }

  .sub {
    color: var(--dim);
    font-size: 12px;
  }

  .icon {
    display: grid;
    place-items: center;
    flex: none;
    width: 30px;
    height: 30px;
    border-radius: 6px;
    background: var(--raised);
  }

  .bar {
    display: block;
    height: 4px;
    overflow: hidden;
    border-radius: 2px;
    background: var(--line);
  }

  .bar span {
    display: block;
    height: 4px;
    background: var(--green);
  }

  .lists {
    padding: 6px;
  }

  h3 {
    margin: 0;
    padding: 8px 8px 4px;
    color: var(--dim);
    font-size: 11px;
    font-weight: 400;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    border-radius: 6px;
  }

  .row:hover {
    background: var(--raised-2);
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: 13px;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .mono {
    color: var(--dim);
    font-family: var(--mono);
    font-size: 12px;
  }

  .small {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 5px;
    color: var(--dim);
  }

  .play {
    display: grid;
    place-items: center;
    flex: none;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--raised-2);
  }

  .play:disabled {
    opacity: 0.35;
    cursor: default;
    filter: none;
  }

  .state {
    color: var(--dim);
    font-size: 12px;
  }

  .state.fail {
    color: var(--red-text);
  }

  .empty {
    margin: 0;
    padding: 8px;
    color: var(--dim);
    font-size: 13px;
  }

  .error {
    margin: 0;
    padding: 10px 14px;
    border-top: 1px solid var(--line);
    color: var(--red-text);
    font-size: 12px;
  }
</style>
