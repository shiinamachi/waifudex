#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_DIR="${ROOT_DIR}/third_party/inochi2d-c"
OUT_DIR="${SOURCE_DIR}/out"
TARGET_TRIPLE="x86_64-pc-windows-msvc"
XWIN_DIR="${XWIN_DIR:-${HOME}/.cache/cargo-xwin/xwin}"
WAIFUDEX_CACHE_DIR="${XDG_CACHE_HOME:-${HOME}/.cache}/waifudex/inochi2d-windows"
RUNTIME_BUILD_DIR="${WAIFUDEX_CACHE_DIR}/ldc-runtime"
RUNTIME_LIB_DIR="${WAIFUDEX_CACHE_DIR}/runtime-lib"
WORK_DIR="${WAIFUDEX_CACHE_DIR}/work"
WRAPPER_DIR="${WAIFUDEX_CACHE_DIR}/dub-wrapper"
REAL_DUB="${DUB_BIN:-$(command -v dub)}"
REAL_LDC2="${LDC2_BIN:-$(command -v ldc2)}"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required tool: $1" >&2
    exit 1
  fi
}

ensure_host_gitver() {
  local host_gitver_dir="${HOME}/.dub/packages/gitver/1.7.2/gitver"
  local host_gitver_bin="${host_gitver_dir}/out/gitver"

  mkdir -p "${WRAPPER_DIR}"
  (
    cd "${host_gitver_dir}"
    "${REAL_DUB}" build --compiler="${REAL_LDC2}" --force >/dev/null
  )

  cat >"${WRAPPER_DIR}/dub" <<EOF
#!/usr/bin/env bash
set -euo pipefail
REAL_DUB="${REAL_DUB}"
HOST_GITVER_BIN="${host_gitver_bin}"
if [[ "\${1-}" == "run" && "\${2-}" == "gitver" ]]; then
  shift 2
  if [[ "\${1-}" == "--" ]]; then
    shift
  fi
  exec "\${HOST_GITVER_BIN}" "\$@"
fi
exec "\${REAL_DUB}" "\$@"
EOF
  chmod +x "${WRAPPER_DIR}/dub"
}

