# shellcheck shell=bash
# Shared helpers for the PostgreSQL cutover hooks (cutover-postgres.sh, plan P6). Sourced, never executed.
#
# The hooks run on the deploy runner, called by cutover-postgres.sh with its environment. They never print
# the environment file, a password, a token or a private key, and never enable shell tracing. Every
# failure exits non-zero, so the cutover step fails closed.

HOOK_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
SCRIPTS_DIR="$(cd -- "$HOOK_DIR/.." && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPTS_DIR/.." && pwd)"
COMPOSE_FILE="${OTERYN_CUTOVER_COMPOSE_FILE:-$DEPLOY_DIR/compose.yml}"
REQUIRED_REPOSITORY='Oteryn/Oteryn-Platform'
TRUSTED_ANCHOR_AUTHOR_ID=41898282
TRUSTED_ANCHOR_AUTHOR_LOGIN='github-actions[bot]'

hook_fail() {
    printf '%s: %s\n' "${HOOK_NAME:-cutover-hook}" "$*" >&2
    exit 1
}

hook_require_tools() {
    local tool
    for tool in "$@"; do
        command -v "$tool" >/dev/null 2>&1 || hook_fail "required tool is missing: $tool"
    done
}

# hook_workdir: an owner-only scratch directory, removed on exit.
hook_workdir() {
    HOOK_WORK="$(mktemp -d "${TMPDIR:-/tmp}/oteryn-cutover-hook.XXXXXX")" || hook_fail "cannot create a scratch directory."
    chmod 700 "$HOOK_WORK"
    trap 'rm -rf -- "$HOOK_WORK"' EXIT
}

hook_env_get() {
    [[ -f "$HOOK_ENV_FILE" ]] || return 0
    sed -n "s/^$1=//p" "$HOOK_ENV_FILE" | tail -n 1 | tr -d '\r'
}

# hook_runtime_env: the environment file the stack really runs with. cutover-postgres.sh exports a derived
# copy for the smoke steps; for native-reenable it does not, and the workflow's file then carries placeholder
# image references. The same derivation (images from the current release record, parsed as data, never
# sourced) is applied into the owner-only scratch directory. Call hook_workdir first.
hook_runtime_env() {
    HOOK_ENV_FILE="${OTERYN_ENV_FILE:-$DEPLOY_DIR/.env}"
    [[ -f "$HOOK_ENV_FILE" ]] || hook_fail "the staging environment file is missing."
    [[ "$(hook_env_get PLATFORM_IMAGE)" == rollback-placeholder ]] || return 0
    local current="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}/current-release.env" key line value
    local derived="$HOOK_WORK/runtime.env"
    local -A values=()
    [[ -f "$current" ]] || hook_fail "the current release record is missing; the runtime images cannot be resolved."
    for key in RELEASE_SHA PLATFORM_SOURCE_SHA GATEWAY_SOURCE_SHA PLATFORM_IMAGE GATEWAY_IMAGE; do
        value="$(sed -n "s/^$key=//p" "$current" | tail -n 1 | tr -d '\r')"
        [[ -n "$value" ]] || hook_fail "the current release record has no $key."
        values[$key]="$value"
    done
    while IFS= read -r line || [[ -n "$line" ]]; do
        case "${line%%=*}" in
            OTERYN_RELEASE_SHA) line="OTERYN_RELEASE_SHA=${values[RELEASE_SHA]}" ;;
            PLATFORM_SOURCE_SHA) line="PLATFORM_SOURCE_SHA=${values[PLATFORM_SOURCE_SHA]}" ;;
            GATEWAY_SOURCE_SHA) line="GATEWAY_SOURCE_SHA=${values[GATEWAY_SOURCE_SHA]}" ;;
            PLATFORM_IMAGE) line="PLATFORM_IMAGE=${values[PLATFORM_IMAGE]}" ;;
            GATEWAY_IMAGE) line="GATEWAY_IMAGE=${values[GATEWAY_IMAGE]}" ;;
            GATEWAY_VERSION) line="GATEWAY_VERSION=sha-${values[GATEWAY_SOURCE_SHA]}" ;;
        esac
        printf '%s\n' "$line"
    done < "$HOOK_ENV_FILE" > "$derived"
    chmod 600 "$derived"
    HOOK_ENV_FILE="$derived"
}

hook_compose() {
    docker compose --env-file "$HOOK_ENV_FILE" -f "$COMPOSE_FILE" "$@"
}

# hook_artisan <command> [args...]: one console command in the running platform service, through the image
# entrypoint (which exports the key ids), as the deploy and the cutover script run them.
hook_artisan() {
    hook_compose exec -T platform /usr/local/bin/oteryn-platform-entrypoint php artisan "$@" --no-interaction
}

# hook_last_json: the last stdout line that is a JSON object (console commands may print a styled error after it).
hook_last_json() {
    python3 -I -c '
import json, sys
found = None
for line in sys.stdin:
    try:
        value = json.loads(line)
    except ValueError:
        continue
    if isinstance(value, dict):
        found = value
if found is None:
    sys.exit(1)
print(json.dumps(found, sort_keys=True, separators=(",", ":")))
'
}

