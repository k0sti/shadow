#!/usr/bin/env bash

runtime_host_backend_resolve() {
  local backend
  backend="${SHADOW_RUNTIME_HOST_BACKEND:-deno-core}"

  case "$backend" in
    deno-core)
      SHADOW_RUNTIME_HOST_BACKEND="$backend"
      SHADOW_RUNTIME_HOST_PACKAGE_ATTR="deno-core-smoke"
      SHADOW_RUNTIME_HOST_BINARY_NAME="deno-core-smoke"
      SHADOW_RUNTIME_HOST_OUTPUT_PREFIX="deno_core host-op ok:"
      ;;
    deno-runtime)
      SHADOW_RUNTIME_HOST_BACKEND="$backend"
      SHADOW_RUNTIME_HOST_PACKAGE_ATTR="deno-runtime-smoke"
      SHADOW_RUNTIME_HOST_BINARY_NAME="deno-runtime-smoke"
      SHADOW_RUNTIME_HOST_OUTPUT_PREFIX="deno_runtime ok:"
      ;;
    *)
      echo "runtime host backend: unknown backend: $backend" >&2
      return 1
      ;;
  esac
}
