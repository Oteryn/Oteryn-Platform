#!/usr/bin/env bash
# Sourced helper. The legacy game-engine service and its variables are
# optional: when the compose topology defines the service the current behaviour
# applies; when it does not, every related check, probe and image pin is
# skipped rather than validated.
#
# Topology is read from the compose file only (never from the environment), so
# a stale value cannot resurrect a removed service.

_OTERYN_ENGINE_TOPOLOGY_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

oteryn_compose_file() {
    printf '%s\n' "${OTERYN_COMPOSE_FILE:-$_OTERYN_ENGINE_TOPOLOGY_DIR/../compose.yml}"
}

# oteryn_engine_present [COMPOSE_FILE]: exit 0 when `services:` defines the legacy engine.
oteryn_engine_present() {
    local compose_file="${1:-$(oteryn_compose_file)}"
    [[ -f "$compose_file" ]] || { echo "Missing compose file: $compose_file" >&2; return 2; }
    awk '
        /^[^[:space:]#]/ { in_services = ($0 ~ /^services:[[:space:]]*(#.*)?$/) }
        in_services && /^  canary:[[:space:]]*(#.*)?$/ { found = 1 }
        END { exit found ? 0 : 1 }
    ' "$compose_file"
}

# oteryn_engine_enabled [COMPOSE_FILE]: prints 1 or 0 and fails closed on an unreadable file.
oteryn_engine_enabled() {
    local rc=0
    oteryn_engine_present "$@" || rc=$?
    case "$rc" in
        0) printf '1\n' ;;
        1) printf '0\n' ;;
        *) return "$rc" ;;
    esac
}