# hook_observe <out-file> [--key-id=<kid>]: the read-only D2 observation; fails unless the result is observed.
hook_observe() {
    local out="$1" raw
    shift
    raw="$(hook_artisan game-auth:native-evidence:observe-trust --trust=fresh "$@")" \
        || hook_fail "the read-only trust observation failed."
    printf '%s\n' "$raw" | hook_last_json > "$out" || hook_fail "the trust observation printed no JSON."
    [[ "$(jq -r '.result' "$out")" == observed ]] || hook_fail "the trust observation did not observe."
}

hook_platform_port() {
    local port
    port="$(hook_env_get PLATFORM_PORT)"
    port="${port:-8000}"
    [[ "$port" =~ ^[1-9][0-9]{0,4}$ ]] || hook_fail "PLATFORM_PORT is not a port number."
    printf '%s\n' "$port"
}

hook_gateway_port() {
    local port
    port="$(hook_env_get GATEWAY_PORT)"
    port="${port:-8080}"
    [[ "$port" =~ ^[1-9][0-9]{0,4}$ ]] || hook_fail "GATEWAY_PORT is not a port number."
    printf '%s\n' "$port"
}

# hook_loopback_python <args...>: runs the Python program on stdin against the NAS loopback, as health-check.sh does.
hook_loopback_python() {
    docker run --rm --network host -i python:3.12-alpine python3 - "$@"
}

# --- Attempt evidence (P6a D3 to D5) --------------------------------------------------------------

hook_evidence_setup() {
    EVIDENCE_DIR="${OTERYN_ATTEMPT_EVIDENCE_DIR:-/var/lib/oteryn-attempt-evidence}"
    EVIDENCE_TOOL="${OTERYN_ATTEMPT_EVIDENCE_TOOL:-$SCRIPTS_DIR/attempt_evidence.py}"
    [[ "$EVIDENCE_DIR" == /* ]] || hook_fail "the attempt evidence directory must be an absolute path."
    [[ "${GITHUB_REPOSITORY:-}" == "$REQUIRED_REPOSITORY" ]] || hook_fail "GITHUB_REPOSITORY must be exactly $REQUIRED_REPOSITORY."
    ANCHOR_ISSUE="${OTERYN_CUTOVER_ANCHOR_ISSUE:-}"
    [[ "$ANCHOR_ISSUE" =~ ^[1-9][0-9]{0,9}$ ]] \
        || hook_fail "OTERYN_CUTOVER_ANCHOR_ISSUE must name the locked anchor issue (P6a D3, owner question O5)."
}

hook_evidence() {
    python3 -I "$EVIDENCE_TOOL" "$@"
}

# hook_fetch_anchors <out-file>: every comment of the anchor issue, all pages, with its author and the
# issue's locked state, in the format attempt_evidence.py reads. Fails unless the issue is locked.
hook_fetch_anchors() {
    local out="$1" locked
    locked="$(gh api "repos/$REQUIRED_REPOSITORY/issues/$ANCHOR_ISSUE" | jq -r '.locked')" \
        || hook_fail "cannot read the anchor issue."
    [[ "$locked" == true ]] || hook_fail "the anchor issue must be locked (P6a D3)."
    gh api --paginate "repos/$REQUIRED_REPOSITORY/issues/$ANCHOR_ISSUE/comments?per_page=100" \
        | jq -s '{locked: true, comments: [add // [] | .[] | {id, author: {id: .user.id, login: .user.login}, body}]}' \
        > "$out" || hook_fail "cannot read the anchor issue comments."
}

# hook_post_comment <body>: posts one comment and prints its id; fails unless the author is the trusted bot.
hook_post_comment() {
    local reply
    reply="$(gh api -X POST "repos/$REQUIRED_REPOSITORY/issues/$ANCHOR_ISSUE/comments" -f body="$1" 2>/dev/null \
        | jq -r '"\(.id) \(.user.id) \(.user.login)"')" || return 1
    [[ "$reply" =~ ^[1-9][0-9]*\ $TRUSTED_ANCHOR_AUTHOR_ID\ (.+)$ && "${BASH_REMATCH[1]}" == "$TRUSTED_ANCHOR_AUTHOR_LOGIN" ]] \
        || return 1
    printf '%s\n' "${reply%% *}"
}

# hook_post_anchor <anchor-json>: publishes one anchor and prints the comment id. Anchor retry (D3): a trusted
# comment that already holds the same anchor is reused and nothing is posted.
hook_post_anchor() {
    local anchor existing id attempt
    anchor="$(jq -cS . <<<"$1")" || hook_fail "malformed anchor."
    for attempt in 1 2 3; do
        hook_fetch_anchors "$HOOK_WORK/anchors-post.json"
        existing="$(jq -r --argjson a "$anchor" --argjson uid "$TRUSTED_ANCHOR_AUTHOR_ID" --arg login "$TRUSTED_ANCHOR_AUTHOR_LOGIN" '
            [.comments[] | select(.author.id == $uid and .author.login == $login)
             | select((.body | try fromjson catch null) == $a) | .id] | first // empty' "$HOOK_WORK/anchors-post.json")"
        if [[ -n "$existing" ]]; then
            printf '%s\n' "$existing"
            return 0
        fi
        if id="$(hook_post_comment "$anchor")"; then
            printf '%s\n' "$id"
            return 0
        fi
        ((attempt == 3)) || sleep 2
    done
    hook_fail "cannot post the anchor for seq $(jq -r .seq <<<"$anchor") of run $(jq -r .run_id <<<"$anchor"); the line stays unanchored and the cutover stops."
}
