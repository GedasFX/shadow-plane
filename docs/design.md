# Design

The look and the screens, as settled on the design canvas on 2026-11-06. A change to a screen starts
as a new direction on the canvas, not in code.

## Look

Dark only: the mascot is a dark shadow with wiggly green outlines (its art is still to come; a
placeholder holds its place in the header).

| Token | Value | For |
|---|---|---|
| `bg` | `#0c0e13` | the window; field backgrounds |
| `panel` | `#141822` | header, columns, timeline |
| `raised` | `#1c212b` | rows, cards, popovers, secondary buttons |
| `raised-2` | `#262c38` | hover, selected, pressed |
| `line` | `#2f3542` | borders and dividers |
| `text` | `#e6e9ee` | text |
| `dim` | `#8a93a3` | secondary text, idle icons |
| `faint` | `#5d6574` | placeholders, drag handles |
| `green` | `#52c23f` | the one accent: Save, kept ranges, progress, selection |
| `green-light` | `#86d977` | green text on dark |
| `green-dark` | `#13301a` | green fills behind ranges and badges |
| `green-line` | `#3a8f2c` | unselected ranges, dividers on green |
| `red` | `#ff5b67` (text `#ff8a93`) | Discard and failures only |
| `amber` | `#f0b429` | "which game is this?" only |

- One accent. No other hue marks state; a second clip is told apart by its number, not a colour.
- Type: `Segoe UI Variable` (system UI); `Cascadia Code` for times, lengths and sizes, so digits keep
  one width.
- Buttons are icons with a tooltip; Save is the only button with a word. Player buttons are 34 px,
  the decision row 44 px.
- Text stays short: no hint sentences, no keys on screen (keys go in tooltips, chosen during the
  build; the legacy keymap is not carried over).

## Header

- The mascot, then the tabs **Library · Review · History**.
- On Review: `‹ ›` between videos, the game and the video's date, length and size; "3 of 40 to
  review" on the right. Moving on without saving is how a video is left for later.
  `‹ ›` walk the to-review videos in the Library's order (games A–Z, then path), whatever its
  filters, and dim at the ends; from a video that isn't to review the right side reads "M to review".
- The job chip, right-aligned and a fixed 176 px wide so nothing moves: a progress ring, the phase
  as an icon (encode, check quality, paused, all saved, idle), the percentage and the time left in
  fixed-width boxes, `+N` waiting, a red count when a job failed. A 2 px green line along the
  header's bottom edge shows the same progress.
- Clicking the chip opens a popover: **Now** (pause, cancel), **Up next** (drag to reorder,
  remove), **Done** (the last 5, each with play), and "Show all history".
- One job runs at a time: one encode already uses the whole machine.

## Library

Search, **To review · Saved · Discarded · All**, and a games filter: multi-select with its own
search, "All games" first, a video count per game; the button reads All games, the game, or
"2 games". Below, a grid of thumbnails grouped by game: the length, a green ✓ with the number of
saved clips, or a bin icon (dimmed) for a discarded video.

- Starts on To review. Search matches the file name and the game; a game's count follows the status
  filter and search, not the games filter.
- With no folder chosen only a choose-folder button shows; otherwise a folder icon (tooltip: the path;
  click: choose again) and a rescan icon sit in the toolbar. The scan runs at launch and after choosing; the list shows what is indexed at once and grows every 2 s while the scan runs.

## Review

- **Picture**, then the **controls**: back a frame, play/pause, forward a frame (holding a frame
  button repeats), the time with frames, Start here, End here (the selected piece's ends at the
  playhead), play the kept part, Fit (shown when zoomed, with the zoom level), speed, mute, volume.
- **Timeline**, always shown: dragging anywhere moves the playhead; a green end drags that end;
  Ctrl + scroll zooms around the mouse, scroll pans when zoomed, and an overview strip below shows
  and drags the visible window. The wheel listener isn't passive and WebView2's own zoom is off, or
  the page zooms instead.
- A fresh video's clip is the whole video, green end to end.
- Pieces: numbered when there are several, a grip at each end; ends snap to whole frames, a piece
  is at least 0.5 s, and an end can't pass the next piece in time. Saved clips of the video show
  hatched grey.
- Keys, in tooltips only: Space play/pause, Left and Right a frame, I Start here, O End here.
- **Right column**, always shown, top to bottom:
  1. the clip's length, Great, Best;
  2. title (required: Save without one marks it red), game (amber when the folder is Base Profile);
  3. the pieces, one row each (start → end, length); `⋯` on hover: Split (halves the piece; adjust
     the ends by hand) and Remove (two pieces or more); drag to reorder;
  4. between two pieces a `+`: Crossfade, Fade to black (with a length), or insert a clip; nothing
     picked is a cut, and clicking the picked one again unpicks it;
  5. a dashed `+` at the end: insert a clip, picked from the clips already saved;
  6. the file name it will be saved as;
  7. **Discard** (asks first: "It moves to the _Bin folder.") and **Save**, a split button: the click
     saves and opens the next video; its menu's one item, Save & clip this video again, stays and
     starts a new whole-video clip, the saved range hatched grey; title, game and labels start over. With no video
     left to review, the Library shows. A toast says what happened.

## History

Search, **All · Saved · Failed**, games; rows grouped by day: the thumbnail plays the clip in the
default video player; title, game and time; sizes before → after; the saving (or "Original kept",
or Failed in red); on hover, show in folder and `⋯`.

## Which stage builds what

| Stage | Adds |
|---|---|
| 3 Review | the Review screen with one or more pieces from one video, Discard and Save; the tabs, Library screen and `‹ ›` |
| 4 Encode | the job chip, its popover, the progress line |
| 6 History | the History screen and its actions |
| 8 Joins | transitions, inserting a saved clip, pieces from other videos (a migration: transitions per join) |
