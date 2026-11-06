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
| `src/lib/` | components (`Header`, `Library`, `GamesFilter`, `VideoCard`, `Review`); each tested beside it (`*.test.ts`) in happy-dom, Tauri mocked with `mockIPC` |
| `src-tauri/` | the Rust app: `src/main.rs`; `capabilities/` (what the page may call) |
| `src-tauri/src/db.rs`, `src-tauri/migrations/` | SQLite in the app's data folder; numbered migrations applied at start |
| `src-tauri/src/settings.rs` | the `setting` table: the library folder |
| `src-tauri/src/library.rs` | the scan: the library's `.mp4` files into the `source` index, probing only new and changed files; a video's game folder (`Misc` outside any) |
| `src-tauri/src/ffmpeg.rs` | FFmpeg's tools, found on `PATH`, run without a console window: a video's length, size and frame rate |
| `src-tauri/src/media.rs` | the `media` protocol: a library video in byte ranges (only below the library folder) and its thumbnail, made by ffmpeg and cached by key, size and modified time; the page's URLs are `convertFileSrc("video/" + key, "media")` and `"thumb/" + key` |
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
- The scan skips folders whose names start with `_`: the app's output and bin live inside the library,
  and scanning them lists finished clips and binned originals as new sources.
- A source's key is its path below the library folder with `/` between parts, the same key a decision
  has, so decisions survive a library move.
- A quality profile is keyed by encoder as well as game, output tier and bits-per-pixel band: each
  encoder's quality setting is its own scale, so their measurements never mix.

## Traps

- The frontend's `node_modules` is Linux's (bun in WSL): Windows runs only cargo, on an already built
  `build/`.
