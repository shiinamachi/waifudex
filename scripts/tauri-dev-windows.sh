#!/bin/sh
set -eu

resolve_tauri_dev_host() {
  hostname -I | awk '{ print $1 }'
}

TAURI_DEV_HOST="${TAURI_DEV_HOST:-$(resolve_tauri_dev_host)}"

if [ -z "$TAURI_DEV_HOST" ]; then
  echo "failed to resolve TAURI_DEV_HOST for Windows dev" >&2
  exit 1
fi

export TAURI_DEV_HOST
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUNNER=../scripts/wsl-exec.sh

pnpm inochi2d:build:windows

exec pnpm tauri dev \
  --config "{\"build\":{\"devUrl\":\"http://${TAURI_DEV_HOST}:1420\"}}" \
  --runner cargo-xwin \
  --target x86_64-pc-windows-msvc
