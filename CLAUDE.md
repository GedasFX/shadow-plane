# shadow-plane — working brief

A Windows desktop app for game clips: review a capture, trim it, re-encode it to a measured quality,
share it. Tauri 2: a Rust backend and a Svelte page in WebView2; installs per user, updates itself.

Keep this file current. No changelog — git log is the history.

## Where things live

| Path | Job |
|---|---|
| `src/routes/` | the page: SvelteKit as a single-page app (adapter-static, no SSR) |
| `src/**/*.svelte.test.ts` | component tests in happy-dom; Tauri mocked with `mockIPC` |
| `src-tauri/` | the Rust app: `src/main.rs`; `capabilities/` (what the page may call) |
| `src-tauri/tauri.conf.json` | the window; the per-user NSIS installer; the updater (public key, GitHub's `latest.json`) |
| `.github/workflows/release.yml` | a `v*` tag publishes the release; a manual run builds without publishing |
| `scripts/win.sh` | from WSL, the Rust side with the Windows toolchain: `fmt`, `check`, `build` |

## How to run and test

`bun run check && bun run lint && bun run test && bun run build` (in WSL) and `scripts/win.sh check`
(`cargo fmt --check`, `clippy -D warnings`) must pass before a commit. Biome formats everything but
`.svelte`; Prettier formats `.svelte`, in the same style. `scripts/win.sh build` makes the release .exe.

To release: set `version` in `src-tauri/Cargo.toml`, commit, then tag `v<version>` and push the tag.
The workflow refuses a tag that differs from that version. Installed copies check for the latest
release at launch and update themselves.

## Invariants

- The page reaches Rust only through Tauri commands and events: no local server, no port.
- The updater signing key (Actions secrets `TAURI_SIGNING_PRIVATE_KEY` and its password) never
  changes: installed copies refuse a release signed with any other key. A backup is kept outside the
  repo.

## Traps

- The frontend's `node_modules` is Linux's (bun in WSL): Windows runs only cargo, on an already built
  `build/`.
