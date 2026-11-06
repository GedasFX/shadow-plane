<script lang="ts">
  import { type Clip, keptLength, type Meta, remove, reorder, select, split } from "./clip";
  import { formatLength, formatTenths } from "./library";

  type Props = {
    clip: Clip;
    meta: Meta;
    fileName: string;
    // Save was tried with no title.
    missing: boolean;
    onsave: (another: boolean) => void;
    ondiscard: () => void;
  };

  let { clip = $bindable(), meta = $bindable(), fileName, missing, onsave, ondiscard }: Props = $props();

  let menu = $state(-1);
  let saveMenu = $state(false);
  let confirming = $state(false);
  let dragging = $state(-1);

  const pieces = $derived(clip.pieces.length);
  const total = $derived(
    pieces > 1 ? `${pieces} pieces · ${formatLength(keptLength(clip))}` : formatLength(keptLength(clip)),
  );

  function closeMenus() {
    menu = -1;
    saveMenu = false;
  }

  // The click that opens a menu would reach the window and close it again.
  function toggle(e: MouseEvent, open: () => void) {
    e.stopPropagation();
    open();
  }
</script>

<svelte:window
  onclick={closeMenus}
  onkeydown={(e) => {
    if (e.key !== "Escape") return;

    closeMenus();
    confirming = false;
  }}
/>

