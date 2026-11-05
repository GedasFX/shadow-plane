#!/usr/bin/env bash
# From WSL, run the Rust side with the Windows toolchain, on this same folder. (On Windows itself,
# run the cargo commands in src-tauri/ directly.) The frontend builds here first: its node_modules
# is Linux's, so Windows never runs the frontend tools.
#
#   scripts/win.sh check   cargo fmt --check, then clippy with warnings as errors
#   scripts/win.sh build   the frontend, then the release .exe; prints its path
set -euo pipefail
cd "$(dirname "$0")/.."

case "${1:-}" in
  check) cmd='& $cargo fmt --check; if ($LASTEXITCODE) { exit $LASTEXITCODE }; & $cargo clippy --all-targets -- -D warnings' ;;
  build) bun run build >/dev/null
         cmd='& $cargo tauri build --no-bundle; if ($LASTEXITCODE) { exit $LASTEXITCODE }; "$env:CARGO_TARGET_DIR\release\shadow-plane.exe"' ;;
  *) echo "usage: scripts/win.sh check|build" >&2; exit 2 ;;
esac

# Build output on a local Windows disk: linking over the WSL share is slow.
powershell.exe -NoProfile -Command "
  \$ErrorActionPreference = 'Stop'
  \$cargo = \"\$env:USERPROFILE\\.cargo\\bin\\cargo.exe\"
  \$env:CARGO_TARGET_DIR = \"\$env:LOCALAPPDATA\\cargo-target\\shadow-plane\"
  Set-Location '$(wslpath -w src-tauri)'
  $cmd
  exit \$LASTEXITCODE
" | tr -d '\r'