build_runtime_libs() {
  local runtime_d_flags
  runtime_d_flags="--mtriple=${TARGET_TRIPLE};--linker=lld-link;--mscrtlib=msvcrt;-link-defaultlib-shared=false"
  local runtime_c_flags
  runtime_c_flags="--target=${TARGET_TRIPLE};-Wno-unused-command-line-argument;-fuse-ld=lld-link;-isystem;${XWIN_DIR}/crt/include;-isystem;${XWIN_DIR}/sdk/include/ucrt;-isystem;${XWIN_DIR}/sdk/include/um;-isystem;${XWIN_DIR}/sdk/include/shared;-isystem;${XWIN_DIR}/sdk/include/winrt"
  local runtime_linker_flags
  runtime_linker_flags="-fuse-ld=lld-link;/LIBPATH:${XWIN_DIR}/crt/lib/x86_64;/LIBPATH:${XWIN_DIR}/sdk/lib/um/x86_64;/LIBPATH:${XWIN_DIR}/sdk/lib/ucrt/x86_64"

  rm -rf "${RUNTIME_BUILD_DIR}" "${RUNTIME_LIB_DIR}"
  mkdir -p "${RUNTIME_BUILD_DIR}" "${RUNTIME_LIB_DIR}"

  (
    export PATH="/usr/bin:/bin:${HOME}/.cargo/bin:${PATH}"
    export CC=clang
    export CXX=clang++

    ldc-build-runtime \
      --ninja \
      --buildDir "${RUNTIME_BUILD_DIR}" \
      CMAKE_POLICY_VERSION_MINIMUM=3.5 \
      BUILD_SHARED_LIBS=OFF \
      HAVE_UNISTD_H=0 \
      --targetSystem "Windows;MSVC" \
      --dFlags="${runtime_d_flags}" \
      --cFlags="${runtime_c_flags}" \
      --linkerFlags="${runtime_linker_flags}" \
      -j2 >/dev/null 2>&1 || true

    (
      cd "${RUNTIME_BUILD_DIR}"
      ninja -k 0 lib/libdruntime-ldc.a lib/libphobos2-ldc.a >/dev/null 2>&1 || true
    )
  )

  python3 - "${RUNTIME_BUILD_DIR}" "${RUNTIME_LIB_DIR}" <<'PY'
import os
import shlex
import subprocess
import sys

build_dir = sys.argv[1]
out_dir = sys.argv[2]
targets = [
    ("lib/libdruntime-ldc.a", "druntime-ldc.lib"),
    ("lib/libphobos2-ldc.a", "phobos2-ldc.lib"),
]

for ninja_target, out_name in targets:
    output = subprocess.check_output(
        ["ninja", "-t", "commands", ninja_target], cwd=build_dir, text=True
    )
    link_line = [line for line in output.splitlines() if " -lib -of=" in line][-1]
    args = shlex.split(link_line)
    objects = []
    capture = False
    for arg in args:
        if arg.startswith("-of=lib/lib"):
            capture = True
            continue
        if not capture:
            continue
        if arg.startswith("-") or arg in {"&&", ":"}:
            break
        if arg.endswith(".o") or arg.endswith(".obj"):
            direct = os.path.join(build_dir, arg)
            alt = (
                os.path.join(build_dir, arg[:-2] + ".obj")
                if arg.endswith(".o")
                else direct
            )
            if os.path.exists(direct):
                objects.append(direct)
            elif os.path.exists(alt):
                objects.append(alt)
            else:
                raise SystemExit(f"missing runtime object: {direct} or {alt}")
    subprocess.run(
        ["llvm-lib", f"/OUT:{os.path.join(out_dir, out_name)}", *objects],
        check=True,
    )
PY

  test -f "${RUNTIME_LIB_DIR}/druntime-ldc.lib"
  test -f "${RUNTIME_LIB_DIR}/phobos2-ldc.lib"
}

