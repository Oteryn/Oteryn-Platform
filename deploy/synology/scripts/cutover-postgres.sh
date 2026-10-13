#!/usr/bin/env bash
# Staging cutover from MariaDB to PostgreSQL (ADR 0046; PLATFORM_POSTGRES_MIGRATION_PLAN.md, P6).
#
# One subcommand per plan step. A state file records every completed step and enforces the order, so a
# step cannot run early and the point of no return (plan step 4) cannot be crossed back.
#
#   preflight       preconditions (plan "Preconditions, checked by the script"); no stack change
#   freeze          step 1: closed-gate proof, external check, stop the stack, record the MariaDB-era witness
#   bootstrap       step 2: bootstrap-postgres.sh run (native admission off)
#   smoke           step 3: health-check.sh and the web smoke on the loopback origins
#   rollback        step 3 failure: back to MariaDB. Refused from step 4 on (forward-only)
#   native-reenable step 4: the P6a provenance-only init and attempt evidence; the LAST rollback point ends here
#   native-smoke    step 5: signing self-check and native login smoke
#   declare-done    step 6a: record the cutover done; writes gate-evidence.json (cutover_complete)
#   retire-mariadb  step 6b: DESTRUCTIVE (mariadb_data volume, mariadb container, MariaDB-era witness directory)
#   unfreeze        step 7: after maintenance-open: check the open edge, start the scheduler
#   status          prints the non-secret state record
#
# Every mutating command honours OTERYN_CUTOVER_DRY_RUN, which defaults to 1: it prints the planned
# commands and records nothing. Only OTERYN_CUTOVER_DRY_RUN=0 executes. Read-only checks always run.
#
# Inputs (environment; the workflow maps its dispatch inputs onto them):
#   OTERYN_CUTOVER_TASK_ID, OTERYN_CUTOVER_CLOSE_RUN_ID, OTERYN_CUTOVER_AUDIT_RUN_ID,
#   OTERYN_CUTOVER_OWNER_GO (preflight), OTERYN_OWNER_DELETE_CONFIRMATION, OTERYN_OWNER_DELETE_ACK.
# Audit and edge evidence are read from run artifacts through `gh api` and validated by pure jq
# functions; the script never reads a log and never trusts the operator's words for gate state.
#
# Exit status: 0 done (or dry run); 1 refused before any change; 2 failed after a change began.
set -euo pipefail
umask 077

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
ENV_FILE="${OTERYN_ENV_FILE:-$DEPLOY_DIR/.env}"
COMPOSE_FILE="${OTERYN_CUTOVER_COMPOSE_FILE:-$DEPLOY_DIR/compose.yml}"
CONTRACT_FILE="${OTERYN_RELEASE_CONTRACT_FILE:-$DEPLOY_DIR/release-contract.env}"

REQUIRED_REPOSITORY='Oteryn/Oteryn-Platform'
CLOUDFLARE_WORKFLOW_PATH='.github/workflows/cloudflare-oteryn-endpoints.yml'
CANONICAL_HOSTS=(oteryn.molehill.cloud gateway.molehill.cloud)
DELETE_PHRASE='DELETE-MARIADB-VOLUME'
AUDIT_MAX_AGE_SECONDS=86400

usage() {
    echo "Usage: cutover-postgres.sh {preflight|freeze|bootstrap|smoke|rollback|native-reenable|native-smoke|declare-done|retire-mariadb|unfreeze|status}" >&2
}

refuse() {
    printf 'Refused: %s\n' "$*" >&2
    exit 1
}

failed() {
    printf 'Cutover step failed: %s\n' "$*" >&2
    exit 2
}

subcommand="${1:-}"
case "$subcommand" in
    preflight | freeze | bootstrap | smoke | rollback | native-reenable | native-smoke | declare-done | retire-mariadb | unfreeze | status) ;;
    *) usage; exit 1 ;;
