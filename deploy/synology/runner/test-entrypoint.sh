#!/usr/bin/env bash
set -euo pipefail

ENTRYPOINT="${ENTRYPOINT_UNDER_TEST:-/usr/local/bin/oteryn-runner-entrypoint}"
TMP="$(mktemp -d)"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

make_dist() {
    local dist="$1"
    mkdir -p "$dist"
    cat > "$dist/config.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$@" > "$CAPTURE_PATH"
touch .runner
SH
    cat > "$dist/run.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'RUN_OK\n'
SH
    chmod 0755 "$dist/config.sh" "$dist/run.sh"
}

assert_line() {
    local expected="$1"
    local path="$2"
    grep -Fx -- "$expected" "$path" >/dev/null
}

# Organization registration must bind both group and strict custom label.
org="$TMP/org"
make_dist "$org/dist"
CAPTURE_PATH="$org/args" \
RUNNER_DIST_DIR="$org/dist" \
RUNNER_CONFIG_DIR="$org/config" \
RUNNER_WORKDIR="$org/work" \
RUNNER_SCOPE=organization \
RUNNER_URL=https://github.com/Oteryn \
RUNNER_GROUP=atlas-runners \
RUNNER_NAME=oteryn-synology-atlas \
RUNNER_LABELS=oteryn-atlas \
RUNNER_TOKEN=test-token \
"$ENTRYPOINT" > "$org/out"
assert_line --runnergroup "$org/args"
assert_line atlas-runners "$org/args"
assert_line --labels "$org/args"
assert_line oteryn-atlas "$org/args"
assert_line --no-default-labels "$org/args"
assert_line RUN_OK "$org/out"

# Repository mode remains backward compatible and must not carry runnergroup.
repo="$TMP/repo"
make_dist "$repo/dist"
CAPTURE_PATH="$repo/args" \
RUNNER_DIST_DIR="$repo/dist" \
RUNNER_CONFIG_DIR="$repo/config" \
RUNNER_WORKDIR="$repo/work" \
RUNNER_SCOPE=repository \
RUNNER_URL=https://github.com/Oteryn/Oteryn-Platform \
RUNNER_NAME=oteryn-synology-staging \
RUNNER_LABELS=oteryn-staging \
RUNNER_TOKEN=test-token \
"$ENTRYPOINT" > "$repo/out"
if grep -Fx -- --runnergroup "$repo/args" >/dev/null; then
    echo 'repository registration unexpectedly received --runnergroup' >&2
    exit 1
fi
assert_line oteryn-staging "$repo/args"
assert_line RUN_OK "$repo/out"

# Organization registration fails closed without a group.
bad="$TMP/bad"
make_dist "$bad/dist"
if CAPTURE_PATH="$bad/args" \
   RUNNER_DIST_DIR="$bad/dist" \
   RUNNER_CONFIG_DIR="$bad/config" \
   RUNNER_WORKDIR="$bad/work" \
   RUNNER_SCOPE=organization \
   RUNNER_URL=https://github.com/Oteryn \
   RUNNER_NAME=oteryn-synology-atlas \
   RUNNER_LABELS=oteryn-atlas \
   RUNNER_TOKEN=test-token \
   "$ENTRYPOINT" > "$bad/out" 2> "$bad/err"; then
    echo 'organization registration without RUNNER_GROUP unexpectedly passed' >&2
    exit 1
fi
grep -F 'RUNNER_GROUP is required for organization scope' "$bad/err" >/dev/null

# A registered persistent config restarts without URL/token/re-registration.
restart="$TMP/restart"
mkdir -p "$restart/config" "$restart/work" "$restart/dist"
touch "$restart/config/.runner"
cat > "$restart/config/run.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'RESTART_OK\n'
SH
chmod 0755 "$restart/config/run.sh"
RUNNER_CONFIG_DIR="$restart/config" \
RUNNER_WORKDIR="$restart/work" \
RUNNER_DIST_DIR="$restart/dist" \
"$ENTRYPOINT" > "$restart/out"
assert_line RESTART_OK "$restart/out"

printf 'runner-entrypoint-contract=PASS\n'
