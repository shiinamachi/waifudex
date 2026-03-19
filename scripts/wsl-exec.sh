#!/bin/sh
# Supervising runner for WSL. Keep the launched app as a child process so
# Ctrl+C in the dev terminal can terminate the full child process tree.

child_pid=""
child_uses_process_group=0
cleanup_ran=0

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

    if command -v taskkill.exe >/dev/null 2>&1; then
      taskkill.exe //PID "$child_pid" //T //F >/dev/null 2>&1 || true
    fi

    wait "$child_pid" 2>/dev/null || true
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

if command -v setsid >/dev/null 2>&1; then
  setsid "$@" &
  child_uses_process_group=1
else
  "$@" &
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