esac
(($# == 1)) || { usage; exit 1; }

dry_run=true
[[ "${OTERYN_CUTOVER_DRY_RUN:-1}" == 0 ]] && dry_run=false

# --- Pure functions (no I/O besides stdin/stdout; unit-testable) ------------------------------------

# valid_task_id <id>
valid_task_id() { [[ "$1" =~ ^OTERYN-[0-9]{8}-[a-z0-9][a-z0-9-]{0,80}$ ]]; }
# valid_run_id <id>
valid_run_id() { [[ "$1" =~ ^[1-9][0-9]{0,18}$ ]]; }

# check_run_provenance <expected-workflow-path>  (run JSON on stdin). Positive allowlist, as the P6b open check.
check_run_provenance() {
    jq -e --arg path "$1" '
        .path == $path and .event == "workflow_dispatch" and .head_branch == "main"
        and .status == "completed" and .conclusion == "success" and (.run_attempt | tostring) == "1"
    ' >/dev/null
}

# select_single_artifact <name>  (artifacts-list JSON on stdin): prints "<id> <created_at>" for exactly one
# unexpired artifact of that name, otherwise fails.
select_single_artifact() {
    local rows
    rows="$(jq -r --arg name "$1" '[.artifacts[]? | select(.name == $name)] as $all
        | if ($all | length) == 1 and ($all[0].expired == false) then "\($all[0].id) \($all[0].created_at)" else empty end')" || return 1
    [[ "$rows" =~ ^[0-9]+\ [0-9T:Z.+-]+$ ]] || return 1
    printf '%s\n' "$rows"
}

# artifact_age_ok <created_at> <now-epoch> <max-seconds>
artifact_age_ok() {
    local created
    created="$(date -u -d "$1" +%s 2>/dev/null)" || return 1
    (( $2 >= created && $2 - created < $3 ))
}

# validate_audit_json <expected-state> <audit-run-id> <task-id> <close-run-id-or-empty>  (audit.json on stdin).
# open: gate_state=open, closure_match=not_requested, other_origin_routes=0.
# closed: gate_state=closed, closure_match=yes, closure_run_id equal. No hostname but the canonical ones anywhere.
validate_audit_json() {
    jq -e --arg state "$1" --arg run "$2" --arg task "$3" --arg close "$4" '
        def hosts: [.. | strings | select(test("^[A-Za-z0-9-]+(\\.[A-Za-z0-9-]+)*\\.[A-Za-z]{2,}$"))
                    | select(. != "oteryn.molehill.cloud" and . != "gateway.molehill.cloud")];
        (.schema | type == "string" and length > 0)
        and (.audit_run_id | tostring) == $run
        and .task_id == $task
        and .gate_state == $state
        and ((.other_origin_routes | tostring) == "0")
        and (if $state == "open" then .closure_match == "not_requested"
             else .closure_match == "yes" and (.closure_run_id | tostring) == $close end)
        and ((del(.schema) | hosts) | length == 0)
    ' >/dev/null
}

# build_gate_evidence <task-id> <close-run-id> <outcome> <head-sha>: the platform-cutover-gate-evidence JSON.
build_gate_evidence() {
    case "$3" in cutover_complete | rollback_verified) ;; *) return 1 ;; esac
    [[ "$4" =~ ^[0-9a-f]{40}$ ]] || return 1
    jq -n --arg task "$1" --arg close "$2" --arg outcome "$3" --arg sha "$4" \
        '{schema: "oteryn.platform-cutover-gate-evidence.v1", task_id: $task, close_run_id: $close, outcome: $outcome, head_sha: $sha}'
}

# --- Inputs and environment ---------------------------------------------------------------------

task_id="${OTERYN_CUTOVER_TASK_ID:-}"
close_run_id="${OTERYN_CUTOVER_CLOSE_RUN_ID:-}"
audit_run_id="${OTERYN_CUTOVER_AUDIT_RUN_ID:-}"

state_root="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"
state_dir="${OTERYN_CUTOVER_STATE_DIR:-$state_root/postgres-cutover}"
bootstrap_state_dir="${OTERYN_PG_BOOTSTRAP_STATE_DIR:-$state_root/postgres-bootstrap}"
state_file="$state_dir/cutover-state.env"
lock_dir="$state_dir/run.lock"
evidence_dir="${OTERYN_CUTOVER_EVIDENCE_DIR:-$state_dir/evidence}"