build_windows_inochi2d() {
  local import_paths
  local string_import_paths
  local versions
  local source_files
  local linker_files
  local top_obj
  local dll_path
  local lib_path

  import_paths="$(dub describe --compiler="${REAL_LDC2}" --config=yesgl --data=import-paths --data-list)"
  string_import_paths="$(dub describe --compiler="${REAL_LDC2}" --config=yesgl --data=string-import-paths --data-list)"
  versions="$(dub describe --compiler="${REAL_LDC2}" --config=yesgl --data=versions --data-list)"
  source_files="$(dub describe --compiler="${REAL_LDC2}" --config=yesgl --data=source-files --data-list)"
  linker_files="$(dub describe --compiler="${REAL_LDC2}" --config=yesgl --data=linker-files --data-list)"
  export LINKER_FILES="${linker_files}"

  rm -rf "${WORK_DIR}"
  mkdir -p "${WORK_DIR}" "${OUT_DIR}"

  top_obj="${WORK_DIR}/inochi2d-c.obj"
  local compile_args=(
    -c
    -fvisibility=hidden
    -link-defaultlib-shared=false
    --mtriple="${TARGET_TRIPLE}"
    --linker=lld-link
    --mscrtlib=msvcrt
    -of="${top_obj}"
  )

  while IFS= read -r line; do
    [[ -n "${line}" ]] && compile_args+=("-I${line}")
  done <<< "${import_paths}"
  while IFS= read -r line; do
    [[ -n "${line}" ]] && compile_args+=("-J${line}")
  done <<< "${string_import_paths}"
  while IFS= read -r line; do
    [[ -n "${line}" ]] && compile_args+=("-d-version=${line}")
  done <<< "${versions}"
  while IFS= read -r line; do
    [[ -n "${line}" ]] && compile_args+=("${line}")
  done <<< "${source_files}"

  "${REAL_LDC2}" "${compile_args[@]}"

  python3 - "${WORK_DIR}" <<'PY'
import os
import subprocess
import sys
import tempfile

work_dir = sys.argv[1]
for path in filter(None, os.environ["LINKER_FILES"].splitlines()):
    base = os.path.basename(path)
    stem = base[:-2] if base.endswith(".a") else os.path.splitext(base)[0]
    out = os.path.join(work_dir, stem + ".lib")
    with tempfile.TemporaryDirectory(dir=work_dir) as tmp:
        subprocess.run(["llvm-ar", "x", path], cwd=tmp, check=True)
        members = []
        for root, _, files in os.walk(tmp):
            for name in files:
                if name.endswith(".o") or name.endswith(".obj"):
                    members.append(os.path.join(root, name))
        subprocess.run(["llvm-lib", f"/OUT:{out}", *sorted(members)], check=True)
PY

  dll_path="${OUT_DIR}/inochi2d-c.dll"
  lib_path="${OUT_DIR}/inochi2d-c.lib"
  rm -f "${dll_path}" "${lib_path}"

  local link_args=(
    --shared
    -link-defaultlib-shared=false
    --mtriple="${TARGET_TRIPLE}"
    --linker=lld-link
    --mscrtlib=msvcrt
    -of="${dll_path}"
    "${top_obj}"
    "-L/IMPLIB:${lib_path}"
    "-L/LIBPATH:${RUNTIME_LIB_DIR}"
    "-L/LIBPATH:${XWIN_DIR}/crt/lib/x86_64"
    "-L/LIBPATH:${XWIN_DIR}/sdk/lib/um/x86_64"
    "-L/LIBPATH:${XWIN_DIR}/sdk/lib/ucrt/x86_64"
  )

  local lib
  for lib in "${WORK_DIR}"/*.lib; do
    link_args+=("${lib}")
  done

  "${REAL_LDC2}" "${link_args[@]}"
  test -f "${dll_path}"
  test -f "${lib_path}"
}

main() {
  if [[ ! -d "${SOURCE_DIR}" ]]; then
    echo "missing submodule: ${SOURCE_DIR}" >&2
    echo "run: git submodule update --init --recursive" >&2
    exit 1
  fi

  require_cmd cargo
  require_cmd cargo-xwin
  require_cmd "${REAL_DUB}"
  require_cmd "${REAL_LDC2}"
  require_cmd ldc-build-runtime
  require_cmd llvm-ar
  require_cmd llvm-lib
  require_cmd ninja
  require_cmd python3
  require_cmd cmake
  require_cmd clang
  require_cmd lld-link

  eval "$(cargo xwin env --target ${TARGET_TRIPLE})"

  if [[ ! -d "${XWIN_DIR}/crt/include" || ! -d "${XWIN_DIR}/sdk/lib/um/x86_64" ]]; then
    echo "missing cargo-xwin sysroot at ${XWIN_DIR}; run cargo xwin env --target ${TARGET_TRIPLE} once first" >&2
    exit 1
  fi

  ensure_host_gitver
  build_runtime_libs

  (
    cd "${SOURCE_DIR}"
    export PATH="${WRAPPER_DIR}:${HOME}/.cache/cargo-xwin:${PATH}"
    export DFLAGS="--mtriple=${TARGET_TRIPLE} --linker=lld-link --mscrtlib=msvcrt -link-defaultlib-shared=false"
    dub build --compiler="${REAL_LDC2}" --config=yesgl --arch=x86_64 --force >/dev/null 2>&1 || true
  )

  (
    cd "${SOURCE_DIR}"
    export PATH="${WRAPPER_DIR}:${HOME}/.cache/cargo-xwin:${PATH}"
    build_windows_inochi2d
  )

  file "${OUT_DIR}/inochi2d-c.dll" "${OUT_DIR}/inochi2d-c.lib"
}

main "$@"
