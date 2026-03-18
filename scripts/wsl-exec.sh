#!/bin/sh
# Passthrough runner for WSL - executes Windows .exe directly via WSL interop
# instead of using wine
exec "$@"