[[ "$state_dir" == /* && "$state_root" == /* && "$bootstrap_state_dir" == /* && "$evidence_dir" == /* ]] \
    || refuse "state and evidence directories must be absolute paths."

env_get() {
    [[ -f "$ENV_FILE" ]] || return 0
    sed -n "s/^$1=//p" "$ENV_FILE" | tail -n 1 | tr -d '\r'
}

contract_engine() {
    local value
    [[ -f "$CONTRACT_FILE" ]] || { echo unreadable; return 0; }
    value="$(sed -n 's/^PLATFORM_DB_ENGINE=//p' "$CONTRACT_FILE" | tail -n 1 | tr -d '\r')"
    printf '%s\n' "${value:-mariadb}"
}

compose=(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")

# --- State record --------------------------------------------------------------------------------

has_step() { [[ -f "$state_file" ]] && grep -q "^STEP_$1=" "$state_file"; }
state_get() { [[ -f "$state_file" ]] && sed -n "s/^$1=//p" "$state_file" | tail -n 1 || true; }

# record_step NAME [KEY=value ...]: appends only in real mode.
record_step() {
    local name="$1"
    shift
    [[ "$dry_run" == false ]] || return 0
    {
        printf 'STEP_%s=%s\n' "$name" "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
        local kv
        for kv in "$@"; do printf '%s\n' "$kv"; done
    } >> "$state_file"
}

lock_held=false
release_lock() {
    rm -f -- "$state_dir/runtime.env" "$state_dir/runtime.env.tmp" 2>/dev/null || true
    [[ "$lock_held" != true ]] || rmdir -- "$lock_dir" 2>/dev/null || true
}

prepare_state() {
    [[ ! -L "$state_dir" ]] || refuse "the cutover state directory must not be a link."
    if [[ "$dry_run" == true ]]; then
        [[ -d "$state_dir" ]] || return 0
    else
        (umask 077; mkdir -p -- "$state_dir") || refuse "cannot create the cutover state directory."
        chmod 700 -- "$state_dir"
        trap release_lock EXIT
        mkdir -- "$lock_dir" 2>/dev/null || refuse "another cutover step holds $lock_dir (or a stale lock is left); remove it only when no step is running."
        lock_held=true
    fi
}

# result <key> <value>: result lines appear only after a real change was made and recorded.
result() { [[ "$dry_run" == true ]] || printf '%s=%s\n' "$1" "$2"; }

# use_runtime_env: the workflow writes the ephemeral environment with placeholder image references (as for a
# rollback). Compose steps need the images that really run, so a derived owner-only copy takes them from the
# current release record (parsed as data, never sourced). It is removed on exit. Real mode only.
use_runtime_env() {
    [[ "$dry_run" == false && "$(env_get PLATFORM_IMAGE)" == rollback-placeholder ]] || return 0
    local current="$state_root/current-release.env" key line value derived="$state_dir/runtime.env"
    local -A values=()
    [[ -f "$current" ]] || failed "the current release record is missing; the runtime images cannot be resolved."
    for key in RELEASE_SHA PLATFORM_SOURCE_SHA GATEWAY_SOURCE_SHA PLATFORM_IMAGE GATEWAY_IMAGE; do
        value="$(sed -n "s/^$key=//p" "$current" | tail -n 1 | tr -d '\r')"
        [[ -n "$value" ]] || failed "the current release record has no $key."
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
    done < "$ENV_FILE" > "$derived.tmp"
    chmod 600 "$derived.tmp"
    mv -- "$derived.tmp" "$derived"
    ENV_FILE="$derived"
    export OTERYN_ENV_FILE="$ENV_FILE"
    compose=(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")
}

# --- Command wrapper -----------------------------------------------------------------------------

# mutate <command...>: dry run prints the plan, a real run executes.
mutate() {
    if [[ "$dry_run" == true ]]; then
        printf 'PLAN:'
        printf ' %q' "$@"
        printf '\n'
        return 0
    fi
    "$@"
}

# --- Checks shared by the steps --------------------------------------------------------------------

require_tools() {
    local tool
    for tool in docker jq curl python3 git "$@"; do
        command -v "$tool" >/dev/null 2>&1 || refuse "required tool is missing: $tool"
    done
}

require_task() {
    valid_task_id "$task_id" || refuse "OTERYN_CUTOVER_TASK_ID must look like OTERYN-YYYYMMDD-slug."
    local recorded
    recorded="$(state_get TASK_ID)"
    [[ -z "$recorded" || "$recorded" == "$task_id" ]] || refuse "the state records another task; this task does not own the cutover state."
}

require_repository() {
    [[ "${GITHUB_REPOSITORY:-}" == "$REQUIRED_REPOSITORY" ]] || refuse "GITHUB_REPOSITORY must be exactly $REQUIRED_REPOSITORY."
    command -v gh >/dev/null 2>&1 || refuse "gh is required to read the audit artifact."
}

require_env_file() { [[ -f "$ENV_FILE" ]] || refuse "the staging environment file is missing."; }

require_close_run() {
    valid_run_id "$close_run_id" || refuse "OTERYN_CUTOVER_CLOSE_RUN_ID must be a numeric run id."
    local recorded
    recorded="$(state_get CLOSE_RUN_ID)"
    [[ -z "$recorded" || "$recorded" == "$close_run_id" ]] || refuse "the close run id differs from the one this cutover froze behind."
}

# fetch_audit_json <audit-run-id>: provenance, exactly one unexpired artifact, age, then prints audit.json.
fetch_audit_json() {
    local id="$1" run_json artifacts_json row artifact_id created tmp
    run_json="$(gh api "repos/$REQUIRED_REPOSITORY/actions/runs/$id")" || return 1
    check_run_provenance "$CLOUDFLARE_WORKFLOW_PATH" <<<"$run_json" || { echo "The audit run does not satisfy the provenance rules." >&2; return 1; }
    artifacts_json="$(gh api "repos/$REQUIRED_REPOSITORY/actions/runs/$id/artifacts")" || return 1
    row="$(select_single_artifact oteryn-edge-audit-result <<<"$artifacts_json")" || { echo "The audit run needs exactly one unexpired oteryn-edge-audit-result artifact." >&2; return 1; }
    artifact_id="${row%% *}"
    created="${row#* }"
    artifact_age_ok "$created" "$(date -u +%s)" "$AUDIT_MAX_AGE_SECONDS" || { echo "The audit artifact is older than 24 hours." >&2; return 1; }
    tmp="$(mktemp)"
    if ! gh api "repos/$REQUIRED_REPOSITORY/actions/artifacts/$artifact_id/zip" > "$tmp"; then
        rm -f "$tmp"
        return 1
    fi
    python3 -I -c 'import sys, zipfile
z = zipfile.ZipFile(sys.argv[1])
names = z.namelist()
if names != ["audit.json"]:
    sys.exit(1)
sys.stdout.buffer.write(z.read("audit.json"))' "$tmp" || { rm -f "$tmp"; return 1; }
    rm -f "$tmp"
}

# require_audit <open|closed>
require_audit() {
    local expected="$1" json close=''
    valid_run_id "$audit_run_id" || refuse "OTERYN_CUTOVER_AUDIT_RUN_ID must be a numeric run id."
    require_repository
    [[ "$expected" != closed ]] || { require_close_run; close="$close_run_id"; }
    json="$(fetch_audit_json "$audit_run_id")" || refuse "the audit evidence for run $audit_run_id could not be fetched and verified."
    validate_audit_json "$expected" "$audit_run_id" "$task_id" "$close" <<<"$json" \
        || refuse "the audit result does not show the required $expected gate state for this task."
    echo "Audit $audit_run_id verified: gate_state=$expected."
}

http_status() { curl -sS -o /dev/null -m 15 --max-redirs 0 -w '%{http_code}' "https://$1/" 2>/dev/null || printf '000'; }

# external_check_closed: both canonical hostnames answer the edge's 503; a random unused label gets no application response.
external_check_closed() {
    local host code label
    for host in "${CANONICAL_HOSTS[@]}"; do
        code="$(http_status "$host")"
        [[ "$code" == 503 ]] || refuse "the external check of a canonical hostname did not get the edge's 503 (got $code); the gate is not closed."
    done
    label="cutover-probe-$(od -An -N6 -tx1 /dev/urandom | tr -d ' \n').molehill.cloud"
    code="$(http_status "$label")"
    [[ ! "$code" =~ ^[23] ]] || refuse "a random unused hostname got an application-like response ($code)."
    echo "External check passed: both canonical hostnames return 503."
}

external_check_open() {
    local host code
    for host in "${CANONICAL_HOSTS[@]}"; do
        code="$(http_status "$host")"
        [[ "$code" =~ ^[1-9][0-9][0-9]$ && "$code" != 503 ]] || refuse "a canonical hostname does not reach its origin again (got $code)."
    done
    echo "External check passed: both canonical hostnames reach their origins."
}

marketplace_disabled() {
    local file="$state_root/marketplace.env" value
    if [[ -f "$file" ]]; then
        value="$(sed -n 's/^MARKETPLACE_ENABLED=//p' "$file" | tail -n 1 | tr -d '\r')"
    else
        value="$(env_get MARKETPLACE_ENABLED)"
        value="${value:-false}"
    fi
    [[ "$value" == false ]]
}

native_admission_off() { [[ "$(env_get OTERYN_NATIVE_ADMISSION_ENABLED)" != true ]]; }

head_sha() {
    local sha="${GITHUB_SHA:-}"
    local checked_out
    checked_out="$(git -C "$DEPLOY_DIR" rev-parse HEAD 2>/dev/null || true)"
    if [[ "$sha" =~ ^[0-9a-f]{40}$ && "$checked_out" =~ ^[0-9a-f]{40}$ && "$sha" != "$checked_out" ]]; then
        refuse "the checked-out tooling ($checked_out) differs from the dispatched SHA ($sha); evidence would name the wrong tooling."
    fi
    [[ "$sha" =~ ^[0-9a-f]{40}$ ]] || sha="$checked_out"
    [[ "$sha" =~ ^[0-9a-f]{40}$ ]] || refuse "cannot determine the head SHA for the gate evidence."
    printf '%s\n' "$sha"
}

# write_gate_evidence <outcome>: the only writer of gate-evidence.json.
write_gate_evidence() {
    local outcome="$1" sha tmp
    sha="$(head_sha)"
    mkdir -p -- "$evidence_dir"
    tmp="$evidence_dir/gate-evidence.json.tmp"
    build_gate_evidence "$task_id" "$close_run_id" "$outcome" "$sha" > "$tmp" || { rm -f "$tmp"; failed "cannot build the gate evidence."; }
    mv -- "$tmp" "$evidence_dir/gate-evidence.json"
    echo "gate_evidence_written=$outcome"
}

hook_command() {
    local name="$1" value="${!1:-}"
    [[ -n "$value" && "$value" == /* && -f "$value" && -x "$value" ]] \
        || refuse "$name must name an existing executable (absolute path); the step is refused while it is absent."
    printf '%s\n' "$value"
}

# --- Witness programs (POSIX sh, run in a one-off Platform container on the platform_storage volume) -----
# shellcheck disable=SC2016 # expanded by the witness shell, not here
WITNESS_PROGRAM='
set -eu
export LC_ALL=C
dir="${GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY:-}"
fail() { echo "witness_error=$1"; exit 3; }
case "$dir" in /*) ;; *) fail unset ;; esac
action="${1:-}"; aside="${2:-}"; expected_store_id="${3:-none}"
case "$aside" in "$dir".mariadb-era-*) ;; *) fail bad_aside ;; esac
[ "$aside" != "$dir" ] || fail bad_aside
store_id_of() {
    if [ -e "$1/witness-store.id" ]; then tr -d " \n\r\t" < "$1/witness-store.id"; else echo none; fi
}
case "$action" in
    restore)
        # Rollback only. The PostgreSQL directory must still be empty: a store id from the P6a init means a witnessed write.
        if [ ! -e "$aside" ] && [ ! -L "$aside" ]; then
            [ -d "$dir" ] && [ ! -L "$dir" ] && [ "$(store_id_of "$dir")" = "$expected_store_id" ] || fail aside_missing
            echo "witness_restored=already"; exit 0
        fi
        [ -d "$aside" ] && [ ! -L "$aside" ] || fail aside_not_directory
        [ "$(store_id_of "$aside")" = "$expected_store_id" ] || fail aside_changed
        if [ -e "$dir" ] || [ -L "$dir" ]; then
            [ -d "$dir" ] && [ ! -L "$dir" ] || fail live_not_directory
            for entry in "$dir"/.[!.]* "$dir"/..?* "$dir"/*; do
                if [ -e "$entry" ] || [ -L "$entry" ]; then fail postgres_witness_not_empty; fi
            done
            rmdir -- "$dir"
        fi
        mv -- "$aside" "$dir"
        echo "witness_restored=true"
        ;;
    delete-aside)
        # Retirement only: the MariaDB-era directory, never the live one.
        if [ ! -e "$aside" ] && [ ! -L "$aside" ]; then echo "witness_aside_deleted=already"; exit 0; fi
        [ -d "$aside" ] && [ ! -L "$aside" ] || fail aside_not_directory
        [ "$(store_id_of "$aside")" = "$expected_store_id" ] || fail aside_changed
        rm -rf -- "$aside"
        echo "witness_aside_deleted=true"
        ;;
    *) fail unknown_action ;;
esac
'

witness_program() {
    "${compose[@]}" run --rm --no-deps -T --entrypoint sh platform -c "$WITNESS_PROGRAM" witness "$@"
}

# moved_aside_path / moved_store_id: from the bootstrap state; empty when no directory was moved aside.
bootstrap_record_key() {
    local file="$bootstrap_state_dir/witness-moved-aside.env"
    [[ -f "$file" ]] || return 0
    sed -n "s/^$1=//p" "$file" | tail -n 1 | tr -d '\r'
}

valid_aside_path() { [[ "$1" =~ ^/[A-Za-z0-9._/-]+\.mariadb-era-[0-9TZ]+$ ]]; }
valid_store_id() { [[ "$1" =~ ^([0-9a-f]{64}|none)$ ]]; }

# --- Step implementations ---------------------------------------------------------------------------

do_status() {
    [[ -f "$state_file" ]] || { echo "No cutover state recorded."; return 0; }
    # The state file holds only step times, run ids, image references and non-secret ids.
    cat -- "$state_file"
}

do_preflight() {
    require_task
    require_tools
    require_env_file
    ! has_step FREEZE || refuse "the cutover already froze the stack; preflight cannot run again."
    [[ "${OTERYN_CUTOVER_OWNER_GO:-}" == "$task_id" ]] \
        || refuse "the owner go decision is missing: OTERYN_CUTOVER_OWNER_GO must name the task id (the owner's go and window)."
    [[ "$(contract_engine)" == mariadb ]] || refuse "the release contract must still declare PLATFORM_DB_ENGINE=mariadb before the freeze."
    marketplace_disabled || refuse "MARKETPLACE_ENABLED must be false."
    native_admission_off || refuse "native admission is on; it must be off before the cutover starts."
    local name count=0
    for name in PLATFORM_POSTGRES_SUPERUSER_PASSWORD PLATFORM_POSTGRES_OWNER_PASSWORD PLATFORM_POSTGRES_APP_PASSWORD PLATFORM_POSTGRES_BACKUP_PASSWORD PLATFORM_POSTGRES_DUMP_PASSWORD; do
        [[ -z "$(env_get "$name")" ]] || count=$((count + 1))
    done
    ((count == 5)) || refuse "all five PostgreSQL secrets must be configured ($count of 5)."
    hook_command OTERYN_CUTOVER_PROVENANCE_INIT_COMMAND >/dev/null
    hook_command OTERYN_CUTOVER_WEB_SMOKE_COMMAND >/dev/null
    hook_command OTERYN_CUTOVER_NATIVE_SMOKE_COMMAND >/dev/null
    [[ -f "$bootstrap_state_dir/witness-moved-aside.env" ]] \
        && refuse "the bootstrap state directory already records a moved-aside witness; archive it before a new cutover attempt."
    require_audit open
    prepare_state
    [[ -n "$(state_get TASK_ID)" || "$dry_run" == true ]] || printf 'TASK_ID=%s\n' "$task_id" >> "$state_file"
    record_step PREFLIGHT
    result preflight_result passed
}

do_freeze() {
    require_task
    require_tools
    require_env_file
    has_step PREFLIGHT || refuse "preflight has not passed for this cutover."
    ! has_step FREEZE || refuse "the freeze is already recorded."
    [[ "$(contract_engine)" == mariadb ]] || refuse "the release contract must still declare PLATFORM_DB_ENGINE=mariadb before the freeze."
    native_admission_off || refuse "native admission must be off before the freeze."
    marketplace_disabled || refuse "MARKETPLACE_ENABLED must be false."
    require_audit closed
    external_check_closed
    local project volume
    project="$(env_get COMPOSE_PROJECT_NAME)"
    project="${project:-oteryn-staging}"
    [[ "$project" =~ ^[a-z0-9][a-z0-9_-]{0,62}$ ]] || refuse "COMPOSE_PROJECT_NAME is not a simple project name."
    volume="${project}_mariadb_data"
    docker volume inspect "$volume" >/dev/null 2>&1 || refuse "the MariaDB data volume $volume does not exist; nothing to cut over from."
    prepare_state
    use_runtime_env
    mutate "${compose[@]}" stop gateway internal-proxy internal-mtls
    mutate "${compose[@]}" stop marketplace-scheduler
    mutate "${compose[@]}" stop platform mariadb
    mutate bash "${OTERYN_CUTOVER_BOOTSTRAP_SCRIPT:-$SCRIPT_DIR/bootstrap-postgres.sh}" record-witness
    record_step FREEZE "CLOSE_RUN_ID=$close_run_id" "MARIADB_VOLUME=$volume" "FREEZE_PLATFORM_IMAGE=$(env_get PLATFORM_IMAGE)" "FREEZE_GATEWAY_IMAGE=$(env_get GATEWAY_IMAGE)"
    result freeze_result done
}

do_bootstrap() {
    require_task
    require_tools
    require_env_file
    has_step FREEZE || refuse "the freeze is not recorded; bootstrap needs the stopped, frozen stack."
    ! has_step BOOTSTRAP || refuse "the bootstrap is already recorded."
    ! has_step REENABLE_STARTED || refuse "native re-enable already started; there is no re-run of the bootstrap."
    ! has_step ROLLBACK_STARTED || refuse "a rollback was started; this cutover can only end through it."
    require_close_run
    [[ "$(contract_engine)" == postgres ]] || refuse "the release contract must declare PLATFORM_DB_ENGINE=postgres for the bootstrap."
    native_admission_off || refuse "native admission must be off for the bootstrap."
    prepare_state
    use_runtime_env
    local out
    local complete_file="$bootstrap_state_dir/bootstrap-complete.env"
    if [[ -e "$complete_file" || -L "$complete_file" ]]; then
        # A previous run finished the bootstrap but stopped before the step was recorded: adopt its durable record.
        [[ -f "$complete_file" && ! -L "$complete_file" ]] || refuse "the bootstrap completion record is not a regular file."
        local c_oauth c_rust c_world
        c_oauth="$(sed -n 's/^OAUTH_CLIENT_ID=//p' "$complete_file" | tail -n 1)"
        c_rust="$(sed -n 's/^OAUTH_RUST_CLIENT_ID=//p' "$complete_file" | tail -n 1)"
        c_world="$(sed -n 's/^NATIVE_WORLD_ID=//p' "$complete_file" | tail -n 1)"
        grep -q '^BOOTSTRAPPED_AT=' "$complete_file" && [[ -n "$c_oauth" && -n "$c_rust" && -n "$c_world" ]] \
            || refuse "the bootstrap completion record is incomplete; inspect it or roll back."
        if [[ "$dry_run" == true ]]; then
            echo "PLAN: adopt the completed bootstrap record and record the BOOTSTRAP step"
            return 0
        fi
        record_step BOOTSTRAP "OAUTH_CLIENT_ID=$c_oauth" "OAUTH_RUST_CLIENT_ID=$c_rust" "NATIVE_WORLD_ID=$c_world" "BOOTSTRAP_PLATFORM_IMAGE=$(env_get PLATFORM_IMAGE)"
        return 0
    fi
    if [[ "$dry_run" == true ]]; then
        mutate bash "${OTERYN_CUTOVER_BOOTSTRAP_SCRIPT:-$SCRIPT_DIR/bootstrap-postgres.sh}" run
        return 0
    fi
    out="$(bash "${OTERYN_CUTOVER_BOOTSTRAP_SCRIPT:-$SCRIPT_DIR/bootstrap-postgres.sh}" run 2>&1)" || { printf '%s\n' "$out" >&2; failed "bootstrap-postgres.sh run failed; only rollback or a clean-slate re-run is possible, before step 4."; }
    printf '%s\n' "$out" | grep -E '^(bootstrap_result|oauth_client_id|oauth_rust_client_id|native_world_id|witness_moved_aside_to)=' || true
    local oauth rust world
    oauth="$(sed -n 's/^oauth_client_id=//p' <<<"$out" | tail -n 1)"
    rust="$(sed -n 's/^oauth_rust_client_id=//p' <<<"$out" | tail -n 1)"
    world="$(sed -n 's/^native_world_id=//p' <<<"$out" | tail -n 1)"
    record_step BOOTSTRAP "OAUTH_CLIENT_ID=$oauth" "OAUTH_RUST_CLIENT_ID=$rust" "NATIVE_WORLD_ID=$world" "BOOTSTRAP_PLATFORM_IMAGE=$(env_get PLATFORM_IMAGE)"
}

do_smoke() {
    require_task
    require_tools
    require_env_file
    has_step BOOTSTRAP || refuse "the bootstrap is not recorded."
    ! has_step REENABLE_STARTED || refuse "native re-enable already started; the step 3 smoke belongs to the rollback window."
    ! has_step ROLLBACK_STARTED || refuse "a rollback was started; this cutover can only end through it."
    require_close_run
    [[ "$(contract_engine)" == postgres ]] || refuse "the release contract must declare PLATFORM_DB_ENGINE=postgres."
    local web
    web="$(hook_command OTERYN_CUTOVER_WEB_SMOKE_COMMAND)"
    prepare_state
    use_runtime_env
    mutate bash "${OTERYN_CUTOVER_HEALTH_SCRIPT:-$SCRIPT_DIR/health-check.sh}"
    mutate "$web"
    record_step SMOKE
    result smoke_result passed
}

do_rollback() {
    require_task
    require_tools
    require_env_file
    has_step FREEZE || refuse "nothing to roll back: the freeze is not recorded."
    if has_step REENABLE_STARTED || has_step NATIVE_SMOKE || has_step DECLARED_DONE; then
        refuse "forward-only: native re-enable (plan step 4) has started, so there is no rollback to MariaDB and the PostgreSQL witness is never touched."
    fi
    require_close_run
    if has_step ROLLBACK_VERIFIED; then
        # Recovery: the rollback itself already ran; only the evidence is missing.
        [[ ! -f "$evidence_dir/gate-evidence.json" ]] || refuse "the rollback is already verified and its evidence exists."
        require_audit closed
        prepare_state
        if [[ "$dry_run" == true ]]; then
            echo "DRY RUN: would re-emit gate-evidence.json (rollback_verified) without repeating the rollback."
            return 0
        fi
        write_gate_evidence rollback_verified
        result rollback_result verified
        return 0
    fi
    [[ "$(contract_engine)" == mariadb ]] || refuse "revert the release contract to PLATFORM_DB_ENGINE=mariadb through the reviewed path before the rollback."
    require_audit closed
    local aside store_id
    aside="$(bootstrap_record_key MOVED_ASIDE_TO)"
    store_id="$(bootstrap_record_key WITNESS_STORE_ID)"
    if [[ -n "$aside" ]]; then
        valid_aside_path "$aside" && valid_store_id "$store_id" || refuse "the bootstrap moved-aside record is malformed."
    fi
    prepare_state
    use_runtime_env
    has_step ROLLBACK_STARTED || record_step ROLLBACK_STARTED
    mutate "${compose[@]}" stop gateway internal-proxy internal-mtls marketplace-scheduler platform
    if [[ -n "$aside" ]]; then
        mutate witness_program restore "$aside" "$store_id"
    else
        echo "No MariaDB-era witness directory was moved aside; nothing to restore."
    fi
    mutate "${compose[@]}" up -d mariadb
    mutate bash "${OTERYN_CUTOVER_ROLLBACK_SCRIPT:-$SCRIPT_DIR/rollback.sh}"
    mutate bash "${OTERYN_CUTOVER_HEALTH_SCRIPT:-$SCRIPT_DIR/health-check.sh}"
    if [[ "$dry_run" == true ]]; then
        echo "DRY RUN: no change made."
        return 0
    fi
    require_audit closed
    if [[ -d "$bootstrap_state_dir" ]]; then
        mv -- "$bootstrap_state_dir" "$bootstrap_state_dir.rolled-back-$(date -u +%Y%m%dT%H%M%SZ)"
    fi
    write_gate_evidence rollback_verified
    record_step ROLLBACK_VERIFIED
    result rollback_result verified
}

do_native_reenable() {
    require_task
    require_tools
    require_env_file
    has_step SMOKE || refuse "the step 3 smoke is not recorded; native re-enable is refused."
    ! has_step REENABLE_STARTED || refuse "native re-enable was already started."
    ! has_step ROLLBACK_STARTED || refuse "a rollback was started; this cutover can only end through it."
    native_admission_off || refuse "native admission must still be off when the provenance-only init runs; the deploy turns it on afterwards."
    require_close_run
    [[ "$(contract_engine)" == postgres ]] || refuse "the release contract must declare PLATFORM_DB_ENGINE=postgres."
    local hook
    hook="$(hook_command OTERYN_CUTOVER_PROVENANCE_INIT_COMMAND)"
    require_audit closed
    prepare_state
    if [[ "$dry_run" == true ]]; then
        mutate "$hook" init "$task_id" "$close_run_id"
        echo "DRY RUN: this step would end the rollback window; no change made."
        return 0
    fi
    # The marker comes first: from here on the cutover is forward-only, even if the init fails.
    record_step REENABLE_STARTED "REENABLE_CLOSE_RUN_ID=$close_run_id"
    "$hook" init "$task_id" "$close_run_id" || failed "the P6a provenance-only init failed. The cutover is forward-only now: keep native admission off and report to the owner."
    result native_reenable_result provenance_recorded
    echo "Next: turn native admission on through the deploy (publish-trust, then self-check), then run native-smoke."
}

do_native_smoke() {
    require_task
    require_tools
    require_env_file
    has_step REENABLE_STARTED || refuse "native re-enable has not started."
    ! has_step ROLLBACK_STARTED || refuse "a rollback was started; this cutover can only end through it."
    require_close_run
    [[ "$(env_get OTERYN_NATIVE_ADMISSION_ENABLED)" == true ]] || refuse "native admission is not on in the deployed environment; run the deploy that publishes trust first."
    local smoke
    smoke="$(hook_command OTERYN_CUTOVER_NATIVE_SMOKE_COMMAND)"
    prepare_state
    use_runtime_env
    mutate "${compose[@]}" exec -T platform /usr/local/bin/oteryn-platform-entrypoint php artisan game-auth:native-admission:self-check --no-interaction
    mutate "$smoke"
    record_step NATIVE_SMOKE
    result native_smoke_result passed
}

do_declare_done() {
    require_task
    require_tools
    require_env_file
    has_step NATIVE_SMOKE || refuse "the step 5 native login smoke is not recorded."
    ! has_step ROLLBACK_STARTED || refuse "a rollback was started; this cutover can only end through it."
    require_close_run
    require_audit closed
    prepare_state
    if has_step DECLARED_DONE; then
        [[ ! -f "$evidence_dir/gate-evidence.json" ]] || refuse "the cutover is already declared done and its evidence exists."
    fi
    if [[ "$dry_run" == true ]]; then
        echo "DRY RUN: would record the cutover done and write gate-evidence.json (cutover_complete)."
        return 0
    fi
    has_step DECLARED_DONE || record_step DECLARED_DONE
    write_gate_evidence cutover_complete
    result declare_done_result recorded
}

do_retire_mariadb() {
    require_task
    require_tools
    require_env_file
    # Entry gate 1: declared done, which needs the step 5 smoke.
    has_step NATIVE_SMOKE || refuse "the native login smoke (step 5) is not recorded; MariaDB is not retired."
    has_step DECLARED_DONE || refuse "the cutover is not declared done; MariaDB is not retired."
    ! has_step ROLLBACK_STARTED || refuse "a rollback was started; there is no retirement."
    ! has_step MARIADB_RETIRED || refuse "MariaDB is already retired."
    # Entry gate 2: the exact owner phrase and the workflow acknowledgement naming this task.
    [[ "${OTERYN_OWNER_DELETE_CONFIRMATION:-}" == "$DELETE_PHRASE" ]] \
        || refuse "the owner delete confirmation is missing or is not the exact phrase."
    [[ -n "$task_id" && "${OTERYN_OWNER_DELETE_ACK:-}" == "$task_id" ]] \
        || refuse "the owner delete acknowledgement must name this task id."
    require_close_run
    require_audit closed
    local volume aside store_id
    volume="$(state_get MARIADB_VOLUME)"
    [[ "$volume" =~ ^[a-z0-9][a-z0-9_-]*_mariadb_data$ ]] || refuse "the state records no valid MariaDB volume name."
    aside="$(bootstrap_record_key MOVED_ASIDE_TO)"
    store_id="$(bootstrap_record_key WITNESS_STORE_ID)"
    if [[ -n "$aside" ]]; then
        valid_aside_path "$aside" && valid_store_id "$store_id" || refuse "the bootstrap moved-aside record is malformed."
    fi
    docker volume inspect "$volume" >/dev/null 2>&1 || [[ "$dry_run" == true ]] || has_step RETIRE_STARTED \
        || refuse "the recorded MariaDB volume $volume does not exist."
    prepare_state
    use_runtime_env
    if [[ "$dry_run" == true ]]; then
        echo "DRY RUN (OTERYN_CUTOVER_DRY_RUN=1): all entry gates passed; nothing is executed or recorded. Planned commands:"
    else
        # Entry gate 3: the state marker, written before the first deletion so an interrupted run is visible.
        has_step RETIRE_STARTED || record_step RETIRE_STARTED
    fi
    mutate "${compose[@]}" rm -s -f mariadb
    if [[ "$dry_run" == true ]]; then
        mutate docker volume rm "$volume"
    elif docker volume inspect "$volume" >/dev/null 2>&1; then
        docker volume rm "$volume"
    else
        echo "The MariaDB volume is already absent (resumed run)."
    fi
    if [[ "$dry_run" == false && -n "$(docker ps -aq --filter "volume=$volume" 2>/dev/null)" ]]; then
        failed "a container still uses the MariaDB volume; nothing further was deleted."
    fi
    if [[ -n "$aside" ]]; then
        mutate witness_program delete-aside "$aside" "$store_id"
    fi
    [[ "$dry_run" == false ]] || return 0
    record_step MARIADB_RETIRED "MARIADB_DELETED_AT=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    result retire_mariadb_result done
    echo "note: do not run an ordinary staging deploy until P7 removes mariadb from the compose and deploy path; it would recreate the retired service."
    result mariadb_deleted_at "$(state_get MARIADB_DELETED_AT)"
}

do_unfreeze() {
    require_task
    require_tools
    require_env_file
    if has_step ROLLBACK_VERIFIED; then
        :
    else
        has_step DECLARED_DONE && has_step MARIADB_RETIRED \
            || refuse "the cutover is neither rolled back and verified nor declared done and retired; the gate stays closed."
    fi
    ! has_step UNFROZEN || refuse "the unfreeze is already recorded."
    require_audit open
    external_check_open
    prepare_state
    use_runtime_env
    mutate "${compose[@]}" up -d --no-deps marketplace-scheduler
    record_step UNFROZEN "UNFREEZE_AUDIT_RUN_ID=$audit_run_id"
    result unfreeze_result done
}

case "$subcommand" in
    status) do_status ;;
    preflight) do_preflight ;;
    freeze) do_freeze ;;
    bootstrap) do_bootstrap ;;
    smoke) do_smoke ;;
    rollback) do_rollback ;;
    native-reenable) do_native_reenable ;;
    native-smoke) do_native_smoke ;;
    declare-done) do_declare_done ;;
    retire-mariadb) do_retire_mariadb ;;
    unfreeze) do_unfreeze ;;
esac

[[ "$dry_run" == false || "$subcommand" == status ]] || echo "DRY RUN (OTERYN_CUTOVER_DRY_RUN is not 0): nothing was executed or recorded."