<aside aria-label="Clip">
  <div class="body">
    <div class="top">
      <span class="total">{total}</span>
      <button
        class="mark"
        aria-label="Great"
        title="Great"
        aria-pressed={meta.great}
        onclick={() => (meta.great = !meta.great)}
      >
        <svg
          aria-hidden="true"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linejoin="round"
          ><path d="M12 3l2.8 5.7 6.2.9-4.5 4.4 1 6.2L12 17.3 6.5 20.2l1-6.2L3 9.6l6.2-.9z" /></svg
        >
      </button>
      <button
        class="mark"
        aria-label="Best"
        title="Best"
        aria-pressed={meta.best}
        onclick={() => (meta.best = !meta.best)}
      >
        <svg
          aria-hidden="true"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"><path d="M3 18a9 9 0 0118 0" /><path d="M7 18a5 5 0 0110 0" /></svg
        >
      </button>
    </div>
    <input class="title" aria-label="Title" placeholder="Title" aria-invalid={missing} bind:value={meta.title} />
    <input class="game" aria-label="Game" class:amber={meta.game === "Base Profile"} bind:value={meta.game} />

    <ol>
      {#each clip.pieces as piece, i (i)}
        <li
          class="row"
          class:selected={i === clip.selected}
          draggable="true"
          ondragstart={() => (dragging = i)}
          ondragover={(e) => e.preventDefault()}
          ondrop={() => {
            if (dragging >= 0) clip = reorder(clip, dragging, i);
            dragging = -1;
          }}
        >
          <svg aria-hidden="true" width="10" height="14" viewBox="0 0 12 16" fill="var(--faint)" style="flex: none">
            <circle cx="3" cy="3" r="1.4" /><circle cx="9" cy="3" r="1.4" />
            <circle cx="3" cy="8" r="1.4" /><circle cx="9" cy="8" r="1.4" />
            <circle cx="3" cy="13" r="1.4" /><circle cx="9" cy="13" r="1.4" />
          </svg>
          <button class="pick" aria-label="Piece {i + 1}" onclick={() => (clip = select(clip, i))}>
            <span class="range">{formatTenths(piece.start_s)} → {formatTenths(piece.end_s)}</span>
            <span class="length">{formatTenths(piece.end_s - piece.start_s)}</span>
          </button>
          <button
            class="more"
            aria-label="More"
            title="More"
            aria-haspopup="menu"
            aria-expanded={menu === i}
            onclick={(e) => toggle(e, () => (menu = menu === i ? -1 : i))}
          >
            <svg aria-hidden="true" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <circle cx="5" cy="12" r="1.8" /><circle cx="12" cy="12" r="1.8" /><circle cx="19" cy="12" r="1.8" />
            </svg>
          </button>
          {#if menu === i}
            <div class="menu row-menu" role="menu">
              <button
                role="menuitem"
                onclick={() => {
                  clip = split(clip, i);
                  menu = -1;
                }}>Split</button
              >
              {#if pieces > 1}
                <button
                  class="danger"
                  role="menuitem"
                  onclick={() => {
                    clip = remove(clip, i);
                    menu = -1;
                  }}>Remove</button
                >
              {/if}
            </div>
          {/if}
        </li>
      {/each}
    </ol>
  </div>

  <div class="file">
    <svg
      aria-hidden="true"
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linejoin="round"
      style="flex: none; margin-top: 2px"><path d="M6 3h8l4 4v14H6z" /><path d="M14 3v4h4" /></svg
    >
    <span aria-label="File name">{fileName}</span>
  </div>

  <div class="actions">
    <button
      class="bin"
      class:open={confirming}
      aria-label="Discard this video"
      title="Discard this video"
      aria-expanded={confirming}
      onclick={(e) => toggle(e, () => ((confirming = !confirming), (saveMenu = false)))}
    >
      <svg
        aria-hidden="true"
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="var(--red-text)"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"><path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13" /></svg
      >
    </button>
    <div class="save">
      <button class="go" onclick={() => onsave(false)}>Save</button>
      <span class="divider"></span>
      <button
        class="chevron"
        class:open={saveMenu}
        aria-label="More ways to save"
        title="More ways to save"
        aria-haspopup="menu"
        aria-expanded={saveMenu}
        onclick={(e) => toggle(e, () => ((saveMenu = !saveMenu), (confirming = false)))}
      >
        <svg
          aria-hidden="true"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.4"
          stroke-linecap="round"
          stroke-linejoin="round"><path d="M6 15l6-6 6 6" /></svg
        >
      </button>
    </div>

    {#if confirming}
      <div class="confirm" role="alertdialog" aria-label="Discard this video">
        <div class="ask">
          <span class="head">Discard this video?</span>
          <span class="note">It moves to the _Bin folder.</span>
        </div>
        <div class="buttons">
          <button class="cancel" onclick={() => (confirming = false)}>Cancel</button>
          <button
            class="yes"
            onclick={() => {
              confirming = false;
              ondiscard();
            }}>Discard</button
          >
        </div>
      </div>
    {/if}

    {#if saveMenu}
      <div class="menu save-menu" role="menu">
        <button role="menuitem" onclick={() => onsave(false)}>Save and continue</button>
        <button role="menuitem" onclick={() => onsave(true)}>Save and make another</button>
      </div>
    {/if}
  </div>
</aside>

<style>
  aside {
    flex: none;
    width: 320px;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--line);
    background: var(--panel);
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 16px;
  }

  .top {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .total {
    flex: 1;
    color: var(--dim);
    font-family: var(--mono);
    font-size: 13px;
  }

  .mark {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--dim);
  }

  .mark[aria-pressed="true"] {
    border-color: var(--green);
    background: var(--green-dark);
    color: var(--green-light);
  }

  .title,
  .game {
    box-sizing: border-box;
    width: 100%;
    padding: 9px 10px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--bg);
  }

  .title {
    font-size: 16px;
    font-weight: 600;
  }

  .title[aria-invalid="true"] {
    border-color: var(--red);
  }

  .game {
    font-size: 13px;
  }

  .game.amber {
    border-color: var(--amber);
    color: var(--amber);
  }

  ol {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 4px 0 0;
    padding: 12px 0 0;
    border-top: 1px solid var(--line);
    list-style: none;
  }

  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--raised);
    cursor: grab;
  }

  .row.selected {
    border-color: var(--green);
  }

  .pick {
    flex: 1;
    min-width: 0;
    display: flex;
    gap: 10px;
    font-family: var(--mono);
    font-size: 12px;
    text-align: left;
  }

  .range {
    flex: 1;
  }

  .length {
    color: var(--dim);
  }

  .more {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    margin: -4px -6px -4px 0;
    border-radius: 5px;
    color: var(--dim);
    visibility: hidden;
  }

  .row:hover .more,
  .more:focus-visible,
  .more[aria-expanded="true"] {
    visibility: visible;
  }

  .menu {
    position: absolute;
    z-index: 6;
    padding: 4px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--raised);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
  }

  .menu button {
    display: block;
    width: 100%;
    padding: 8px 10px;
    border-radius: 5px;
    font-size: 13px;
    text-align: left;
  }

  .menu .danger {
    color: var(--red-text);
  }

  .row-menu {
    right: 4px;
    top: 40px;
    min-width: 140px;
  }

  .file {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    padding: 12px 16px 0;
    border-top: 1px solid var(--line);
    color: var(--dim);
    font-family: var(--mono);
    font-size: 12px;
    line-height: 1.5;
    word-break: break-word;
  }

  .actions {
    position: relative;
    display: flex;
    gap: 8px;
    padding: 12px 16px 16px;
  }

  .bin {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 8px;
    background: var(--raised);
  }

  .bin.open {
    background: var(--red-dark);
  }

  .save {
    flex: 1;
    display: flex;
    height: 44px;
    overflow: hidden;
    border-radius: 8px;
    background: var(--green);
    color: #07160a;
  }

  .go {
    flex: 1;
    font-size: 15px;
    font-weight: 700;
  }

  .divider {
    width: 1px;
    margin: 8px 0;
    background: var(--green-line);
  }

  .chevron {
    display: grid;
    place-items: center;
    width: 40px;
  }

  .chevron.open {
    background: var(--green-light);
  }

  .save-menu {
    right: 16px;
    bottom: 68px;
    width: 232px;
  }

  .confirm {
    position: absolute;
    left: 16px;
    bottom: 68px;
    z-index: 8;
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 248px;
    padding: 14px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--raised);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.55);
  }

  .ask {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .head {
    font-size: 14px;
    font-weight: 600;
  }

  .note {
    color: var(--dim);
    font-size: 13px;
  }

  .buttons {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .cancel,
  .yes {
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 13px;
  }

  .cancel {
    background: var(--raised-2);
  }

  .yes {
    background: var(--red);
    color: #1a0507;
    font-weight: 700;
  }
</style>
