#!/usr/bin/env bash
set -euo pipefail

if ! grep -q '^ID=ubuntu$' /etc/os-release; then
  echo "WSL dev setup supports Ubuntu only." >&2
  exit 1
fi

if [ -z "${WSL_DISTRO_NAME:-}" ] && ! grep -qiE '(microsoft|wsl)' /proc/version; then
  echo "WSL dev setup supports Ubuntu on WSL only." >&2
  exit 1
fi

missing=()
for cmd in node pnpm cargo cargo-xwin clang lld dub ldc2 ldc-build-runtime cmake ninja python3; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    missing+=("$cmd")
  fi
done

if [ ! -d node_modules ]; then
  missing+=("node_modules")
fi

if [ "${#missing[@]}" -eq 0 ]; then
  exit 0
fi

if ! command -v mise >/dev/null 2>&1; then
  echo "Missing WSL dev prerequisites: ${missing[*]}" >&2
  echo "mise is not installed, so automatic setup cannot continue." >&2
  exit 1
fi

echo "Missing WSL dev prerequisites detected: ${missing[*]}"
echo "Running: mise run setup:wsl-dev-windows"
mise run setup:wsl-dev-windows

missing=()
for cmd in node pnpm cargo cargo-xwin clang lld dub ldc2 ldc-build-runtime cmake ninja python3; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    missing+=("$cmd")
  fi
done

if [ ! -d node_modules ]; then
  missing+=("node_modules")
fi

if [ "${#missing[@]}" -gt 0 ]; then
  echo "WSL dev prerequisites are still missing after setup: ${missing[*]}" >&2
  exit 1
fi
