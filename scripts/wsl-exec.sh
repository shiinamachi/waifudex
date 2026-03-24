#!/bin/sh
# Supervising runner for WSL. Keep the launched app as a child process so
# Ctrl+C in the dev terminal can terminate the full child process tree.

child_pid=""
child_uses_process_group=0
cleanup_ran=0
child_image_name=""
child_command=""
child_staged_copy=""

derive_child_image_name() {
  if [ "$#" -eq 0 ]; then
    return
  fi

  candidate="$(basename -- "$1")"
  case "$candidate" in
    *.exe) child_image_name="$candidate" ;;
  esac
}

prepare_child_command() {
  if [ "$#" -eq 0 ]; then
    return
  fi

  child_command="$1"
  candidate="$(basename -- "$1")"
  case "$candidate" in
    *.exe)
      dir="$(dirname -- "$1")"
      stem="${candidate%.exe}"
      child_staged_copy="${dir}/${stem}-runner-$$.exe"
      cp -- "$1" "$child_staged_copy"
      child_command="$child_staged_copy"
      child_image_name="$(basename -- "$child_staged_copy")"
      ;;
    *)
      derive_child_image_name "$1"
      ;;
  esac
}

kill_windows_image() {
  if [ -z "$child_image_name" ]; then
    return
  fi

  if command -v taskkill.exe >/dev/null 2>&1; then
    taskkill.exe /IM "$child_image_name" /T /F >/dev/null 2>&1 || true
  fi
}

cleanup_child() {
  if [ "$cleanup_ran" -eq 1 ]; then
    return
  fi
  cleanup_ran=1

  if [ -z "$child_pid" ]; then
    return
  fi

  if kill -0 "$child_pid" 2>/dev/null; then
    if [ "$child_uses_process_group" -eq 1 ]; then
      kill -TERM "-$child_pid" 2>/dev/null || true
    else
      kill -TERM "$child_pid" 2>/dev/null || true
    fi

    wait "$child_pid" 2>/dev/null || true
  fi

  kill_windows_image

  if [ -n "$child_staged_copy" ]; then
    rm -f -- "$child_staged_copy"
  fi
}

exit_after_cleanup() {
  status="$1"
  trap - EXIT
  cleanup_child
  exit "$status"
}

trap 'exit_after_cleanup 130' INT
trap 'exit_after_cleanup 143' TERM
trap 'cleanup_child' EXIT

prepare_child_command "$@"
kill_windows_image

command_path="$child_command"
shift

if command -v setsid >/dev/null 2>&1; then
  setsid "$command_path" "$@" &
  child_uses_process_group=1
else
  "$command_path" "$@" &
fi
child_pid=$!

if wait "$child_pid"; then
  child_status=0
else
  child_status=$?
fi

trap - EXIT INT TERM
cleanup_child
exit "$child_status"
