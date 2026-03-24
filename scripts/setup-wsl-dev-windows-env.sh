#!/usr/bin/env bash
set -euo pipefail

if ! grep -q '^ID=ubuntu$' /etc/os-release; then
  echo "This task supports Ubuntu only." >&2
  exit 1
fi

if [ -z "${WSL_DISTRO_NAME:-}" ] && ! grep -qiE '(microsoft|wsl)' /proc/version; then
  echo "This task supports Ubuntu on WSL only." >&2
  exit 1
fi

sudo apt update
sudo apt install -y llvm clang lld cmake ninja-build python3 ldc dub libegl1-mesa-dev libclang-dev

if ! command -v cargo-xwin >/dev/null 2>&1; then
  cargo install --locked cargo-xwin
fi

cargo xwin env --target x86_64-pc-windows-msvc >/dev/null
pnpm install
