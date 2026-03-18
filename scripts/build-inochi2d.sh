#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_DIR="${ROOT_DIR}/third_party/inochi2d-c"
# Upstream dub.sdl already targets third_party/inochi2d-c/out.
OUT_DIR="${SOURCE_DIR}/out"
DUB_BIN="${DUB_BIN:-dub}"
LDC2_BIN="${LDC2_BIN:-ldc2}"

if [[ ! -d "${SOURCE_DIR}" ]]; then
  echo "missing submodule: ${SOURCE_DIR}" >&2
  echo "run: git submodule update --init --recursive" >&2
  exit 1
fi

if ! command -v "${LDC2_BIN}" >/dev/null 2>&1; then
  echo "missing required tool: ${LDC2_BIN}" >&2
  exit 1
fi

if ! command -v "${DUB_BIN}" >/dev/null 2>&1; then
  echo "missing required tool: ${DUB_BIN}" >&2
  exit 1
fi

mkdir -p "${OUT_DIR}"

(
  cd "${SOURCE_DIR}"
  if [[ "${DUB_BIN}" == "dub" && "${LDC2_BIN}" == "ldc2" ]]; then
    dub build --compiler=ldc2 --config=yesgl
  else
    "${DUB_BIN}" build --compiler="${LDC2_BIN}" --config=yesgl
  fi
)

case "$(uname -s)" in
  Linux*)
    test -f "${OUT_DIR}/libinochi2d-c.so"
    ;;
  MINGW*|MSYS*|CYGWIN*|Windows_NT)
    test -f "${OUT_DIR}/inochi2d-c.dll"
    test -f "${OUT_DIR}/inochi2d-c.lib"
    ;;
  *)
    echo "unsupported platform for validation: $(uname -s)" >&2
    ;;
esac
