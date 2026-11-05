# shadow-plane — working brief

A Windows desktop app for game clips: review a capture, trim it, re-encode it to a measured quality,
share it. Tauri 2: a Rust backend and a Svelte page in WebView2; installs per user, updates itself.

Keep this file current. No changelog — git log is the history.

## Where things live

| Path | Job |
|---|---|
| `src/routes/` | the page: SvelteKit as a single-page app (adapter-static, no SSR) |
| `src/**/*.svelte.test.ts` | component tests in happy-dom; Tauri mocked with `mockIPC` |
| `src-tauri/` | the Rust app: `src/main.rs`; `tauri.conf.json` (window, bundle); `capabilities/` (what the page may call) |
| `scripts/win.sh` | from WSL, the Rust side with the Windows toolchain: `check`, `build` |

## How to run and test

`bun run check && bun run lint && bun run test && bun run build` (in WSL) and `scripts/win.sh check`
(`cargo fmt --check`, `clippy -D warnings`) must pass before a commit. Biome formats everything but
`.svelte`; Prettier formats `.svelte`, in the same style. `scripts/win.sh build` makes the release .exe.

## Traps

- The frontend's `node_modules` is Linux's (bun in WSL): Windows runs only cargo, on an already built
  `build/`.

## Invariants

- The page reaches Rust only through Tauri commands and events: no local server, no port.
