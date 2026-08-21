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

expect_registration_failure() {
    local name="$1"
    local expected="$2"
    shift 2
    local root="$TMP/$name"
    make_dist "$root/dist"
    if env \
        CAPTURE_PATH="$root/args" \
        RUNNER_DIST_DIR="$root/dist" \
        RUNNER_CONFIG_DIR="$root/config" \
        RUNNER_WORKDIR="$root/work" \
        "$@" \
        "$ENTRYPOINT" > "$root/out" 2> "$root/err"; then
        echo "$name unexpectedly passed" >&2
        exit 1
    fi
    grep -F -- "$expected" "$root/err" >/dev/null
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
RUNNER_TOKEN=test-token \
"$ENTRYPOINT" > "$repo/out"
if grep -Fx -- --runnergroup "$repo/args" >/dev/null; then
    echo 'repository registration unexpectedly received --runnergroup' >&2
    exit 1
fi
assert_line oteryn-synology-staging "$repo/args"
assert_line oteryn-staging "$repo/args"
assert_line RUN_OK "$repo/out"

# Token-file registration keeps the token out of the runner process environment.
token_file="$TMP/one-time-token"
printf 'file-token\n' > "$token_file"
chmod 0600 "$token_file"
filemode="$TMP/filemode"
make_dist "$filemode/dist"
CAPTURE_PATH="$filemode/args" \
RUNNER_DIST_DIR="$filemode/dist" \
RUNNER_CONFIG_DIR="$filemode/config" \
RUNNER_WORKDIR="$filemode/work" \
RUNNER_SCOPE=organization \
RUNNER_URL=https://github.com/Oteryn \
RUNNER_GROUP=game-runners \
RUNNER_NAME=oteryn-synology-game \
RUNNER_LABELS=oteryn-game \
RUNNER_TOKEN_FILE="$token_file" \
"$ENTRYPOINT" > "$filemode/out"
assert_line file-token "$filemode/args"
assert_line RUN_OK "$filemode/out"

# Fail closed on malformed routing or registration identity.
expect_registration_failure bad-scope \
    'RUNNER_SCOPE must be exactly repository or organization' \
    RUNNER_SCOPE=team RUNNER_URL=https://github.com/Oteryn \
    RUNNER_GROUP=atlas-runners RUNNER_NAME=oteryn-synology-atlas \
    RUNNER_LABELS=oteryn-atlas RUNNER_TOKEN=test-token
expect_registration_failure org-repo-url \
    'RUNNER_URL must be an exact github.com organization URL for organization scope' \
    RUNNER_SCOPE=organization RUNNER_URL=https://github.com/Oteryn/Oteryn-Atlas \
    RUNNER_GROUP=atlas-runners RUNNER_NAME=oteryn-synology-atlas \
    RUNNER_LABELS=oteryn-atlas RUNNER_TOKEN=test-token
expect_registration_failure repo-org-url \
    'RUNNER_URL must be an exact github.com owner/repository URL for repository scope' \
    RUNNER_SCOPE=repository RUNNER_URL=https://github.com/Oteryn \
    RUNNER_TOKEN=test-token
expect_registration_failure org-missing-group \
    'RUNNER_GROUP is required for organization scope' \
    RUNNER_SCOPE=organization RUNNER_URL=https://github.com/Oteryn \
    RUNNER_NAME=oteryn-synology-atlas RUNNER_LABELS=oteryn-atlas RUNNER_TOKEN=test-token
expect_registration_failure org-missing-name \
    'RUNNER_NAME is required for organization scope' \
    RUNNER_SCOPE=organization RUNNER_URL=https://github.com/Oteryn \
    RUNNER_GROUP=atlas-runners RUNNER_LABELS=oteryn-atlas RUNNER_TOKEN=test-token
expect_registration_failure org-missing-label \
    'RUNNER_LABELS is required for organization scope' \
    RUNNER_SCOPE=organization RUNNER_URL=https://github.com/Oteryn \
    RUNNER_GROUP=atlas-runners RUNNER_NAME=oteryn-synology-atlas RUNNER_LABELS= \
    RUNNER_TOKEN=test-token
expect_registration_failure malformed-label \
    'RUNNER_LABELS must be a non-empty comma-separated list of strict custom labels' \
    RUNNER_SCOPE=organization RUNNER_URL=https://github.com/Oteryn \
    RUNNER_GROUP=atlas-runners RUNNER_NAME=oteryn-synology-atlas \
    'RUNNER_LABELS=oteryn-atlas,bad label' RUNNER_TOKEN=test-token
expect_registration_failure reserved-default-label \
    'RUNNER_LABELS must not recreate GitHub default self-hosted labels' \
    RUNNER_SCOPE=organization RUNNER_URL=https://github.com/Oteryn \
    RUNNER_GROUP=atlas-runners RUNNER_NAME=oteryn-synology-atlas \
    'RUNNER_LABELS=oteryn-atlas,self-hosted' RUNNER_TOKEN=test-token
expect_registration_failure repo-with-group \
    'RUNNER_GROUP is not valid for repository-scoped registration' \
    RUNNER_SCOPE=repository RUNNER_URL=https://github.com/Oteryn/Oteryn-Platform \
    RUNNER_GROUP=platform-runners RUNNER_TOKEN=test-token
expect_registration_failure missing-token-file \
    'RUNNER_TOKEN_FILE must reference a readable one-time registration token file' \
    RUNNER_SCOPE=organization RUNNER_URL=https://github.com/Oteryn \
    RUNNER_GROUP=atlas-runners RUNNER_NAME=oteryn-synology-atlas \
    RUNNER_LABELS=oteryn-atlas RUNNER_TOKEN_FILE="$TMP/does-not-exist"

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
