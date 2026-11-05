# shadow-plane — working brief

A Windows desktop app for game clips: review a capture, trim it, re-encode it to a measured quality,
share it. Tauri 2: a Rust backend and a Svelte page in WebView2; installs per user, updates itself.

Keep this file current. No changelog — git log is the history.

## Where things live

| Path | Job |
|---|---|
| `src/routes/` | the page: SvelteKit as a single-page app (adapter-static, no SSR) |
| `src/**/*.svelte.test.ts` | component tests in happy-dom; Tauri mocked with `mockIPC` |

## How to run and test

`bun run check && bun run lint && bun run test && bun run build` (in WSL) must pass before a commit.
Biome formats everything but `.svelte`; Prettier formats `.svelte`, in the same style.

## Invariants

- The page reaches Rust only through Tauri commands and events: no local server, no port.
