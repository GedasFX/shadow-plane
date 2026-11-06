# shadow-plane — working brief

A Windows desktop app for game clips: review a capture, trim it, re-encode it to a measured quality,
share it. Tauri 2: a Rust backend and a Svelte page in WebView2; installs per user, updates itself.

Keep this file current. No changelog — git log is the history.

## Where things live

| Path | Job |
|---|---|
| `src/routes/` | the page: SvelteKit as a single-page app (adapter-static, no SSR); `+page.svelte` is the shell (tabs, update check at launch) |
| `src/app.css` | the design's tokens as CSS custom properties, the font stacks, base button and focus styles |
| `src/lib/api.ts` | the page's contract with Rust: one typed wrapper per command, the types they return, the `media` URL helpers; read this, not `main.rs` |
| `src/lib/library.ts` | the Library's pure logic: filters, search, counts, grouping, formatting |
| `src/lib/videos.svelte.ts` | the videos index shared by the Library and the Review header: `store.videos`, `refresh()` after a scan, a Save and a Discard |
| `src/lib/clip.ts` | the Review's clip: its pieces, the selected one, set an end, split, remove, reorder, the kept length and the play-the-kept-part order; pure |
| `src/lib/timeline.ts` | the timeline's maths: zoom window, pan, ruler marks, time and frame; pure |
| `src/lib/` | components (`Header`, `Library`, `GamesFilter`, `VideoCard`, `Review` with its `Player`, `Timeline` and `ClipColumn`, the right column); each tested beside it (`*.test.ts`) in happy-dom, Tauri mocked with `mockIPC` |
| `src-tauri/` | the Rust app: `src/main.rs`; `capabilities/` (what the page may call) |
| `src-tauri/src/db.rs`, `src-tauri/migrations/` | SQLite in the app's data folder; numbered migrations applied at start |
| `src-tauri/src/settings.rs` | the `setting` table: the library folder |
| `src-tauri/src/library.rs` | the scan: the library's `.mp4` files into the `source` index, probing only new and changed files and storing each as it is probed; a video's game folder (`Misc` outside any); `_Bin`'s files are indexed too, flagged `binned`, under the key they had before the move |
| `src-tauri/src/jobs.rs` | the `job` table: one job per saved clip, queued by `review::save`; the queue for the popover, reorder, remove (pending becomes cancelled) |
| `src-tauri/src/bin.rs` | `bin`: moves a video to `_Bin/<key>` and flags its row; discard calls it before recording the decision |
| `src-tauri/src/files.rs` | `move_file` (rename, else copy + SHA-1 compare + delete) and `copy_verified`; neither overwrites |
| `src-tauri/src/ffmpeg.rs` | FFmpeg's tools, found on `PATH`, run without a console window: a video's length, size and frame rate; its audio track count and colour tags |
| `src-tauri/src/encode_args.rs` | `normalise` a clip's pieces and `args`, the ffmpeg arguments for one encode (pure); the recipe lives here |
| `src-tauri/src/encoder.rs` | `Encoder`, the app's one kill-on-close job; `start` runs ffmpeg below-normal in it with progress, and `Running` pauses, resumes, kills, waits |
| `src-tauri/src/verify.rs` | the output checks: `probes` (video stream, size, length within 2%) and `decodes` (full decode, any error fails) |
| `src-tauri/src/media.rs` | the `media` protocol: a library video in byte ranges (only below the library folder; a binned video is found in `_Bin`) and its thumbnail, made by ffmpeg and cached by key, size and modified time; the page's URLs are `convertFileSrc("video/" + key, "media")` and `"thumb/" + key` |
| `src-tauri/tauri.conf.json` | the window; the per-user NSIS installer; the updater (public key, GitHub's `latest.json`) |
| `.github/workflows/release.yml` | a `v*` tag publishes the release; a manual run builds without publishing |
| `scripts/win.sh` | from WSL, the Rust side with the Windows toolchain: `fmt`, `check`, `test`, `build` |
| `docs/design.md` | the look (colours, type, buttons) and each screen's layout and behaviour; read before UI work |

## How to run and test

`bun run check && bun run lint && bun run test && bun run build` (in WSL), `scripts/win.sh check`
(`cargo fmt --check`, `clippy -D warnings`) and `scripts/win.sh test` must pass before a commit. Biome
formats everything but `.svelte`; Prettier formats `.svelte`, in the same style. `scripts/win.sh build`
makes the release .exe.

To release: set `version` in `src-tauri/Cargo.toml`, commit, then tag `v<version>` and push the tag.
The workflow refuses a tag that differs from that version. Installed copies check for the latest
release at launch and update themselves.

## Invariants

- The page reaches Rust only through Tauri commands, events and the `media` protocol: no local
  server, no port.
- The updater signing key (Actions secrets `TAURI_SIGNING_PRIVATE_KEY` and its password) never
  changes: installed copies refuse a release signed with any other key. A backup is kept outside the
  repo.
- A migration never changes once released: each runs once (`PRAGMA user_version` counts them), so a
  schema change is a new numbered file.
- The scan skips folders whose names start with `_`: the app's output lives inside the library, and
  scanning it lists finished clips as new sources. `_Bin` alone is walked, apart, its files flagged
  binned under the key they had (their path below `_Bin`).
- A source's key is its path below the library folder with `/` between parts, the same key a decision
  has, so decisions survive a library move.
- A quality profile is keyed by encoder as well as game, output tier and bits-per-pixel band: each
  encoder's quality setting is its own scale, so their measurements never mix.

## Traps

- The frontend's `node_modules` is Linux's (bun in WSL): Windows runs only cargo, on an already built
  `build/`.
