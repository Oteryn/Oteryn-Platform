#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/.."
mode=${1:-with-mfa}
case "$mode" in
    with-mfa|without-mfa) ;;
    *) printf '%s\n' 'Usage: bash sec-headers/launch.sh [with-mfa|without-mfa]' >&2; exit 2 ;;
esac
export PORTAL_E2E_SEC_HEADERS_MODE="$mode"
export PORTAL_E2E_SCRATCH="${PORTAL_E2E_SCRATCH:-$(mktemp -d)}"
mkdir -p -m 700 -- "$PORTAL_E2E_SCRATCH"
export PORTAL_E2E_ROUTE_REPORT
PORTAL_E2E_ROUTE_REPORT="$(mktemp "$PORTAL_E2E_SCRATCH/public-routes.XXXXXXXX.json")"
if [[ "$mode" == 'without-mfa' ]]; then
    : "${PORTAL_E2E_BINARY:?Set PORTAL_E2E_BINARY to an absolute path to the built portal binary}"
    wrapper="$(mktemp -d "$PORTAL_E2E_SCRATCH/without-mfa.XXXXXXXX")"
    # The secondary outbox harness starts the binary with a restricted environment.
    ln -s -- "$(realpath -- "$PORTAL_E2E_BINARY")" "$wrapper/oteryn-portal"
    ln -s -- "$PWD/sec-headers/without-mfa.sh" "$wrapper/portal"
    export PORTAL_E2E_BINARY="$wrapper/portal"
fi
exec node local.mjs
