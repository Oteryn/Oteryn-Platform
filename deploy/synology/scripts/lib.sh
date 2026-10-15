#!/usr/bin/env bash

# Docker Hub index digests verified 2026-08-12. Health probes keep their exact
# historical behavior while the Docker boundary substitutes immutable identities.
OTERYN_HEALTH_ALPINE_IMAGE='alpine@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce'
OTERYN_HEALTH_PYTHON_IMAGE='python@sha256:6d43704baacd1bfbe7c295d7f13079d5d8104ed33568873133f8fc69980419df'

# Resolves a non-secret native-login staging setting without a hand-set GitHub repository variable.
# Precedence: the repository variable (passed as the second argument) wins; otherwise the committed
# deploy/synology/native-defaults.env value; otherwise empty (the native admission gate then fails
# closed with its own message). The file is read as data, never sourced.
oteryn_native_default() {
    local name="$1" current="${2:-}" file value
    if [[ -n "$current" ]]; then
        printf '%s' "$current"
        return 0
    fi
    if [[ ! "$name" =~ ^OTERYN_STAGING_NATIVE_[A-Z_]+$ ]]; then
        echo "Invalid native default name: $name" >&2
        return 1
    fi
    file="${OTERYN_NATIVE_DEFAULTS_FILE:-$(dirname "${BASH_SOURCE[0]}")/../native-defaults.env}"
    [[ -f "$file" ]] || return 0
    value="$(sed -n "s/^${name}=//p" "$file" | tail -n 1)"
    printf '%s' "${value%$'\r'}"
}

load_oteryn_env_file() {
    local env_file="$1"
    local line key value

    if [[ ! -f "$env_file" ]]; then
        echo "Missing staging environment file: $env_file" >&2
        return 1
    fi

    while IFS= read -r line || [[ -n "$line" ]]; do
        line="${line%$'\r'}"
        if [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]]; then
            continue
        fi
        if [[ "$line" != *=* ]]; then
            echo "Invalid staging environment line; expected KEY=VALUE." >&2
            return 1
        fi
        key="${line%%=*}"
        value="${line#*=}"
        if [[ ! "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
            echo "Invalid staging environment key: $key" >&2
            return 1
        fi
        printf -v "$key" '%s' "$value"
        export "$key"
    done < "$env_file"

    if [[ "${GITHUB_WORKFLOW:-}" == "Character Bazaar Staging Control" && "$(basename -- "$env_file")" == ".env" ]]; then
        case "${APP_URL:-}" in
            https://oteryn.molehill.cloud|http://127.0.0.1:8000) ;;
            *) echo "Character Bazaar public staging APP_URL must be the canonical origin or the exact legacy loopback value." >&2; return 1 ;;
        esac
        case "${SESSION_SECURE_COOKIE:-}" in
            true|false|'') ;;
            *) echo "Character Bazaar public staging SESSION_SECURE_COOKIE must be boolean." >&2; return 1 ;;
        esac
        APP_URL=https://oteryn.molehill.cloud
        SESSION_SECURE_COOKIE=true
        export APP_URL SESSION_SECURE_COOKIE
    fi

    oteryn_platform_db_engine_settings
    oteryn_resolve_platform_mtls_settings
    oteryn_resolve_game_node_route_settings
}

# Platform database engine (ADR 0046, docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md P7). The committed
# deploy/synology/release-contract.env declares PLATFORM_DB_ENGINE, read as data and never sourced. PostgreSQL is
# the only engine since the P6 cutover retired MariaDB; the value stays in the contract (and in every Platform
# image) so a rollback still refuses a MariaDB-era image (oteryn_require_image_db_engine).
oteryn_platform_db_engine() {
    local file value
    file="${OTERYN_RELEASE_CONTRACT_FILE:-$(dirname "${BASH_SOURCE[0]}")/../release-contract.env}"
    if [[ ! -f "$file" ]]; then
        echo "Missing release contract: $file" >&2
        return 1
    fi
    value="$(sed -n 's/^PLATFORM_DB_ENGINE=//p' "$file" | tail -n 1)"
    value="${value%$'\r'}"
    if [[ "$value" != postgres ]]; then
        echo "PLATFORM_DB_ENGINE must be postgres (MariaDB was retired by the P6 cutover)." >&2
        return 1
    fi
    printf '%s\n' "$value"
}

# Exports the engine and the Platform runtime connection that compose.yml maps into the platform and
# marketplace-scheduler services (x-platform-db-*): the runtime role only (oteryn_platform_app) with
# verify-full TLS against the stack CA; the owner, backup, dump and superuser passwords never reach a
# Platform container.
oteryn_platform_db_engine_settings() {
    local engine
    engine="$(oteryn_platform_db_engine)" || return 1
    OTERYN_PLATFORM_DB_CONNECTION=pgsql
    OTERYN_PLATFORM_DB_HOST=postgres
    OTERYN_PLATFORM_DB_PORT=5432
    OTERYN_PLATFORM_DB_USERNAME=oteryn_platform_app
    OTERYN_PLATFORM_DB_PASSWORD="${PLATFORM_POSTGRES_APP_PASSWORD:-}"
    OTERYN_PLATFORM_DB_SSLMODE=verify-full
    # P6a D8: on PostgreSQL the producer refuses a first trust publication without a fence that names an open,
    # anchored attempt intent (attempt_evidence.py).
    OTERYN_PLATFORM_FIRST_PUBLICATION_FENCE=required
    OTERYN_PLATFORM_DB_ENGINE="$engine"
    export OTERYN_PLATFORM_DB_ENGINE OTERYN_PLATFORM_DB_CONNECTION OTERYN_PLATFORM_DB_HOST \
        OTERYN_PLATFORM_DB_PORT OTERYN_PLATFORM_DB_USERNAME OTERYN_PLATFORM_DB_PASSWORD OTERYN_PLATFORM_DB_SSLMODE \
        OTERYN_PLATFORM_FIRST_PUBLICATION_FENCE
}

# Refuses (returns 1) a partial PostgreSQL secret set, and on the postgres engine anything but all five: the
# stack would otherwise be stopped and recreated with an empty runtime password. Call it right after
# load_oteryn_env_file in every script that mutates the stack, before the first mutation.
oteryn_require_postgres_secrets_for_engine() {
    local state
    state="$(oteryn_postgres_secret_state)" || return 1
    if [[ "${OTERYN_PLATFORM_DB_ENGINE:-}" == postgres && "$state" != configured ]]; then
        echo "PLATFORM_DB_ENGINE=postgres requires all five PostgreSQL secrets." >&2
        return 1
    fi
}

# The read-only cutover state check shared by oteryn_require_cutover_deploy_window and the Synology autostart
# repair (.github/workflows/repair-synology-autostart.yml), so both stay off the stack on the same evidence.
# Refuses (returns 1, "<refusal>: ..." on stderr) a linked, unsearchable or not owner-only cutover or bootstrap
# state directory, a state file that is not a regular file, cannot be proven absent or cannot be read, and a
# state text cutover-postgres.sh cannot have written (_oteryn_cutover_state_valid). Returns 3 without a
# message while $OTERYN_CUTOVER_STATE_LOCK (run.lock) exists, before the state is read: a running step may be
# mid-append. Returns 0 with the validated state text in OTERYN_CUTOVER_STATE_TEXT ("" only when the state
# file is provably absent).
# Usage: oteryn_read_cutover_state <refusal prefix>
oteryn_read_cutover_state() {
    local refused="$1" root state_dir bootstrap_dir state_file state_text=""
    OTERYN_CUTOVER_STATE_TEXT=""
    root="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"
    state_dir="${OTERYN_CUTOVER_STATE_DIR:-$root/postgres-cutover}"
    bootstrap_dir="${OTERYN_PG_BOOTSTRAP_STATE_DIR:-$root/postgres-bootstrap}"
    state_file="$state_dir/cutover-state.env"
    OTERYN_CUTOVER_STATE_LOCK="$state_dir/run.lock"
    # Checked before any marker is read: -d and -x follow a link, and the cutover and bootstrap scripts refuse
    # linked state directories, so a linked one is never inspected through its target.
    if [[ -L "$state_dir" || -L "$bootstrap_dir" ]]; then
        echo "$refused: a cutover or bootstrap state directory is a link ($state_dir, $bootstrap_dir)." >&2
        return 1
    fi
    if [[ -e "$state_dir" ]] && [[ ! -d "$state_dir" || ! -x "$state_dir" ]]; then
        echo "$refused: the cutover state directory $state_dir cannot be inspected." >&2
        return 1
    fi
    # Every record in these directories decides the gate, so an existing one must be trusted as
    # bootstrap-postgres.sh requires before it reads any record: owned by the caller (the workflows run the
    # deploy, the repair and the cutover as the same user), not group- or world-writable.
    local trusted_dir
    for trusted_dir in "$state_dir" "$bootstrap_dir"; do
        if [[ -e "$trusted_dir" ]] && ! _oteryn_state_dir_trusted "$trusted_dir"; then
            echo "$refused: the cutover or bootstrap state directory $trusted_dir must be an owner-only directory (owned by the deploying user, not group- or world-writable)." >&2
            return 1
        fi
    done
    if [[ -L "$state_file" ]] || [[ -e "$state_file" && ! -f "$state_file" ]]; then
        echo "$refused: the cutover state $state_file is not a regular file." >&2
        return 1
    fi
    # -e is false on any stat error, not only on a missing file: absence counts only below a searchable directory.
    if [[ ! -e "$state_file" ]] && ! _oteryn_state_path_absent "$state_file"; then
        echo "$refused: the cutover state $state_file cannot be inspected; restore access to its directory first." >&2
        return 1
    fi
    if [[ -e "$OTERYN_CUTOVER_STATE_LOCK" || -L "$OTERYN_CUTOVER_STATE_LOCK" ]]; then
        return 3
    fi
    if [[ -e "$state_file" ]]; then
        # The trailing "." keeps a missing final newline visible (command substitution strips newlines).
        if ! state_text="$(_oteryn_read_state_file "$state_file" && printf .)"; then
            echo "$refused: the cutover state $state_file cannot be read; restore its access first." >&2
            return 1
        fi
        state_text="${state_text%.}"
        if ! _oteryn_cutover_state_valid "$state_text"; then
            echo "$refused: the cutover state $state_file is empty, truncated or malformed; inspect it first." >&2
            return 1
        fi
    fi
    OTERYN_CUTOVER_STATE_TEXT="$state_text"
}

# Prints the first PostgreSQL bootstrap state record (witness-freeze.env, witness-moved-aside.env,
# bootstrap-complete.env) that exists in any form or whose absence cannot be proven, and returns 0; returns 1
# when all three are provably absent. The freeze writes witness-freeze.env (record-witness) after it stopped the
# stack and before STEP_FREEZE, so a freeze that exited in between (its run.lock removed on exit) is still seen;
# the verified cutover rollback moves the whole bootstrap state directory aside.
# Usage: oteryn_cutover_bootstrap_record
oteryn_cutover_bootstrap_record() {
    local root bootstrap_dir record
    root="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"
    bootstrap_dir="${OTERYN_PG_BOOTSTRAP_STATE_DIR:-$root/postgres-bootstrap}"
    for record in "$bootstrap_dir/witness-freeze.env" "$bootstrap_dir/witness-moved-aside.env" \
        "$bootstrap_dir/bootstrap-complete.env"; do
        if [[ -e "$record" || -L "$record" ]] || ! _oteryn_state_path_absent "$record"; then
            printf '%s\n' "$record"
            return 0
        fi
    done
    return 1
}

# Cutover deploy window (ADR 0046, plan P6, scripts/cutover-postgres.sh). An ordinary deploy must not touch the
# stack while the cutover owns it. Since P7 the engine is postgres only. Refuses (returns 1), read-only and
# before any stack change:
#   - a cutover state that exists but cannot be inspected (not a regular file, a link, a linked or unsearchable
#     state directory, as cutover-postgres.sh refuses a linked one, or a read error): a missed ROLLBACK_STARTED
#     or MARIADB_RETIRED would let the deploy race a rollback or recreate the retired service, so it fails
#     closed. Only an absent state file means no cutover step is recorded;
#   - a cutover or bootstrap state directory that is not owner-only (owned by the caller, not group- or
#     world-writable; the rule bootstrap-postgres.sh applies to its own), before any record in it is read;
#   - a cutover run.lock (a step running, or one interrupted before it recorded its marker, such as a freeze that
#     stopped the stack before STEP_FREEZE);
#   - a readable state file that cutover-postgres.sh cannot have written (empty, truncated, reordered,
#     contradictory or foreign; see _oteryn_cutover_step_allowed for the step model): it is never taken as
#     "no step recorded";
#   - the postgres engine before the clean-slate bootstrap finished: a merge-triggered deploy would migrate the
#     empty database, which the bootstrap then refuses, or cut the stack over without the freeze. The completion
#     record must be a readable regular file with a valid UTC BOOTSTRAPPED_AT (YYYY-MM-DDTHH:MM:SSZ, a real
#     date, as bootstrap-postgres.sh writes it) and nonempty OAUTH_CLIENT_ID, OAUTH_RUST_CLIENT_ID and
#     NATIVE_WORLD_ID, the rule the cutover-postgres.sh adoption path applies. The deploy after the bootstrap
#     (plan step 2) and every later one pass;
#   - the postgres engine once a cutover rollback started;
#   - the rollback window (bootstrap complete, native re-enable not started) for a release whose deploy would
#     move a PostgreSQL release into last-good-release.env: the cutover rollback runs rollback.sh, which
#     restores that file and refuses a PostgreSQL image there. Only the deploy that replaces the frozen MariaDB
#     release (plan step 2) and a redeploy of the current release pass, and a missing current-release.env is
#     refused. The current and pending candidate records count only after release-state.sh validate accepted
#     them (a link, a read error or an invalid record is refused);
#   - a compose file that defines the retired mariadb service once retire-mariadb started (RETIRE_STARTED, also
#     after an interrupted run) or finished (a reverted P7): it would recreate the retired service on an empty
#     volume.
# The paths follow cutover-postgres.sh (OTERYN_STATE_DIR, OTERYN_CUTOVER_STATE_DIR, OTERYN_PG_BOOTSTRAP_STATE_DIR).
# Usage: oteryn_require_cutover_deploy_window <compose file>
oteryn_require_cutover_deploy_window() {
    local compose_file="$1" root bootstrap_dir complete_file state_text complete_text state_rc=0
    root="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"
    bootstrap_dir="${OTERYN_PG_BOOTSTRAP_STATE_DIR:-$root/postgres-bootstrap}"
    complete_file="$bootstrap_dir/bootstrap-complete.env"
    oteryn_read_cutover_state "Deploy refused" || state_rc=$?
    case "$state_rc" in
        0) ;;
        # cutover-postgres.sh holds run.lock while a step runs; a lock left behind marks a step interrupted before
        # it recorded its marker (a freeze that stopped the stack before STEP_FREEZE).
        3)
            echo "Deploy refused: a cutover step holds or left $OTERYN_CUTOVER_STATE_LOCK; finish or inspect that step first." >&2
            return 1
            ;;
        *) return 1 ;;
    esac
    state_text="$OTERYN_CUTOVER_STATE_TEXT"
    case "${OTERYN_PLATFORM_DB_ENGINE:-}" in
        postgres)
            if [[ ! -f "$complete_file" || -L "$complete_file" ]]; then
                echo "Deploy refused: PLATFORM_DB_ENGINE=postgres but the PostgreSQL bootstrap is not complete ($complete_file); run the cutover (preflight, freeze, bootstrap) first." >&2
                return 1
            fi
            if ! complete_text="$(_oteryn_read_state_file "$complete_file")"; then
                echo "Deploy refused: the PostgreSQL bootstrap completion record $complete_file cannot be read; restore its access before deploying." >&2
                return 1
            fi
            # bootstrap-postgres.sh writes BOOTSTRAPPED_AT as `date -u +%Y-%m-%dT%H:%M:%SZ`: an empty or
            # malformed value is a torn or foreign record, never a finished bootstrap.
            if ! _oteryn_utc_timestamp_valid "$(_oteryn_state_value "$complete_text" BOOTSTRAPPED_AT)" \
                || [[ -z "$(_oteryn_state_value "$complete_text" OAUTH_CLIENT_ID)" ]] \
                || [[ -z "$(_oteryn_state_value "$complete_text" OAUTH_RUST_CLIENT_ID)" ]] \
                || [[ -z "$(_oteryn_state_value "$complete_text" NATIVE_WORLD_ID)" ]]; then
                echo "Deploy refused: the PostgreSQL bootstrap completion record $complete_file is incomplete (a valid UTC BOOTSTRAPPED_AT, YYYY-MM-DDTHH:MM:SSZ, and nonempty OAuth client and native world IDs required); inspect it or roll back." >&2
                return 1
            fi
            if _oteryn_cutover_step_recorded "$state_text" ROLLBACK_STARTED; then
                echo "Deploy refused: a cutover rollback to MariaDB was started; it ends only through cutover-postgres.sh rollback." >&2
                return 1
            fi
            if ! _oteryn_cutover_step_recorded "$state_text" REENABLE_STARTED; then
                _oteryn_cutover_keeps_rollback_release "$state_text" || return 1
            fi
            # RETIRE_STARTED is the durable mark of an interrupted retire-mariadb that may already have removed the
            # container or volume; a deploy then would recreate the service on an empty volume.
            # grep: 0 = the service is defined, 1 = it is not, anything else (unreadable file) fails closed.
            if _oteryn_cutover_step_recorded "$state_text" RETIRE_STARTED \
                || _oteryn_cutover_step_recorded "$state_text" MARIADB_RETIRED; then
                local compose_status=0
                grep -q '^  mariadb:' -- "$compose_file" || compose_status=$?
                case "$compose_status" in
                    0)
                        echo "Deploy refused: MariaDB retirement started or finished but the compose file still defines the mariadb service." >&2
                        return 1
                        ;;
                    1) ;;
                    *)
                        echo "Deploy refused: MariaDB retirement started or finished and the compose file $compose_file cannot be read to prove it no longer defines the mariadb service." >&2
                        return 1
                        ;;
                esac
            fi
            ;;
        *)
            echo "Deploy refused: the Platform database engine is not resolved." >&2
            return 1
            ;;
    esac
}

# Rollback window rule of oteryn_require_cutover_deploy_window. A deploy of another release copies
# current-release.env into last-good-release.env (and first finalizes a pending candidate the same way), so it
# passes only while the current release is the frozen MariaDB one (FREEZE_PLATFORM_IMAGE) and no candidate of
# another release is pending; a redeploy of the current release moves nothing, and a missing current record is
# refused.
# Usage: _oteryn_cutover_keeps_rollback_release <cutover state text>
_oteryn_cutover_keeps_rollback_release() {
    local state_text="$1" release_dir current_file candidate_file frozen_image current_sha current_image candidate_sha
    local requested="${OTERYN_RELEASE_SHA:-}"
    release_dir="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"
    current_file="$release_dir/current-release.env"
    candidate_file="$release_dir/candidate-release.env"
    if [[ -e "$candidate_file" || -L "$candidate_file" ]] || ! _oteryn_state_path_absent "$candidate_file"; then
        if ! read -r candidate_sha _ < <(_oteryn_release_record_identity "$candidate_file" "pending candidate"); then
            return 1
        fi
        if [[ "$candidate_sha" != "$requested" ]]; then
            echo "Deploy refused: the cutover rollback window is open and the pending candidate $candidate_sha is not the requested release; finishing it would replace the MariaDB rollback release in last-good-release.env. Redeploy the candidate release, roll back, or run native re-enable first." >&2
            return 1
        fi
    fi
    # Without the current record the deploy would not move the frozen release into last-good at all, so the
    # rollback could restore an older release: absence during the cutover is never taken as a first deployment.
    if [[ ! -e "$current_file" && ! -L "$current_file" ]]; then
        echo "Deploy refused: the cutover rollback window is open and $current_file is missing; the frozen MariaDB release must stay recoverable through last-good-release.env. Restore it, roll back, or run native re-enable first." >&2
        return 1
    fi
    if ! read -r current_sha current_image < <(_oteryn_release_record_identity "$current_file" "current release"); then
        return 1
    fi
    [[ "$current_sha" != "$requested" ]] || return 0
    frozen_image="$(_oteryn_state_value "$state_text" FREEZE_PLATFORM_IMAGE)"
    [[ -z "$frozen_image" || "$current_image" != "$frozen_image" ]] || return 0
    echo "Deploy refused: the cutover rollback window is open (bootstrap complete, native re-enable not started) and the current release $current_sha is not the frozen MariaDB release; deploying ${requested:-an unnamed release} would move it into last-good-release.env, which the cutover rollback restores. Redeploy the current release, roll back, or run native re-enable first." >&2
    return 1
}

# Prints "<RELEASE_SHA> <PLATFORM_IMAGE>" of a release record only after the full release-state.sh validator
# accepted it (the check rollback.sh applies to last-good-release.env); refuses a link, a read error or an
# invalid record, so a corrupt record is never evidence that the rollback release is kept.
# Usage: _oteryn_release_record_identity <file> <label>
_oteryn_release_record_identity() {
    local file="$1" label="$2" validator
    validator="$(dirname -- "${BASH_SOURCE[0]}")/release-state.sh"
    if [[ -L "$file" || ! -f "$file" ]]; then
        echo "Deploy refused: the $label $file in the cutover rollback window is not a regular file." >&2
        return 1
    fi
    if ! _oteryn_read_state_file "$file" >/dev/null; then
        echo "Deploy refused: the $label $file cannot be read in the cutover rollback window." >&2
        return 1
    fi
    if ! bash "$validator" validate "$file" >/dev/null 2>&1; then
        echo "Deploy refused: the $label $file is not a valid release record (release-state.sh validate); the cutover rollback window needs it intact." >&2
        return 1
    fi
    # The validator sources the record (release-state.sh writes every value with printf %q); read the values it
    # accepted the same way, in a subshell.
    (
        unset RELEASE_SHA PLATFORM_IMAGE
        # shellcheck disable=SC1090
        source -- "$file" >/dev/null 2>&1 || exit 1
        [[ "${RELEASE_SHA:-}" =~ ^[0-9a-f]{40}$ && "${PLATFORM_IMAGE:-}" =~ @sha256:[0-9a-f]{64}$ ]] || exit 1
        printf '%s %s\n' "$RELEASE_SHA" "$PLATFORM_IMAGE"
    ) || {
        echo "Deploy refused: the $label $file is not a valid release record; the cutover rollback window needs it intact." >&2
        return 1
    }
}

# True only when the path is provably absent: nothing is there and the nearest existing ancestor is a searchable
# directory. A stat error (an unsearchable or non-directory ancestor) is never taken as absence.
# Usage: _oteryn_state_path_absent <path>
_oteryn_state_path_absent() {
    local path="$1" dir
    [[ ! -e "$path" && ! -L "$path" ]] || return 1
    dir="$(dirname -- "$path")"
    while [[ ! -e "$dir" && ! -L "$dir" && "$dir" != / && "$dir" != . ]]; do
        dir="$(dirname -- "$dir")"
    done
    [[ -d "$dir" && -x "$dir" ]]
}

# The cutover state model, derived from scripts/cutover-postgres.sh. The script writes TASK_ID once, right
# before the first STEP_PREFLIGHT, and otherwise only appends a step's marker (STEP_<step>=<UTC time>) followed
# by exactly the keys that step records. Each marker is appended only when the step's guard holds for the
# markers already recorded:
#   PREFLIGHT          not FREEZE (repeatable until the freeze)
#   FREEZE             PREFLIGHT; once
#   BOOTSTRAP          FREEZE; once; not REENABLE_STARTED, not ROLLBACK_STARTED
#   SMOKE              BOOTSTRAP; not REENABLE_STARTED, not ROLLBACK_STARTED (repeatable)
#   ROLLBACK_STARTED   FREEZE; once; not ROLLBACK_VERIFIED; not REENABLE_STARTED, NATIVE_SMOKE or DECLARED_DONE
#   ROLLBACK_VERIFIED  ROLLBACK_STARTED; once; not REENABLE_STARTED, NATIVE_SMOKE or DECLARED_DONE
#   REENABLE_STARTED   SMOKE; once; not ROLLBACK_STARTED (forward-only from here)
#   NATIVE_SMOKE       REENABLE_STARTED; not ROLLBACK_STARTED (repeatable)
#   DECLARED_DONE      NATIVE_SMOKE; once; not ROLLBACK_STARTED
#   RETIRE_STARTED     NATIVE_SMOKE and DECLARED_DONE; once; not MARIADB_RETIRED, not ROLLBACK_STARTED
#   MARIADB_RETIRED    RETIRE_STARTED; once; not ROLLBACK_STARTED
#   UNFROZEN           ROLLBACK_VERIFIED, or DECLARED_DONE and MARIADB_RETIRED; once
# Usage: _oteryn_cutover_step_allowed <step> <recorded markers as " A B ... ">
_oteryn_cutover_step_allowed() {
    local step="$1" seen="$2"
    case "$step" in
        PREFLIGHT) [[ "$seen" != *" FREEZE "* ]] ;;
        FREEZE) [[ "$seen" == *" PREFLIGHT "* && "$seen" != *" FREEZE "* ]] ;;
        BOOTSTRAP)
            [[ "$seen" == *" FREEZE "* && "$seen" != *" BOOTSTRAP "* && "$seen" != *" REENABLE_STARTED "* \
                && "$seen" != *" ROLLBACK_STARTED "* ]] ;;
        SMOKE) [[ "$seen" == *" BOOTSTRAP "* && "$seen" != *" REENABLE_STARTED "* && "$seen" != *" ROLLBACK_STARTED "* ]] ;;
        ROLLBACK_STARTED)
            [[ "$seen" == *" FREEZE "* && "$seen" != *" ROLLBACK_STARTED "* && "$seen" != *" ROLLBACK_VERIFIED "* \
                && "$seen" != *" REENABLE_STARTED "* && "$seen" != *" NATIVE_SMOKE "* && "$seen" != *" DECLARED_DONE "* ]] ;;
        ROLLBACK_VERIFIED)
            [[ "$seen" == *" ROLLBACK_STARTED "* && "$seen" != *" ROLLBACK_VERIFIED "* && "$seen" != *" REENABLE_STARTED "* \
                && "$seen" != *" NATIVE_SMOKE "* && "$seen" != *" DECLARED_DONE "* ]] ;;
        REENABLE_STARTED)
            [[ "$seen" == *" SMOKE "* && "$seen" != *" REENABLE_STARTED "* && "$seen" != *" ROLLBACK_STARTED "* ]] ;;
        NATIVE_SMOKE) [[ "$seen" == *" REENABLE_STARTED "* && "$seen" != *" ROLLBACK_STARTED "* ]] ;;
        DECLARED_DONE)
            [[ "$seen" == *" NATIVE_SMOKE "* && "$seen" != *" DECLARED_DONE "* && "$seen" != *" ROLLBACK_STARTED "* ]] ;;
        RETIRE_STARTED)
            [[ "$seen" == *" NATIVE_SMOKE "* && "$seen" == *" DECLARED_DONE "* && "$seen" != *" RETIRE_STARTED "* \
                && "$seen" != *" MARIADB_RETIRED "* && "$seen" != *" ROLLBACK_STARTED "* ]] ;;
        MARIADB_RETIRED)
            [[ "$seen" == *" RETIRE_STARTED "* && "$seen" != *" MARIADB_RETIRED "* && "$seen" != *" ROLLBACK_STARTED "* ]] ;;
        UNFROZEN)
            [[ "$seen" != *" UNFROZEN "* ]] \
                && [[ "$seen" == *" ROLLBACK_VERIFIED "* || ( "$seen" == *" DECLARED_DONE "* && "$seen" == *" MARIADB_RETIRED "* ) ]] ;;
        *) return 1 ;;
    esac
}

# The keys record_step writes after a step's marker, in order (cutover-postgres.sh); every value is nonempty.
# PREFLIGHT's PRESERVED_RULES_HASH is optional on read (_oteryn_cutover_state_valid): a state written before the
# hash existed carries the bare marker, and cutover-postgres.sh itself refuses such a state at its audit gates.
# Usage: _oteryn_cutover_step_keys <step>
_oteryn_cutover_step_keys() {
    case "$1" in
        PREFLIGHT) echo "PRESERVED_RULES_HASH" ;;
        FREEZE) echo "CLOSE_RUN_ID MARIADB_VOLUME FREEZE_PLATFORM_IMAGE FREEZE_GATEWAY_IMAGE" ;;
        BOOTSTRAP) echo "OAUTH_CLIENT_ID OAUTH_RUST_CLIENT_ID NATIVE_WORLD_ID BOOTSTRAP_PLATFORM_IMAGE" ;;
        REENABLE_STARTED) echo "REENABLE_CLOSE_RUN_ID" ;;
        MARIADB_RETIRED) echo "MARIADB_DELETED_AT" ;;
        UNFROZEN) echo "UNFREEZE_AUDIT_RUN_ID" ;;
        *) echo "" ;;
    esac
}

# True only for a state text cutover-postgres.sh can have written: newline-terminated, TASK_ID first, then
# step blocks (a marker with a UTC timestamp no earlier than the previous one, followed by exactly its keys),
# each step allowed by _oteryn_cutover_step_allowed for the markers before it. A readable but empty,
# truncated, reordered, contradictory or foreign file is never taken as "no step recorded".
# Usage: _oteryn_cutover_state_valid <state text>
_oteryn_cutover_state_valid() {
    local text="$1" seen=" " last_time="" line step time key optional=false
    local -a lines expected=()
    [[ "$text" == *$'\n' ]] || return 1
    mapfile -t lines <<<"${text%$'\n'}"
    [[ "${lines[0]:-}" =~ ^TASK_ID=OTERYN-[0-9]{8}-[a-z0-9][a-z0-9-]{0,80}$ ]] || return 1
    for line in "${lines[@]:1}"; do
        # A bare legacy PREFLIGHT marker: the next line is already the next marker.
        if [[ "$optional" == true && "$line" == STEP_* ]]; then
            expected=()
        fi
        optional=false
        if ((${#expected[@]} > 0)); then
            key="${expected[0]}"
            [[ "$line" == "$key="?* ]] || return 1
            [[ "$key" != PRESERVED_RULES_HASH || "$line" =~ ^PRESERVED_RULES_HASH=[0-9a-f]{64}$ ]] || return 1
            expected=("${expected[@]:1}")
            continue
        fi
        [[ "$line" =~ ^STEP_([A-Z_]+)=([0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z)$ ]] || return 1
        step="${BASH_REMATCH[1]}"
        time="${BASH_REMATCH[2]}"
        [[ ! "$time" < "$last_time" ]] || return 1
        _oteryn_cutover_step_allowed "$step" "$seen" || return 1
        read -r -a expected <<<"$(_oteryn_cutover_step_keys "$step")"
        [[ "$step" != PREFLIGHT ]] || optional=true
        seen+="$step "
        last_time="$time"
    done
    # A state ending right after a legacy bare PREFLIGHT marker is complete too.
    [[ "$optional" == true ]] && expected=()
    ((${#expected[@]} == 0)) && [[ "$seen" == *" PREFLIGHT "* ]]
}

# True for a directory, not a link, owned by the caller and not group- or world-writable (bootstrap-postgres.sh
# state_dir_is_trusted).
# Usage: _oteryn_state_dir_trusted <dir>
_oteryn_state_dir_trusted() {
    local dir="$1" mode
    [[ -d "$dir" && ! -L "$dir" && -O "$dir" ]] || return 1
    mode="$(stat -c '%a' -- "$dir" 2>/dev/null || stat -f '%Lp' -- "$dir" 2>/dev/null)" || return 1
    [[ "$mode" =~ ^[0-7]+$ ]] && (( (8#$mode & 8#022) == 0 ))
}

# Prints a state file; fails on any read error (an unreadable file is never taken as an empty one).
_oteryn_read_state_file() {
    [[ -r "$1" ]] || return 1
    cat -- "$1"
}

# Usage: _oteryn_cutover_step_recorded <state text> <step>
_oteryn_cutover_step_recorded() {
    [[ $'\n'"$1" == *$'\n'"STEP_$2="* ]]
}

# True only for a UTC timestamp as bootstrap-postgres.sh writes it (`date -u +%Y-%m-%dT%H:%M:%SZ`) that names a
# real instant: the value must survive a round trip through date(1) unchanged (GNU -d, else BSD -j -f), so an
# impossible date such as 2026-02-30T00:00:00Z or 24:00:00 is refused. A date(1) that can parse neither way
# fails closed.
# Usage: _oteryn_utc_timestamp_valid <value>
_oteryn_utc_timestamp_valid() {
    local value="$1" format='%Y-%m-%dT%H:%M:%SZ' parsed
    [[ "$value" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ ]] || return 1
    parsed="$(date -u -d "$value" "+$format" 2>/dev/null)" \
        || parsed="$(date -u -j -f "$format" "$value" "+$format" 2>/dev/null)" \
        || return 1
    [[ "$parsed" == "$value" ]]
}

# Prints the last value of KEY= in a state text (the cutover-postgres.sh `sed | tail -n 1` rule).
# Usage: _oteryn_state_value <state text> <key>
_oteryn_state_value() {
    local line value=""
    while IFS= read -r line; do
        [[ "$line" == "$2="* ]] && value="${line#"$2="}"
    done <<<"$1"
    printf '%s' "$value"
}

# PostgreSQL branches of the deploy, baseline, recovery and Marketplace scripts (plan P4 work 4 and 5). The
# database name and every role name are fixed identifiers (postgres/init/10-platform-roles.sh).
#
# Owner-role migration step: a one-off Platform container (`compose run --rm`, never the runtime container)
# runs the migrations on the pgsql_migrations connection as oteryn_platform_owner. The owner credentials
# reach only that container, by name from this shell's environment (`-e NAME`, so never in an argument);
# the runtime platform and marketplace-scheduler services keep the app role only. FastCGI is off in the
# one-off container: its entrypoint would otherwise start a second pool and replace the socket the
# running Platform shares with internal-mtls. The deploy.sh docker() wrapper below matches these exact
# arguments for the pre-migration dump and the schema-state bookkeeping.
OTERYN_POSTGRES_MIGRATE_ARGS=(
    run --rm --no-deps -T
    -e OTERYN_PLATFORM_FASTCGI_ENABLED=false
    -e DB_MIGRATIONS_USERNAME -e DB_MIGRATIONS_PASSWORD
    platform php artisan migrate --force --no-interaction --database=pgsql_migrations
)

# Usage: oteryn_platform_migrate_postgres <compose command...>
oteryn_platform_migrate_postgres() {
    if [[ -z "${PLATFORM_POSTGRES_OWNER_PASSWORD:-}" ]]; then
        echo "The PostgreSQL owner role password is not configured; refusing the migration step." >&2
        return 1
    fi
    DB_MIGRATIONS_USERNAME=oteryn_platform_owner DB_MIGRATIONS_PASSWORD="$PLATFORM_POSTGRES_OWNER_PASSWORD" \
        "$@" "${OTERYN_POSTGRES_MIGRATE_ARGS[@]}"
}

# Every relation (table, partitioned table, view, materialized view, foreign table, sequence) outside the
# system schemas of the Platform database: zero means a fresh, empty database.
OTERYN_POSTGRES_RELATION_COUNT_SQL="SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE c.relkind IN ('r', 'p', 'v', 'm', 'f', 'S') AND n.nspname <> 'information_schema' AND n.nspname !~ '^pg_'"

# Prints the relation count of the Platform PostgreSQL database (local superuser over the socket, peer).
# Usage: oteryn_postgres_relation_count <compose command...>
oteryn_postgres_relation_count() {
    local count
    count="$(_oteryn_postgres_local_sql "$@" oteryn_platform "$OTERYN_POSTGRES_RELATION_COUNT_SQL")" || return 1
    [[ "$count" =~ ^[0-9]+$ ]] || return 1
    printf '%s\n' "$count"
}

# Writes a custom-format dump of the Platform PostgreSQL database, taken by the read-only dump role over
# TLS verify-full (its password on standard input), and keeps it only after pg_restore --list has read the
# whole archive table of contents. The file is owner-only from creation.
# Usage: oteryn_postgres_dump_platform <compose command...> <output file>
oteryn_postgres_dump_platform() {
    local -a compose=("${@:1:$#-1}")
    local output="${*: -1}"
    if [[ -z "${PLATFORM_POSTGRES_DUMP_PASSWORD:-}" ]]; then
        echo "The PostgreSQL dump role password is not configured; refusing the dump." >&2
        return 1
    fi
    rm -f -- "$output.tmp"
    (umask 077 && : >"$output.tmp") || return 1
    if ! printf '%s\n' "$PLATFORM_POSTGRES_DUMP_PASSWORD" | "${compose[@]}" --profile postgres exec -T postgres sh -c '
        IFS= read -r PGPASSWORD
        export PGPASSWORD
        exec pg_dump --format=custom --no-password \
            --dbname="host=postgres port=5432 dbname=oteryn_platform user=oteryn_platform_dump sslmode=verify-full sslrootcert=/run/oteryn-postgres-tls/ca.crt connect_timeout=5"' \
        >"$output.tmp"; then
        rm -f -- "$output.tmp"
        echo "The PostgreSQL dump by oteryn_platform_dump failed." >&2
        return 1
    fi
    if [[ ! -s "$output.tmp" ]] \
        || ! "${compose[@]}" --profile postgres exec -T postgres pg_restore --list <"$output.tmp" >/dev/null; then
        rm -f -- "$output.tmp"
        echo "The PostgreSQL dump is not a readable custom-format archive." >&2
        return 1
    fi
    mv -- "$output.tmp" "$output"
}

# Prints the number of non-terminal Character Bazaar auctions (0 when the table does not exist yet).
# Usage: oteryn_postgres_non_terminal_auction_count <compose command...>
oteryn_postgres_non_terminal_auction_count() {
    local table_count pending
    table_count="$(_oteryn_postgres_local_sql "$@" oteryn_platform \
        "SELECT count(*) FROM pg_tables WHERE schemaname = 'public' AND tablename = 'character_auctions'")" || return 1
    [[ "$table_count" =~ ^[0-9]+$ ]] || return 1
    if [[ "$table_count" == 0 ]]; then
        printf '0\n'
        return 0
    fi
    pending="$(_oteryn_postgres_local_sql "$@" oteryn_platform \
        "SELECT count(*) FROM public.character_auctions WHERE status IN ('escrow_pending','active','settlement_pending','cancel_pending','recovery_required')")" || return 1
    [[ "$pending" =~ ^[0-9]+$ ]] || return 1
    printf '%s\n' "$pending"
}

# Restores a custom-format dump into a recreated Platform PostgreSQL database: drops it (closing every
# session), recreates it and its grants with the checked-out role script in --database-only mode (no
# password), restores every object as oteryn_platform_owner without the archive's owners or grants, then
# re-applies the grants. Runs as the local superuser over the socket (peer); no password is needed.
# Usage: oteryn_postgres_restore_platform <compose command...> <roles script> <dump file>
oteryn_postgres_restore_platform() {
    local -a compose=("${@:1:$#-2}")
    local roles_script="${*: -2:1}" dump_file="${*: -1}"
    local -a pg=("${compose[@]}" --profile postgres exec -T -u postgres postgres)
    _oteryn_postgres_local_sql "${compose[@]}" postgres 'DROP DATABASE IF EXISTS oteryn_platform WITH (FORCE)' >/dev/null || return 1
    "${pg[@]}" sh -s -- --database-only <"$roles_script" || return 1
    "${pg[@]}" pg_restore --exit-on-error --single-transaction --no-owner --no-privileges \
        --role=oteryn_platform_owner --dbname=oteryn_platform <"$dump_file" || return 1
    "${pg[@]}" sh -s -- --database-only <"$roles_script" || return 1
}

# The five PostgreSQL secrets (README: PostgreSQL service). The superuser password is used only by the
# image's first-start initdb; the four role passwords are applied by postgres/init/10-platform-roles.sh.
OTERYN_POSTGRES_SECRET_NAMES=(
    PLATFORM_POSTGRES_SUPERUSER_PASSWORD
    PLATFORM_POSTGRES_OWNER_PASSWORD
    PLATFORM_POSTGRES_APP_PASSWORD
    PLATFORM_POSTGRES_BACKUP_PASSWORD
    PLATFORM_POSTGRES_DUMP_PASSWORD
)

# Digest of the staged postgresql.conf and pg_hba.conf (the postgres service label that makes Compose
# recreate the container when they change). Refuses an unreadable file.
oteryn_postgres_config_digest() {
    local config_dir="${OTERYN_POSTGRES_CONFIG_DIR:-$(dirname "${BASH_SOURCE[0]}")/../postgres}" digest
    digest="$(cat -- "$config_dir/postgresql.conf" "$config_dir/pg_hba.conf" | sha256sum | awk '{print $1}')" || return 1
    [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || { echo "PostgreSQL configuration digest failed." >&2; return 1; }
    printf '%s\n' "$digest"
}

# Reconcile record. oteryn_reconcile_postgres applies the role passwords and the role split, which the running
# container does not reveal, so the fast deploy path trusts it only while this record still matches every input
# the reconciliation depends on: the five secrets (rotated passwords are applied only by the reconcile), the
# staged configuration digest, the role-setup script and the postgres service definition (image, probe
# environment). The record holds a random salt and a salted SHA-256 over those inputs, mode 0600 beside the
# release state, and is never printed. It is cleared before a reconcile starts and written only after it fully
# succeeds, so a failed or absent reconcile can never look current.
oteryn_postgres_reconcile_record_file() {
    printf '%s/postgres-reconcile.env\n' "${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"
}

_oteryn_postgres_reconcile_hash() {
    local salt="$1" name root compose_file roles_file
    root="${OTERYN_POSTGRES_CONFIG_DIR:-$(dirname "${BASH_SOURCE[0]}")/../postgres}"
    compose_file="${OTERYN_POSTGRES_COMPOSE_FILE:-$(dirname "${BASH_SOURCE[0]}")/../compose.yml}"
    roles_file="$root/init/10-platform-roles.sh"
    {
        printf 'salt=%s\n' "$salt"
        for name in "${OTERYN_POSTGRES_SECRET_NAMES[@]}"; do
            printf '%s=%s\n' "$name" "${!name:-}"
        done
        printf 'config=%s\n' "$(oteryn_postgres_config_digest)" || return 1
        printf 'roles=%s\n' "$(sha256sum < "$roles_file" | awk '{print $1}')" || return 1
        awk '/^  postgres:$/ {p = 1; next} p && /^  [a-z][a-z0-9-]*:$/ {exit} p' "$compose_file"
    } | sha256sum | awk '{print $1}'
}

# Succeeds only when the record exists, is a regular file and matches the current inputs.
oteryn_postgres_reconcile_is_current() {
    local file salt recorded current
    file="$(oteryn_postgres_reconcile_record_file)"
    [[ -f "$file" && ! -L "$file" ]] || return 1
    salt="$(sed -n 's/^SALT=//p' "$file" | head -n 1)"
    recorded="$(sed -n 's/^FINGERPRINT=//p' "$file" | head -n 1)"
    [[ "$salt" =~ ^[0-9a-f]{32}$ && "$recorded" =~ ^[0-9a-f]{64}$ ]] || return 1
    current="$(_oteryn_postgres_reconcile_hash "$salt")" || return 1
    [[ "$current" == "$recorded" ]]
}

oteryn_postgres_reconcile_clear() {
    rm -f -- "$(oteryn_postgres_reconcile_record_file)" "$(oteryn_postgres_reconcile_record_file).tmp"
}

oteryn_postgres_reconcile_record() {
    local file salt hash
    file="$(oteryn_postgres_reconcile_record_file)"
    salt="$(head -c 16 /dev/urandom | od -An -tx1 | tr -d ' \n')" || return 1
    [[ "$salt" =~ ^[0-9a-f]{32}$ ]] || return 1
    hash="$(_oteryn_postgres_reconcile_hash "$salt")" || return 1
    mkdir -p -- "$(dirname "$file")" || return 1
    ( umask 077; printf 'SALT=%s\nFINGERPRINT=%s\n' "$salt" "$hash" > "$file.tmp" ) \
        && chmod 600 "$file.tmp" && mv -- "$file.tmp" "$file" || return 1
}

# Seconds before expiry at which tls/init.sh renews the PostgreSQL server certificate (7 days; the same
# value as its `openssl x509 -checkend`, pinned by tests/ci/test_synology_postgres_contract.py).
OTERYN_POSTGRES_CERT_RENEWAL_SECONDS=604800

# True when the server pair the running postgres container serves is current by the same predicate tls-init
# applies (tls/init.sh postgres_certificate_is_current, run here inside the container through its
# --check-postgres-pair mode): outside the renewal window, verified by the CA copy next to it, certificate
# and key a matching pair, key owner-only for the postgres user. Only tls-init renews it, so a deploy that
# skips tls-init (the Platform fast path) must not run with a pair that is due or inconsistent. Anything the
# check cannot prove, such as a missing openssl, counts as not current and takes the full path.
oteryn_postgres_certificate_is_current() {
    local container="$1" init
    init="${OTERYN_TLS_INIT_SCRIPT:-$(dirname "${BASH_SOURCE[0]}")/../tls/init.sh}"
    [[ -f "$init" ]] || return 1
    docker exec -i -e POSTGRES_TLS_DIR=/run/oteryn-postgres-tls "$container" \
        sh -s -- --check-postgres-pair < "$init" >/dev/null 2>&1
}

# Prints `configured` when all five secrets are set, `absent` when none is; refuses a partial or malformed
# set. Each value is generated hex/alphanumeric of at least 24 characters, distinct from every other
# PostgreSQL secret. Values are never printed.
oteryn_postgres_secret_state() {
    local name value count=0 seen=' '
    for name in "${OTERYN_POSTGRES_SECRET_NAMES[@]}"; do
        value="${!name:-}"
        [[ -n "$value" ]] || continue
        count=$((count + 1))
        if [[ ! "$value" =~ ^[A-Za-z0-9_.-]{24,}$ ]]; then
            echo "$name must be at least 24 generated hex/alphanumeric characters." >&2
            return 1
        fi
        if [[ "$seen" == *" $value "* ]]; then
            echo "$name must be distinct from every other database secret." >&2
            return 1
        fi
        seen+="$value "
    done
    case "$count" in
        0) printf 'absent\n' ;;
        "${#OTERYN_POSTGRES_SECRET_NAMES[@]}") printf 'configured\n' ;;
        *)
            echo "PostgreSQL secrets are partially configured ($count of ${#OTERYN_POSTGRES_SECRET_NAMES[@]}); set all of them or none." >&2
            return 1
            ;;
    esac
}

# Runs one SQL text inside the postgres container as the postgres OS user over the local socket (peer).
_oteryn_postgres_local_sql() {
    local -a compose=("${@:1:$#-2}")
    local database="${*: -2:1}" sql="${*: -1}"
    "${compose[@]}" --profile postgres exec -T -u postgres postgres \
        psql -X -q -t -A -v ON_ERROR_STOP=1 -d "$database" -c "$sql"
}

# Runs one SQL text as a Platform role over TCP to `postgres` from inside the postgres container, with the
# given sslmode and the stack CA. The password travels on standard input, never in an argument or env flag.
_oteryn_postgres_role_sql() {
    local -a compose=("${@:1:$#-4}")
    local role="${*: -4:1}" password="${*: -3:1}" sslmode="${*: -2:1}" sql="${*: -1}"
    printf '%s\n' "$password" | "${compose[@]}" --profile postgres exec -T postgres sh -c '
        IFS= read -r PGPASSWORD
        export PGPASSWORD
        exec psql -X -w -q -t -A -v ON_ERROR_STOP=1 \
            "host=postgres port=5432 dbname=oteryn_platform user=$1 sslmode=$2 sslrootcert=/run/oteryn-postgres-tls/ca.crt connect_timeout=5" \
            -c "$3"' sh "$role" "$sslmode" "$sql"
}

# Starts the idle postgres service, re-applies pg_hba.conf, the database and the role split, and proves the
# P4 contract before the deploy continues: TLS-only logins, verify-full works, the runtime role cannot
# create objects, the runtime and dump roles own nothing and no Platform role has elevated attributes.
# The postgres engine requires all PostgreSQL secrets. Usage: oteryn_reconcile_postgres <compose command...>
oteryn_reconcile_postgres() {
    local -a compose=("$@")
    local state deadline status output container
    local timeout="${OTERYN_POSTGRES_READY_TIMEOUT_SECONDS:-300}"
    local roles="'oteryn_platform_owner','oteryn_platform_app','oteryn_platform_backup','oteryn_platform_dump'"

    state="$(oteryn_postgres_secret_state)" || return 1
    if [[ "$state" == absent ]]; then
        echo "PLATFORM_DB_ENGINE=postgres requires all PostgreSQL secrets." >&2
        return 1
    fi
    oteryn_postgres_reconcile_clear
    if [[ ! "$timeout" =~ ^[1-9][0-9]*$ ]]; then
        echo "OTERYN_POSTGRES_READY_TIMEOUT_SECONDS must be a positive integer." >&2
        return 1
    fi

    # tls-init has already run in this deploy, so the server certificate and bootstrap files are current.
    # A startup-only setting (listen_addresses, max_connections, hba_file, ...) changed in postgresql.conf
    # is ignored by pg_reload_conf(), and the volume content is not part of the Compose service definition.
    # The digest label makes `up` recreate the container exactly when the staged configuration changed.
    OTERYN_POSTGRES_CONFIG_DIGEST="$(oteryn_postgres_config_digest)" || return 1
    export OTERYN_POSTGRES_CONFIG_DIGEST
    "${compose[@]}" --profile postgres up -d --no-deps postgres || return 1

    # Ready means the final server (PID 1), not the image's socket-only first-start init server.
    deadline=$((SECONDS + timeout))
    until "${compose[@]}" --profile postgres exec -T -u postgres postgres sh -c \
        'pg_isready -q -h /var/run/postgresql && [ "$(head -n 1 "$PGDATA/postmaster.pid")" = 1 ]' >/dev/null 2>&1; do
        if (( SECONDS >= deadline )); then
            echo "PostgreSQL did not become ready within ${timeout} seconds." >&2
            "${compose[@]}" --profile postgres ps postgres >&2 || true
            "${compose[@]}" --profile postgres logs --no-color --tail 100 postgres >&2 || true
            return 1
        fi
        sleep 2
    done

    _oteryn_postgres_local_sql "${compose[@]}" postgres 'SELECT pg_reload_conf()' >/dev/null || return 1
    printf '%s\n%s\n%s\n%s\n' \
        "$PLATFORM_POSTGRES_OWNER_PASSWORD" "$PLATFORM_POSTGRES_APP_PASSWORD" \
        "$PLATFORM_POSTGRES_BACKUP_PASSWORD" "$PLATFORM_POSTGRES_DUMP_PASSWORD" \
        | "${compose[@]}" --profile postgres exec -T -u postgres postgres sh /bootstrap/postgres-roles.sh || {
            echo "PostgreSQL role bootstrap failed." >&2
            return 1
        }

    container="$("${compose[@]}" --profile postgres ps -q postgres)" || return 1
    if [[ -z "$container" || "$(docker inspect --format '{{len .HostConfig.PortBindings}}' "$container")" != 0 ]]; then
        echo "The postgres service must exist and publish no host port." >&2
        return 1
    fi

    output="$(_oteryn_postgres_role_sql "${compose[@]}" oteryn_platform_app "$PLATFORM_POSTGRES_APP_PASSWORD" verify-full \
        'SELECT ssl FROM pg_stat_ssl WHERE pid = pg_backend_pid()' 2>&1)" || {
        echo "The runtime role cannot log in with sslmode=verify-full." >&2
        return 1
    }
    [[ "$output" == t ]] || { echo "The runtime role session is not encrypted." >&2; return 1; }
    if _oteryn_postgres_role_sql "${compose[@]}" oteryn_platform_app "$PLATFORM_POSTGRES_APP_PASSWORD" disable \
        'SELECT 1' >/dev/null 2>&1; then
        echo "PostgreSQL accepted a connection without TLS." >&2
        return 1
    fi
    output="$(_oteryn_postgres_role_sql "${compose[@]}" oteryn_platform_app "$PLATFORM_POSTGRES_APP_PASSWORD" verify-full \
        'BEGIN; CREATE TABLE oteryn_p4_privilege_probe (id integer); ROLLBACK' 2>&1)" && {
        echo "The runtime role can create tables in schema public." >&2
        return 1
    }
    [[ "$output" == *"permission denied for schema public"* ]] || {
        echo "The runtime role CREATE TABLE probe failed for an unexpected reason." >&2
        return 1
    }
    output="$(_oteryn_postgres_local_sql "${compose[@]}" oteryn_platform "
        SELECT (SELECT count(*) FROM pg_class c JOIN pg_roles r ON r.oid = c.relowner
                 WHERE r.rolname IN ('oteryn_platform_app', 'oteryn_platform_dump', 'oteryn_platform_backup'))
             + (SELECT count(*) FROM pg_roles WHERE rolname IN ($roles)
                 AND (rolsuper OR rolcreatedb OR rolcreaterole OR rolbypassrls))
             + (SELECT count(*) FROM pg_roles WHERE rolname IN ($roles)
                 AND rolreplication <> (rolname = 'oteryn_platform_backup'))
             + (SELECT count(*) FROM pg_namespace n JOIN pg_roles r ON r.oid = n.nspowner
                 WHERE n.nspname = 'public' AND r.rolname <> 'oteryn_platform_owner')
             + (SELECT count(*) FROM pg_database d JOIN pg_roles r ON r.oid = d.datdba
                 WHERE d.datname = 'oteryn_platform' AND r.rolname <> 'oteryn_platform_owner')
             + (SELECT count(*) FROM pg_class c WHERE c.relnamespace = 'public'::regnamespace AND c.relkind IN ('r', 'p')
                 AND (has_table_privilege('oteryn_platform_dump', c.oid, 'INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER')
                      OR has_table_privilege('oteryn_platform_app', c.oid, 'TRUNCATE,REFERENCES,TRIGGER')))
             + (SELECT count(*) FROM unnest(ARRAY['oteryn_platform_app', 'oteryn_platform_dump']) AS grantee
                 WHERE has_schema_privilege(grantee, 'public', 'CREATE'))
             + (SELECT count(*) FROM pg_proc p WHERE p.pronamespace = 'public'::regnamespace
                 AND (p.proacl IS NULL OR EXISTS (SELECT 1 FROM aclexplode(p.proacl) a
                                                  WHERE a.grantee = 0 AND a.privilege_type = 'EXECUTE')))
             + (SELECT count(*) FROM pg_type t LEFT JOIN pg_class c ON c.oid = t.typrelid
                 WHERE t.typnamespace = 'public'::regnamespace
                 AND (t.typtype IN ('e', 'd', 'r', 'm') OR (t.typtype = 'c' AND c.relkind = 'c'))
                 AND (t.typacl IS NULL OR EXISTS (SELECT 1 FROM aclexplode(t.typacl) a
                                                  WHERE a.grantee = 0 AND a.privilege_type = 'USAGE')))
             + (SELECT CASE WHEN EXISTS (SELECT 1 FROM pg_default_acl d JOIN pg_roles r ON r.oid = d.defaclrole
                                         WHERE r.rolname = 'oteryn_platform_owner' AND d.defaclobjtype = 'f'
                                         AND d.defaclnamespace = 0
                                         AND NOT EXISTS (SELECT 1 FROM aclexplode(d.defaclacl) a WHERE a.grantee = 0))
                            THEN 0 ELSE 1 END)
             + (SELECT CASE WHEN EXISTS (SELECT 1 FROM pg_default_acl d JOIN pg_roles r ON r.oid = d.defaclrole
                                         WHERE r.rolname = 'oteryn_platform_owner' AND d.defaclobjtype = 'T'
                                         AND d.defaclnamespace = 0
                                         AND NOT EXISTS (SELECT 1 FROM aclexplode(d.defaclacl) a WHERE a.grantee = 0))
                            THEN 0 ELSE 1 END)
             + (SELECT count(*) FROM unnest(ARRAY['oteryn_platform_app', 'oteryn_platform_dump']) AS grantee
                 WHERE has_database_privilege(grantee, 'oteryn_platform', 'CREATE,TEMPORARY'))")" || return 1
    [[ "$output" == 0 ]] || {
        echo "The PostgreSQL role split does not hold (ownership, role attributes, stale write privileges or PUBLIC function/type access)." >&2
        return 1
    }

    deadline=$((SECONDS + timeout))
    until status="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}' "$container")" \
        && [[ "$status" == healthy ]]; do
        if (( SECONDS >= deadline )); then
            echo "The postgres service did not become healthy (status: ${status:-unknown})." >&2
            return 1
        fi
        sleep 3
    done
    echo "Verified PostgreSQL: idle service healthy, TLS-only logins, role split in place."
    oteryn_postgres_reconcile_record || { echo "The PostgreSQL reconcile record could not be written." >&2; return 1; }
}

# Platform settings of the Game-node routes behind the mTLS listener that compose.yml maps into the
# platform service (native evidence, character bootstrap intents, Character Authority commands and
# web Character creation; README: Platform internal mTLS listener). The deploy workflow renders the
# environment file from a fixed key list without them, so each comes from the environment file (or the
# caller's environment) when set, otherwise from the committed non-secret OTERYN_STAGING_NATIVE_* value
# in deploy/synology/native-defaults.env, otherwise a fail-closed built-in: not activated or enabled,
# no client identity, no binding, no TTL, no source authority, so Platform answers those routes 503
# and the Create character form stays unavailable. Rate limits, the evidence source authority and the
# pending-intent bound are never empty (an empty value would replace the Platform default or make
# the command surface unavailable); the bound's built-in is 40, the U-CC9 enabled-backlog bound and
# the Platform code default (runbook CHARACTER_AUTHORITY_COMMANDS_ENABLEMENT.md). Client identities are
# certificate subject names, never secrets.
oteryn_resolve_game_node_route_settings() {
    local pair name default_name fallback value
    for pair in \
        GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED:EVIDENCE_ACTIVATED:false \
        GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY:EVIDENCE_MTLS_CLIENT_IDENTITY: \
        GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_PURPOSE:EVIDENCE_FRESH_ACCOUNT_PURPOSE: \
        GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_SCOPE:EVIDENCE_FRESH_ACCOUNT_SCOPE: \
        GAME_AUTH_NATIVE_EVIDENCE_CLOCK_UNCERTAINTY_SECONDS:EVIDENCE_CLOCK_UNCERTAINTY_SECONDS: \
        GAME_AUTH_NATIVE_EVIDENCE_SOURCE_AUTHORITY:EVIDENCE_SOURCE_AUTHORITY:platform \
        GAME_AUTH_NATIVE_EVIDENCE_REQUESTS_PER_MINUTE:EVIDENCE_REQUESTS_PER_MINUTE:120 \
        GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY:BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY: \
        GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_TTL_SECONDS:BOOTSTRAP_INTENT_TTL_SECONDS: \
        CHARACTER_AUTHORITY_COMMANDS_ENABLED:CHARACTER_COMMANDS_ENABLED:false \
        CHARACTER_AUTHORITY_COMMAND_IDENTITIES:CHARACTER_COMMAND_IDENTITIES: \
        CHARACTER_AUTHORITY_COMMAND_SOURCE_AUTHORITY:CHARACTER_COMMAND_SOURCE_AUTHORITY: \
        CHARACTER_AUTHORITY_COMMAND_MAX_PENDING_INTENTS:CHARACTER_COMMAND_MAX_PENDING_INTENTS:40 \
        CHARACTER_AUTHORITY_WEB_CREATE_ENABLED:CHARACTER_WEB_CREATE_ENABLED:false; do
        name="${pair%%:*}"
        default_name="OTERYN_STAGING_NATIVE_$(cut -d: -f2 <<<"$pair")"
        fallback="${pair#*:*:}"
        value="$(oteryn_native_default "$default_name" "${!name:-}")" || return 1
        printf -v "$name" '%s' "${value:-$fallback}"
        export "${name?}"
    done
    # Platform reads anything but a boolean here as a configuration error, also when it revokes an
    # identity's game authorizations, so a malformed switch refuses the run before any stack change.
    for name in GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED CHARACTER_AUTHORITY_COMMANDS_ENABLED CHARACTER_AUTHORITY_WEB_CREATE_ENABLED; do
        case "${!name}" in
            true|false) ;;
            *) echo "$name must be true or false." >&2; return 1 ;;
        esac
    done
    # Character Authority commands (Game contract §4.1, gate U-CC9: the enabled backlog is at most 40
    # unreceipted PENDING intents, and Platform reads a larger value as unavailable). Platform would read
    # an incomplete command surface as unavailable, so a switch that cannot work refuses the run instead
    # of deploying a silent 503 or an always-unavailable Create character form.
    if [[ ! "$CHARACTER_AUTHORITY_COMMAND_MAX_PENDING_INTENTS" =~ ^[1-9][0-9]?$ ]] \
        || (( CHARACTER_AUTHORITY_COMMAND_MAX_PENDING_INTENTS > 40 )); then
        echo "CHARACTER_AUTHORITY_COMMAND_MAX_PENDING_INTENTS must be an integer from 1 to 40." >&2
        return 1
    fi
    if [[ "$CHARACTER_AUTHORITY_COMMANDS_ENABLED" == true ]] \
        && [[ -z "$CHARACTER_AUTHORITY_COMMAND_IDENTITIES" || -z "$CHARACTER_AUTHORITY_COMMAND_SOURCE_AUTHORITY" ]]; then
        echo "CHARACTER_AUTHORITY_COMMANDS_ENABLED=true needs CHARACTER_AUTHORITY_COMMAND_IDENTITIES and CHARACTER_AUTHORITY_COMMAND_SOURCE_AUTHORITY." >&2
        return 1
    fi
    if [[ "$CHARACTER_AUTHORITY_WEB_CREATE_ENABLED" == true && "$CHARACTER_AUTHORITY_COMMANDS_ENABLED" != true ]]; then
        echo "CHARACTER_AUTHORITY_WEB_CREATE_ENABLED=true needs CHARACTER_AUTHORITY_COMMANDS_ENABLED=true." >&2
        return 1
    fi
}

# Platform internal mTLS listener for the Game node (compose.yml internal-mtls, README: Platform
# internal mTLS listener). Each setting comes from the environment file (or the caller's environment)
# when set, otherwise from the committed non-secret OTERYN_STAGING_NATIVE_PLATFORM_MTLS_* value in
# deploy/synology/native-defaults.env, otherwise a fail-closed built-in (listener off, loopback). The
# three file settings are host paths of PUBLIC certificates and of the server key file; the key
# bytes are never read by these scripts and nothing here is a secret.
oteryn_resolve_platform_mtls_settings() {
    local pair name default_name fallback value
    for pair in \
        OTERYN_PLATFORM_MTLS_ENABLED:ENABLED:false \
        PLATFORM_MTLS_BIND_ADDRESS:BIND_ADDRESS:127.0.0.1 \
        PLATFORM_MTLS_PORT:PORT:8543 \
        PLATFORM_MTLS_SERVER_CERT_FILE:SERVER_CERT_FILE:/volume1/oteryn/preprod-platform-tls/platform-internal-preprod.fullchain.pem \
        PLATFORM_MTLS_SERVER_KEY_FILE:SERVER_KEY_FILE:/volume1/oteryn/preprod-platform-tls/platform-internal-preprod.key \
        PLATFORM_MTLS_CLIENT_CA_FILE:CLIENT_CA_FILE:/volume1/oteryn/preprod-platform-tls/oteryn-preprod-ca.crt; do
        name="${pair%%:*}"
        default_name="OTERYN_STAGING_NATIVE_PLATFORM_MTLS_$(cut -d: -f2 <<<"$pair")"
        fallback="${pair#*:*:}"
        value="$(oteryn_native_default "$default_name" "${!name:-}")" || return 1
        printf -v "$name" '%s' "${value:-$fallback}"
        export "${name?}"
    done
}

# Refuses malformed listener settings (deploy.sh, before any stack change). Off needs nothing else.
oteryn_platform_mtls_settings_valid() {
    local name value
    case "${OTERYN_PLATFORM_MTLS_ENABLED:-}" in
        false) return 0 ;;
        true) ;;
        *) echo "OTERYN_PLATFORM_MTLS_ENABLED must be true or false." >&2; return 1 ;;
    esac
    if ! bash "$(dirname "${BASH_SOURCE[0]}")/validate-ipv4.sh" "${PLATFORM_MTLS_BIND_ADDRESS:-}" private-or-loopback; then
        echo "PLATFORM_MTLS_BIND_ADDRESS must be loopback or an exact private LAN IPv4 address." >&2
        return 1
    fi
    if [[ ! "${PLATFORM_MTLS_PORT:-}" =~ ^[1-9][0-9]{0,4}$ ]] || ((PLATFORM_MTLS_PORT > 65535)); then
        echo "PLATFORM_MTLS_PORT must be an integer in 1..65535." >&2
        return 1
    fi
    for name in PLATFORM_MTLS_SERVER_CERT_FILE PLATFORM_MTLS_SERVER_KEY_FILE PLATFORM_MTLS_CLIENT_CA_FILE; do
        value="${!name:-}"
        if [[ ! "$value" =~ ^/[A-Za-z0-9._/-]+$ || "$value" == *..* || "$value" == */ ]]; then
            echo "$name must be an absolute host file path." >&2
            return 1
        fi
    done
}

# True when a Platform image carries the FastCGI pool the listener needs (php-fpm and the pool file
# from platform.Dockerfile). Images built before the listener have neither: status 3. Any other
# non-zero status means the image could not be inspected.
oteryn_platform_image_has_fastcgi() {
    local image="$1"
    [[ -n "$image" ]] || return 2
    docker run --rm --network none --entrypoint sh "$image" \
        -c 'if command -v php-fpm >/dev/null && test -f /usr/local/etc/php-fpm.d/oteryn-internal-mtls.conf; then exit 0; fi; exit 3' \
        >/dev/null 2>&1
}

# The listener setting a given Platform image can serve: the resolved setting, except false for an
# image built before the listener (legacy path, deploy.sh finalize_previous_candidate_if_healthy).
# Fails when the image cannot be inspected.
oteryn_platform_mtls_flag_for_image() {
    local image="$1" rc=0
    [[ "${OTERYN_PLATFORM_MTLS_ENABLED:-false}" == true ]] || { echo false; return 0; }
    oteryn_platform_image_has_fastcgi "$image" || rc=$?
    case "$rc" in
        0) echo true ;;
        3) echo false ;;
        *) echo "Unable to inspect Platform image $image for the FastCGI pool." >&2; return 1 ;;
    esac
}

# Pins the listener setting into a derived environment file (the file's value wins when
# health-check.sh loads it).
oteryn_pin_platform_mtls_flag() {
    local env_file="$1" flag="$2"
    { grep -v '^OTERYN_PLATFORM_MTLS_ENABLED=' "$env_file" || true; printf 'OTERYN_PLATFORM_MTLS_ENABLED=%s\n' "$flag"; } >"$env_file.pin"
    chmod 600 "$env_file.pin"
    mv "$env_file.pin" "$env_file"
}

# Refuses, before any stack change, a Platform image that cannot serve the listener while it is on:
# the listener would accept the Game node's TLS session and answer 502 for every request. A no-op
# while the listener is off. Arguments: the Platform image reference (already pulled).
oteryn_platform_mtls_image_ready() {
    local image="$1"
    [[ "${OTERYN_PLATFORM_MTLS_ENABLED:-false}" == true ]] || return 0
    if ! oteryn_platform_image_has_fastcgi "$image"; then
        echo "Platform mTLS listener is on but Platform image $image has no FastCGI pool (built before the listener); set OTERYN_PLATFORM_MTLS_ENABLED=false for this image or use a newer image." >&2
        return 1
    fi
}

# Before any stack change, with the bootstrap files staged: prove the bind address exists on the host
# and that nginx accepts the staged configuration with the mounted server chain, key and client CA
# (a missing host file fails here, because Compose never creates the bind sources), and that
# PLATFORM_IMAGE carries the FastCGI pool. Arguments: the
# route's Compose command words. A no-op while the listener is off.
oteryn_platform_mtls_preflight() {
    (($# > 0)) || { echo "Platform mTLS preflight needs the Compose command." >&2; return 1; }
    [[ "${OTERYN_PLATFORM_MTLS_ENABLED:-false}" == true ]] || return 0
    oteryn_platform_mtls_image_ready "${PLATFORM_IMAGE:-}" || return 1
    if ! docker run --rm --network host alpine:3.22 ip -o -4 addr show \
        | awk -v address="$PLATFORM_MTLS_BIND_ADDRESS" '{ split($4, cidr, "/"); if (cidr[1] == address) found = 1 } END { exit found ? 0 : 1 }'; then
        echo "Platform mTLS bind address $PLATFORM_MTLS_BIND_ADDRESS is not an address of this host." >&2
        return 1
    fi
    oteryn_platform_mtls_certificate_valid || return 1
    if ! "$@" run --rm --no-deps -T --entrypoint nginx internal-mtls -t -q -c /bootstrap/internal-mtls-nginx.conf; then
        echo "Platform mTLS listener configuration or its host certificate files were refused (README: Platform internal mTLS listener)." >&2
        return 1
    fi
    echo "Platform mTLS listener preflight passed for ${PLATFORM_MTLS_BIND_ADDRESS}:${PLATFORM_MTLS_PORT}."
}

# The name the Game node verifies; the server certificate must carry it as a DNS SAN.
OTERYN_PLATFORM_MTLS_SERVER_NAME='platform-internal.preprod.oteryn.internal'

# Proves, before any ingress mutation (oteryn_platform_mtls_preflight), that the mounted server
# certificate will be accepted by the Game node: the chain verifies against the mounted CA, it is
# inside its validity period (not expired, not yet valid), it carries the DNS SAN
# OTERYN_PLATFORM_MTLS_SERVER_NAME, it is usable for server authentication and the key file matches
# it. nginx -t checks none of that. A real TLS 1.3 handshake runs in memory in a throwaway container
# with no network: the server side loads the chain and key (a key that does not match the
# certificate is refused there), the client side trusts only the mounted CA and checks the name.
# Only the verdict leaves the container; the key bytes never do. --mount fails on a missing file.
oteryn_platform_mtls_certificate_valid() {
    local verdict
    if verdict="$(docker run --rm --network none \
        --mount "type=bind,source=${PLATFORM_MTLS_SERVER_CERT_FILE},target=/m/server-fullchain.pem,readonly" \
        --mount "type=bind,source=${PLATFORM_MTLS_SERVER_KEY_FILE},target=/m/server.key,readonly" \
        --mount "type=bind,source=${PLATFORM_MTLS_CLIENT_CA_FILE},target=/m/ca.crt,readonly" \
        python:3.12-alpine python3 -I -c '
import ssl, sys
name = sys.argv[1]
try:
    server = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    server.minimum_version = ssl.TLSVersion.TLSv1_3
    server.load_cert_chain("/m/server-fullchain.pem", "/m/server.key", password="")
except (ssl.SSLError, OSError) as error:
    sys.exit("the server chain or key cannot be loaded, or the key does not match the certificate (%s)" % (getattr(error, "reason", None) or type(error).__name__))
try:
    client = ssl.create_default_context(cafile="/m/ca.crt")
except (ssl.SSLError, OSError) as error:
    sys.exit("the CA file cannot be loaded (%s)" % (getattr(error, "reason", None) or type(error).__name__))
client.minimum_version = ssl.TLSVersion.TLSv1_3
pipes = [ssl.MemoryBIO() for _ in range(4)]
tls_client = client.wrap_bio(pipes[0], pipes[1], server_hostname=name)
tls_server = server.wrap_bio(pipes[2], pipes[3], server_side=True)
done = set()
try:
    for _ in range(16):
        for side in (tls_client, tls_server):
            if side not in done:
                try:
                    side.do_handshake()
                    done.add(side)
                except ssl.SSLWantReadError:
                    pass
        pipes[2].write(pipes[1].read())
        pipes[0].write(pipes[3].read())
        if len(done) == 2:
            break
except ssl.SSLCertVerificationError as error:
    sys.exit("the server certificate is refused: %s" % error.verify_message)
except ssl.SSLError as error:
    sys.exit("the TLS 1.3 handshake failed (%s)" % (error.reason or type(error).__name__))
if len(done) != 2 or tls_client.version() != "TLSv1.3":
    sys.exit("the TLS 1.3 handshake did not complete")
print("ok")
' "$OTERYN_PLATFORM_MTLS_SERVER_NAME" 2>&1)" && [[ "$verdict" == ok ]]; then
        return 0
    fi
    echo "Platform mTLS server certificate refused for ${OTERYN_PLATFORM_MTLS_SERVER_NAME}: ${verdict:-the check could not run} (README: Platform internal mTLS listener)." >&2
    return 1
}

# One digest of everything the running listener depends on: bind address, port, the three host
# paths, the contents of the server chain, key and client CA, and the nginx config. It is stored as
# the oteryn.platform-mtls.fingerprint label of internal-mtls, so the fast deploy path can tell
# whether the listener needs a full reconcile. The files are hashed in a throwaway container with no
# network (they may be root-only); only the combined digest leaves it. --mount fails on a missing
# file instead of creating it.
oteryn_platform_mtls_fingerprint() {
    local conf="${OTERYN_PLATFORM_MTLS_NGINX_CONF:-$(dirname "${BASH_SOURCE[0]}")/../nginx/internal-mtls.conf}" files
    files="$(docker run --rm --network none \
        --mount "type=bind,source=${PLATFORM_MTLS_SERVER_CERT_FILE},target=/m/server-fullchain.pem,readonly" \
        --mount "type=bind,source=${PLATFORM_MTLS_SERVER_KEY_FILE},target=/m/server.key,readonly" \
        --mount "type=bind,source=${PLATFORM_MTLS_CLIENT_CA_FILE},target=/m/client-ca.crt,readonly" \
        alpine:3.22 sha256sum /m/server-fullchain.pem /m/server.key /m/client-ca.crt 2>/dev/null)" || return 1
    [[ -n "$files" && -r "$conf" ]] || return 1
    {
        printf '%s\n' "$PLATFORM_MTLS_BIND_ADDRESS" "$PLATFORM_MTLS_PORT" "$PLATFORM_MTLS_SERVER_CERT_FILE" \
            "$PLATFORM_MTLS_SERVER_KEY_FILE" "$PLATFORM_MTLS_CLIENT_CA_FILE" "$files"
        sha256sum <"$conf"
    } | sha256sum | awk '{print $1}'
}

# The fingerprint label of a running internal-mtls container (empty when absent).
oteryn_platform_mtls_fingerprint_of() {
    command docker inspect --format '{{index .Config.Labels "oteryn.platform-mtls.fingerprint"}}' "$1" 2>/dev/null
}

# Brings the listener to the resolved setting after Platform is up: recreated while on (it then
# re-reads the host certificate files and carries their fingerprint), removed while off. Arguments:
# the route's Compose command words.
oteryn_reconcile_platform_mtls() {
    (($# > 0)) || { echo "Platform mTLS reconcile needs the Compose command." >&2; return 1; }
    if [[ "${OTERYN_PLATFORM_MTLS_ENABLED:-false}" == true ]]; then
        local fingerprint
        fingerprint="$(oteryn_platform_mtls_fingerprint)" || {
            echo "Platform mTLS listener files could not be fingerprinted (README: Platform internal mTLS listener)." >&2
            return 1
        }
        OTERYN_PLATFORM_MTLS_FINGERPRINT="$fingerprint" "$@" up -d --no-deps --force-recreate internal-mtls || return 1
    else
        "$@" rm -s -f internal-mtls >/dev/null || return 1
    fi
    # The listener the ingress stop kept is replaced (or removed); nothing is left to start.
    OTERYN_PLATFORM_MTLS_STOPPED_ID=''
}

# The Platform FastCGI switch a running Platform container carries (absent means false).
oteryn_platform_fastcgi_flag_of() {
    local container_id="$1" value
    value="$(command docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$container_id" \
        | sed -n 's/^OTERYN_PLATFORM_FASTCGI_ENABLED=//p')" || return 1
    printf '%s' "${value:-false}"
}

_oteryn_platform_container_http_status() {
    local container_id="$1"
    local path="$2"

    [[ -n "$container_id" ]] || return 2
    [[ "$path" == /* && "$path" != *$'\r'* && "$path" != *$'\n'* ]] || return 2

    # The Platform Actions runner is itself a Docker container. Host-published
    # 127.0.0.1 ports therefore belong to the Synology host namespace, not the
    # runner namespace. Probe from inside the exact Platform container instead.
    command docker exec "$container_id" php -r '
$path = $argv[1] ?? "/";
$socket = @fsockopen("127.0.0.1", 8000, $errno, $errstr, 2.0);
if ($socket === false) {
    exit(10);
}
stream_set_timeout($socket, 2);
$request = "GET {$path} HTTP/1.1\r\n"
    . "Host: oteryn.molehill.cloud\r\n"
    . "X-Forwarded-Host: oteryn.molehill.cloud\r\n"
    . "X-Forwarded-Proto: https\r\n"
    . "X-Forwarded-Port: 443\r\n"
    . "Connection: close\r\n\r\n";
if (@fwrite($socket, $request) === false) {
    fclose($socket);
    exit(11);
}
$statusLine = fgets($socket);
fclose($socket);
if (! is_string($statusLine)
    || preg_match("/^HTTP\\/[0-9.]+\\s+([0-9]{3})(?:\\s|$)/", trim($statusLine), $matches) !== 1) {
    exit(12);
}
fwrite(STDOUT, $matches[1]);
' "$path"
}

_oteryn_wait_platform_http_200() {
    local container_id="$1"
    local path="${2:-/health}"
    local attempts="${3:-30}"
    local status

    [[ "$attempts" =~ ^[1-9][0-9]*$ ]] || return 2
    for _ in $(seq 1 "$attempts"); do
        status="$(_oteryn_platform_container_http_status "$container_id" "$path" 2>/dev/null || true)"
        if [[ "$status" == 200 ]]; then
            return 0
        fi
        sleep 1
    done
    return 1
}

_oteryn_deploy_state_dir() { printf '%s\n' "${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"; }

# Legacy bootstrap only: old managed releases used one shared application SHA
# for Platform and Gateway. New releases never derive the overall release from
# component image metadata.
_oteryn_release_sha_for_images() {
    local platform_image="$1" gateway_image="$2"
    local platform_revision gateway_revision
    platform_revision="$(command docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "$platform_image" 2>/dev/null || true)"
    gateway_revision="$(command docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "$gateway_image" 2>/dev/null || true)"

    [[ "$platform_revision" =~ ^[0-9a-f]{40}$ ]] || {
        echo "Cannot prove Platform OCI application revision; refusing migration-bearing deployment." >&2
        return 1
    }
    [[ "$gateway_revision" =~ ^[0-9a-f]{40}$ ]] || {
        echo "Cannot prove Gateway OCI application revision; refusing migration-bearing deployment." >&2
        return 1
    }
    [[ "$platform_revision" == "$gateway_revision" ]] || {
        echo "Legacy Platform/Gateway OCI application revisions disagree; refusing bootstrap." >&2
        return 1
    }
    printf '%s\n' "$platform_revision"
}

_oteryn_verify_component_image_source() {
    local image="$1" expected_sha="$2" label="$3" revision
    [[ "$expected_sha" =~ ^[0-9a-f]{40}$ ]] || {
        echo "$label source SHA is missing or invalid." >&2
        return 1
    }
    [[ "$image" =~ @sha256:[0-9a-f]{64}$ ]] || {
        echo "$label image is not an immutable digest reference." >&2
        return 1
    }
    revision="$(command docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "$image" 2>/dev/null || true)"
    [[ "$revision" == "$expected_sha" ]] || {
        echo "$label OCI application revision does not match its persisted component source SHA." >&2
        return 1
    }
}

_oteryn_release_sha() {
    local release_sha="${OTERYN_RELEASE_SHA:-}"
    [[ "$release_sha" =~ ^[0-9a-f]{40}$ ]] || {
        echo "OTERYN_RELEASE_SHA must be the exact protected-main release identity." >&2
        return 1
    }
    [[ "${PLATFORM_SOURCE_SHA:-}" =~ ^[0-9a-f]{40}$ ]] || {
        echo "PLATFORM_SOURCE_SHA must be an exact component source SHA." >&2
        return 1
    }
    [[ "${GATEWAY_SOURCE_SHA:-}" =~ ^[0-9a-f]{40}$ ]] || {
        echo "GATEWAY_SOURCE_SHA must be an exact component source SHA." >&2
        return 1
    }
    _oteryn_verify_component_image_source "$PLATFORM_IMAGE" "$PLATFORM_SOURCE_SHA" Platform || return 1
    _oteryn_verify_component_image_source "$GATEWAY_IMAGE" "$GATEWAY_SOURCE_SHA" Gateway || return 1
    if [[ "${GATEWAY_VERSION:-}" =~ ^sha-([0-9a-f]{40})$ && "${BASH_REMATCH[1]}" != "$GATEWAY_SOURCE_SHA" ]]; then
        echo "GATEWAY_VERSION disagrees with Gateway component source SHA." >&2
        return 1
    fi
    printf '%s\n' "$release_sha"
}

_oteryn_contract_from_platform_image() {
    local platform_image="$1" payload line key value
    local policy='' schema='' accepts='' engine=''
    payload="$(command docker run --rm --network none --read-only --entrypoint cat "$platform_image" /var/www/html/deploy/synology/release-contract.env 2>/dev/null)" || {
        echo "Cannot read release compatibility contract from Platform image; refusing migration-bearing deployment." >&2
        return 1
    }
    while IFS= read -r line || [[ -n "$line" ]]; do
        line="${line%$'\r'}"
        [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue
        [[ "$line" == *=* ]] || { echo "Invalid release contract line in Platform image." >&2; return 1; }
        key="${line%%=*}"
        value="${line#*=}"
        case "$key" in
            OTERYN_MIGRATION_POLICY) policy="$value" ;;
            OTERYN_SCHEMA_COMPATIBILITY_ID) schema="$value" ;;
            OTERYN_APP_ACCEPTS_SCHEMA_IDS) accepts="$value" ;;
            PLATFORM_DB_ENGINE) engine="$value" ;;
            *) echo "Unexpected release contract key in Platform image: $key" >&2; return 1 ;;
        esac
    done <<<"$payload"

    [[ "$policy" == expand-contract ]] || { echo "Platform image migration policy must be expand-contract." >&2; return 1; }
    [[ "$schema" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$ ]] || { echo "Platform image schema compatibility identity is invalid." >&2; return 1; }
    [[ "$accepts" =~ ^[A-Za-z0-9._-]+(,[A-Za-z0-9._-]+)*$ ]] || { echo "Platform image accepted schema identity list is invalid." >&2; return 1; }
    # An image built before the engine switch existed carries no PLATFORM_DB_ENGINE: it is a MariaDB image,
    # which every deploy and rollback now refuses (oteryn_require_image_db_engine).
    engine="${engine:-mariadb}"
    [[ "$engine" == mariadb || "$engine" == postgres ]] || { echo "Platform image database engine is invalid." >&2; return 1; }
    printf '%s\t%s\t%s\t%s\n' "$policy" "$schema" "$accepts" "$engine"
}

# Refuses a Platform image whose release contract declares another database engine than the deploy engine
# (the same rule deploy.sh applies to a candidate), so a rollback never recreates a MariaDB-contract image
# against the PostgreSQL schema or the reverse. Usage: oteryn_require_image_db_engine <platform image>
oteryn_require_image_db_engine() {
    local contract image_engine
    contract="$(_oteryn_contract_from_platform_image "$1")" || return 1
    IFS=$'\t' read -r _ _ _ image_engine <<<"$contract" || return 1
    [[ "$image_engine" == "${OTERYN_PLATFORM_DB_ENGINE:-}" ]] || {
        echo "Platform image database engine (${image_engine:-unset}) differs from the deploy engine (${OTERYN_PLATFORM_DB_ENGINE:-unset}); refusing." >&2
        return 1
    }
}

_oteryn_load_candidate_contract() {
    local contract
    contract="$(_oteryn_contract_from_platform_image "$PLATFORM_IMAGE")" || return 1
    local candidate_engine
    IFS=$'\t' read -r OTERYN_MIGRATION_POLICY OTERYN_SCHEMA_COMPATIBILITY_ID OTERYN_APP_ACCEPTS_SCHEMA_IDS candidate_engine <<<"$contract" || return 1
    [[ -n "${OTERYN_MIGRATION_POLICY:-}" && -n "${OTERYN_SCHEMA_COMPATIBILITY_ID:-}" && -n "${OTERYN_APP_ACCEPTS_SCHEMA_IDS:-}" ]] || {
        echo "Unable to load candidate release contract from Platform image." >&2
        return 1
    }
    # The image and the deploy checkout must name the same database engine: an image built for the other
    # engine would migrate or serve the wrong database.
    [[ "$candidate_engine" == "${OTERYN_PLATFORM_DB_ENGINE:-}" ]] || {
        echo "Platform image database engine (${candidate_engine:-unset}) differs from the deploy engine (${OTERYN_PLATFORM_DB_ENGINE:-unset}); refusing." >&2
        return 1
    }
    export OTERYN_MIGRATION_POLICY OTERYN_SCHEMA_COMPATIBILITY_ID OTERYN_APP_ACCEPTS_SCHEMA_IDS
}

_oteryn_read_state_key() {
    local file="$1" key="$2" value
    value="$(sed -n "s/^${key}=//p" "$file" | head -n 1)"
    [[ -n "$value" ]] || { echo "Missing $key in $file" >&2; return 1; }
    printf '%s\n' "$value"
}

_oteryn_schema_list_contains() {
    local needle="$1" list="$2" item
    IFS=',' read -r -a items <<<"$list"
    for item in "${items[@]}"; do
        [[ "$item" == "$needle" ]] && return 0
    done
    return 1
}

_oteryn_write_schema_state_known() {
    local state_dir="$1" schema_id="$2" release_sha="$3"
    [[ "$schema_id" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$ ]] || { echo "Cannot persist invalid schema identity." >&2; return 1; }
    [[ "$release_sha" =~ ^[0-9a-f]{40}$ ]] || { echo "Cannot persist schema state without exact release identity." >&2; return 1; }
    {
        printf 'SCHEMA_STATE=known\n'
        printf 'SCHEMA_COMPATIBILITY_ID=%s\n' "$schema_id"
        printf 'MIGRATION_TARGET_RELEASE_SHA=%s\n' "$release_sha"
    } >"$state_dir/schema-state.env.tmp"
    chmod 600 "$state_dir/schema-state.env.tmp"
    mv "$state_dir/schema-state.env.tmp" "$state_dir/schema-state.env"
}

_oteryn_known_schema_identity() {
    local state_dir="$1" schema_file="$state_dir/schema-state.env" schema_state schema_id
    [[ -f "$schema_file" ]] || { echo "Managed schema identity is missing; refusing migration-bearing deployment." >&2; return 1; }
    schema_state="$(_oteryn_read_state_key "$schema_file" SCHEMA_STATE)" || return 1
    schema_id="$(_oteryn_read_state_key "$schema_file" SCHEMA_COMPATIBILITY_ID)" || return 1
    [[ "$schema_state" == known ]] || { echo "Managed schema identity is not known; refusing migration-bearing deployment." >&2; return 1; }
    [[ "$schema_id" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$ ]] || { echo "Managed schema identity is invalid; refusing migration-bearing deployment." >&2; return 1; }
    printf '%s\n' "$schema_id"
}

_oteryn_bootstrap_legacy_current_release() {
    local state_dir="$1" current_file="$state_dir/current-release.env" legacy_file="$state_dir/last-good.env"
    local old_platform old_gateway old_sha observed_schema table_count
    local -a legacy_images
    [[ ! -f "$current_file" ]] || return 0

    if [[ ! -f "$legacy_file" ]]; then
        table_count="$(oteryn_postgres_relation_count command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")" || {
            echo "Cannot determine whether Platform DB is fresh; refusing migration." >&2
            return 1
        }
        [[ "$table_count" =~ ^[0-9]+$ ]] || { echo "Cannot determine whether Platform DB is fresh; refusing migration." >&2; return 1; }
        if (( table_count > 0 )); then
            echo "Existing Platform DB has no managed application baseline; refusing migration before backup-capable baseline is proven." >&2
            return 1
        fi
        return 0
    fi

    mapfile -t legacy_images < <(bash -c 'set -euo pipefail; source "$1"; printf "%s\n%s\n" "${PLATFORM_IMAGE:-}" "${GATEWAY_IMAGE:-}"' bash "$legacy_file")
    old_platform="${legacy_images[0]:-}"
    old_gateway="${legacy_images[1]:-}"
    [[ -n "$old_platform" && -n "$old_gateway" ]] || {
        echo "Legacy running-release snapshot is incomplete; refusing migration." >&2
        return 1
    }
    old_sha="$(_oteryn_release_sha_for_images "$old_platform" "$old_gateway")" || return 1

    observed_schema="observed-${old_sha}"
    # A legacy snapshot predates the native-only cut and needs the retired legacy-engine
    # topology, so it is recorded as ineligible for image rollback.
    bash "$SCRIPT_DIR/release-state.sh" write "$current_file" "$old_sha" "$old_sha" "$old_sha" \
        "$observed_schema" "$observed_schema" "$old_platform" "$old_gateway" 0
    _oteryn_write_schema_state_known "$state_dir" "$observed_schema" "$old_sha"
}

_oteryn_marketplace_state_file() {
    printf '%s/marketplace.env\n' "$(_oteryn_deploy_state_dir)"
}

# Durable Marketplace state written by earlier releases also carries retired escrow
# account and character-transfer keys. They are dropped: ignored, never re-emitted
# and never a reason to block a deploy. Every other unknown key still fails closed.
OTERYN_RETIRED_MARKETPLACE_STATE_KEYS=(
    MARKETPLACE_ESCROW_CANARY_ACCOUNT_ID
    MARKETPLACE_ESCROW_CANARY_ACCOUNT_NAME
    MARKETPLACE_ESCROW_CANARY_ACCOUNT_CREATION_EPOCH
    CANARY_CHARACTER_TRANSFER_DB_USER
    CANARY_CHARACTER_TRANSFER_DB_PASSWORD
)

_oteryn_marketplace_non_terminal_auction_count() {
    oteryn_postgres_non_terminal_auction_count command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE"
}

# Reverse of _oteryn_quiesce_platform_db_consumers for a refused migration: starts only
# the services that were running before, then the scheduler if it was running.
_oteryn_restart_after_refused_migration() {
    local was_running="$1" service scheduler_id
    local -a base_compose=(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")

    for service in platform internal-proxy gateway; do
        if grep -qx "$service" <<<"$was_running"; then
            "${base_compose[@]}" start "$service" >/dev/null || true
        fi
    done
    if [[ "${OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING:-0}" == 1 ]]; then
        scheduler_id="$(_oteryn_marketplace_scheduler_id)" || return 0
        [[ -z "$scheduler_id" ]] || command docker start "$scheduler_id" >/dev/null || true
    fi
    return 0
}

# Rewrites a durable state file to the current format (MARKETPLACE_ENABLED=false
# only): the retired keys are dropped and any enabled runtime is disabled, because
# the transfer contract is gone. Either rewrite needs a verified zero
# count of non-terminal auctions (N > 0 is refused). When no count can be obtained
# (database not running), it fails closed unless the operator attests, with
# OTERYN_ATTEST_NO_PENDING_AUCTIONS=1, that no auction can be pending. Idempotent.
# With the argument "quiesce" (the deploy path), Platform, Gateway, the internal proxy
# and the scheduler are stopped before the count so no listing can be accepted between
# the count and the key removal; the deploy brings them back up afterwards.
_oteryn_migrate_legacy_marketplace_state() {
    local quiesce="${1:-}"
    local durable_file line key value enabled='' legacy=0 retired pending tmp

    durable_file="$(_oteryn_marketplace_state_file)"
    [[ -f "$durable_file" ]] || return 0

    while IFS= read -r line || [[ -n "$line" ]]; do
        line="${line%$'\r'}"
        [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue
        [[ "$line" == *=* ]] || { echo "Invalid Marketplace state line." >&2; return 1; }
        key="${line%%=*}"
        value="${line#*=}"
        if [[ "$key" == MARKETPLACE_ENABLED ]]; then
            [[ -z "$enabled" ]] || { echo "Duplicate Marketplace state key: $key" >&2; return 1; }
            enabled="$value"
            continue
        fi
        for retired in "${OTERYN_RETIRED_MARKETPLACE_STATE_KEYS[@]}"; do
            if [[ "$key" == "$retired" ]]; then
                legacy=1
                continue 2
            fi
        done
        echo "Unexpected durable Marketplace state key: $key" >&2
        return 1
    done < "$durable_file"

    [[ "$legacy" == 1 || "$enabled" == true ]] || return 0
    [[ "$enabled" == true || "$enabled" == false ]] || {
        echo "Marketplace runtime state has invalid enabled flag." >&2
        return 1
    }

    # Any rewrite (retired keys or an enabled flag) disables the runtime, so it needs the drain proof.
    if [[ "$legacy" == 1 || "$enabled" == true ]]; then
        local was_running='' refused=''
        if [[ "$quiesce" == quiesce ]]; then
            was_running="$(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" ps --services --status running 2>/dev/null | grep -E '^(platform|internal-proxy|gateway)$' || true)"
            _oteryn_quiesce_platform_db_consumers || { _oteryn_restart_after_refused_migration "$was_running" || true; return 1; }
        fi
        if pending="$(_oteryn_marketplace_non_terminal_auction_count 2>/dev/null)" && [[ "$pending" =~ ^[0-9]+$ ]]; then
            if [[ "$pending" -gt 0 ]]; then
                echo "Legacy Character Bazaar has $pending non-terminal auctions; drain them before disabling the Marketplace state." >&2
                refused=1
            fi
        elif [[ "${OTERYN_ATTEST_NO_PENDING_AUCTIONS:-}" != 1 ]]; then
            echo "Cannot prove the legacy Character Bazaar is drained; refusing to disable the Marketplace state. If no auction can be pending, set OTERYN_ATTEST_NO_PENDING_AUCTIONS=1 to attest that." >&2
            refused=1
        fi
        if [[ -n "$refused" ]]; then
            # Bring back exactly what the quiesce stopped (best effort; the refusal stays the result).
            if [[ "$quiesce" == quiesce ]]; then
                _oteryn_restart_after_refused_migration "$was_running" || true
            fi
            return 1
        fi
    fi

    tmp="$durable_file.tmp"
    ( umask 077; printf 'MARKETPLACE_ENABLED=false\n' >"$tmp" )
    chmod 600 "$tmp"
    mv "$tmp" "$durable_file"
    echo "Rewrote the durable Marketplace state as disabled without the retired keys."
}

_oteryn_load_marketplace_runtime_state() {
    local durable_file source_file line key value seen='|'
    local -a required=(
        MARKETPLACE_ENABLED
    )
    durable_file="$(_oteryn_marketplace_state_file)"
    source_file="$durable_file"
    if [[ -f "$ENV_FILE" ]] && grep -q '^MARKETPLACE_ENABLED=' "$ENV_FILE"; then
        source_file="$ENV_FILE"
    fi
    [[ -f "$source_file" ]] || return 2
    if [[ "$source_file" == "$durable_file" ]]; then
        _oteryn_migrate_legacy_marketplace_state || return 1
    fi

    unset MARKETPLACE_ENABLED

    while IFS= read -r line || [[ -n "$line" ]]; do
        line="${line%$'\r'}"
        [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue
        [[ "$line" == *=* ]] || { echo "Invalid Marketplace state line." >&2; return 1; }
        key="${line%%=*}"
        value="${line#*=}"
        case "$key" in
            MARKETPLACE_ENABLED)
                if [[ "$seen" == *"|$key|"* ]]; then
                    echo "Duplicate Marketplace state key: $key" >&2
                    return 1
                fi
                seen="${seen}${key}|"
                printf -v "$key" '%s' "$value"
                export "$key"
                ;;
            *)
                [[ "$source_file" != "$durable_file" ]] || { echo "Unexpected durable Marketplace state key: $key" >&2; return 1; }
                ;;
        esac
    done < "$source_file"

    local name
    for name in "${required[@]}"; do
        [[ -n "${!name:-}" ]] || { echo "Marketplace runtime state is incomplete: $name" >&2; return 1; }
    done
    [[ "$MARKETPLACE_ENABLED" == true || "$MARKETPLACE_ENABLED" == false ]] || { echo "Marketplace runtime state has invalid enabled flag." >&2; return 1; }
}

_oteryn_marketplace_scheduler_id() {
    local ids count
    ids="$(command docker ps -a \
        --filter "label=com.docker.compose.project=${COMPOSE_PROJECT_NAME:-oteryn-staging}" \
        --filter 'label=com.docker.compose.service=marketplace-scheduler' \
        --format '{{.ID}}' | sed '/^$/d')" || return 1
    count="$(printf '%s\n' "$ids" | sed '/^$/d' | wc -l | tr -d ' ')"
    [[ "$count" -le 1 ]] || { echo "Expected at most one Marketplace scheduler container." >&2; return 1; }
    printf '%s\n' "$ids"
}

_oteryn_recreate_marketplace_runtime() {
    local marketplace_file="$DEPLOY_DIR/compose.marketplace.yml"
    local -a marketplace_compose=(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" -f "$marketplace_file")
    [[ -f "$marketplace_file" ]] || { echo "Marketplace Compose file is missing." >&2; return 1; }
    _oteryn_load_marketplace_runtime_state || { echo "Cannot prove Marketplace runtime settings for runtime recreation." >&2; return 1; }
    [[ "$MARKETPLACE_ENABLED" == true ]] || { echo "Refusing to recreate Marketplace runtime while durable/effective state is disabled." >&2; return 1; }

    # From the ingress stop on, every failure leaves Platform, the internal proxy and Gateway removed
    # (fail closed): the callers may have had ingress running, and a stopped container would come
    # back through `restart: always`.
    if _oteryn_recreate_marketplace_runtime_gated "${marketplace_compose[@]}"; then
        return 0
    fi
    echo "Marketplace runtime recreation failed; removing the Platform, internal proxy and Gateway containers (fail closed)." >&2
    oteryn_platform_mtls_remove_fail_closed "${marketplace_compose[@]}"
    "${marketplace_compose[@]}" rm -s -f gateway internal-proxy platform >&2 || true
    return 1
}

_oteryn_recreate_marketplace_runtime_gated() {
    local platform_id scheduler_id platform_running scheduler_running platform_image scheduler_image
    local platform_enabled scheduler_enabled ingress
    local -a marketplace_compose=("$@")

    # Same order as every Platform route: ingress stays stopped until the recreated Platform passes the
    # trust gate and the self-check; only the ingress containers that were running before start again.
    ingress="$(oteryn_native_admission_ingress_stop "${marketplace_compose[@]}")" || return 1
    "${marketplace_compose[@]}" up -d --no-deps --force-recreate platform || return 1
    oteryn_publish_native_admission_trust_or_stop "${marketplace_compose[@]}" || return 1
    # The replacement Platform is the one that will serve: prove it can sign before ingress returns.
    oteryn_native_admission_self_check_or_stop "${marketplace_compose[@]}" || return 1
    # shellcheck disable=SC2086 # container ids, one per line
    oteryn_native_admission_ingress_start $ingress || return 1
    "${marketplace_compose[@]}" up -d --force-recreate marketplace-scheduler || return 1

    platform_id="$("${marketplace_compose[@]}" ps -a -q platform)" || return 1
    scheduler_id="$(_oteryn_marketplace_scheduler_id)" || return 1
    [[ -n "$platform_id" && -n "$scheduler_id" ]] || { echo "Marketplace Platform/scheduler runtime was not recreated." >&2; return 1; }

    platform_running="$(command docker inspect --format '{{.State.Running}}' "$platform_id")" || return 1
    scheduler_running="$(command docker inspect --format '{{.State.Running}}' "$scheduler_id")" || return 1
    platform_image="$(command docker inspect --format '{{.Config.Image}}' "$platform_id")" || return 1
    scheduler_image="$(command docker inspect --format '{{.Config.Image}}' "$scheduler_id")" || return 1
    platform_enabled="$(command docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$platform_id" | sed -n 's/^MARKETPLACE_ENABLED=//p' | head -n 1)"
    scheduler_enabled="$(command docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$scheduler_id" | sed -n 's/^MARKETPLACE_ENABLED=//p' | head -n 1)"

    [[ "$platform_running" == true && "$scheduler_running" == true ]] || { echo "Marketplace Platform/scheduler is not fully running after recreation." >&2; return 1; }
    [[ "$platform_image" == "$PLATFORM_IMAGE" ]] || { echo "Marketplace Platform image does not match the selected Platform image." >&2; return 1; }
    [[ "$scheduler_image" == "$PLATFORM_IMAGE" ]] || { echo "Marketplace scheduler image does not match the selected Platform image." >&2; return 1; }
    [[ "$platform_enabled" == true && "$scheduler_enabled" == true ]] || { echo "Marketplace runtime did not receive enabled settings." >&2; return 1; }
}

_oteryn_recreate_marketplace_scheduler() {
    # Compatibility name retained for the existing contract test; recreation now
    # covers both the browser-facing Platform container and scheduler.
    _oteryn_recreate_marketplace_runtime
}

_oteryn_quiesce_platform_db_consumers() {
    local scheduler_id scheduler_running
    local -a base_compose=(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")

    OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING=0
    export OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING
    scheduler_id="$(_oteryn_marketplace_scheduler_id)" || return 1
    if [[ -n "$scheduler_id" ]]; then
        scheduler_running="$(command docker inspect --format '{{.State.Running}}' "$scheduler_id")" || return 1
        if [[ "$scheduler_running" == true ]]; then
            OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING=1
            export OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING
            command docker stop "$scheduler_id" >/dev/null
            scheduler_running="$(command docker inspect --format '{{.State.Running}}' "$scheduler_id")" || return 1
            [[ "$scheduler_running" == false ]] || {
                echo "Deployment rejected: marketplace-scheduler is still running before migration backup." >&2
                return 1
            }
        fi
    fi

    "${base_compose[@]}" stop platform gateway internal-proxy
}

# The scheduler is an always-on base service (compose.yml): every runtime change recreates it with the
# selected Platform image. With Marketplace enabled the whole browser-facing runtime is recreated
# through the overlay; otherwise (disabled, or no durable state yet) only the scheduler is recreated
# from the base file with the switch off.
_oteryn_recreate_base_scheduler() {
    local scheduler_id running image
    command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" up -d --no-deps --force-recreate marketplace-scheduler || return 1
    scheduler_id="$(_oteryn_marketplace_scheduler_id)" || return 1
    [[ -n "$scheduler_id" ]] || { echo "Scheduler was not created." >&2; return 1; }
    running="$(command docker inspect --format '{{.State.Running}}' "$scheduler_id")" || return 1
    image="$(command docker inspect --format '{{.Config.Image}}' "$scheduler_id")" || return 1
    [[ "$running" == true ]] || { echo "Scheduler is not running after recreation." >&2; return 1; }
    [[ "$image" == "$PLATFORM_IMAGE" ]] || { echo "Scheduler image does not match the selected Platform image." >&2; return 1; }
}

_oteryn_reconcile_marketplace_scheduler_after_runtime_change() {
    local state_rc
    set +e
    _oteryn_load_marketplace_runtime_state
    state_rc=$?
    set -e
    if [[ "$state_rc" -eq 2 ]]; then
        _oteryn_recreate_base_scheduler || return 1
        return 0
    fi
    [[ "$state_rc" -eq 0 ]] || return "$state_rc"

    if [[ "$MARKETPLACE_ENABLED" == true ]]; then
        _oteryn_recreate_marketplace_scheduler || return 1
    else
        _oteryn_recreate_base_scheduler || return 1
    fi
}

_oteryn_restore_quiesced_consumers_after_migrate() {
    _oteryn_reconcile_marketplace_scheduler_after_runtime_change || return 1
    OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING=0
    export OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING
}

_oteryn_resume_candidate_if_safe() {
    local state_dir="$1" release_sha="$2"
    local candidate_file="$state_dir/candidate-release.env" schema_file="$state_dir/schema-state.env"
    local schema_state schema_id schema_target candidate_schema candidate_accepts
    local -a candidate_runtime

    [[ -f "$candidate_file" ]] || return 1
    bash "$SCRIPT_DIR/release-state.sh" validate "$candidate_file" || return 1
    [[ -f "$schema_file" ]] || {
        echo "Candidate resume rejected: managed schema state is missing." >&2
        return 1
    }

    mapfile -t candidate_runtime < <(bash -c '
        set -euo pipefail
        source "$1"
        printf "%s\n" \
            "$RELEASE_SHA" "$PLATFORM_SOURCE_SHA" "$GATEWAY_SOURCE_SHA" \
            "$PLATFORM_IMAGE" "$GATEWAY_IMAGE" \
            "${GAME_WORLD_ID:-}" "${GAME_WORLD_SLUG:-}" "${GAME_WORLD_NAME:-}" \
            "${GAME_WORLD_REGION:-}" "${GAME_WORLD_HOST:-}" "${GAME_WORLD_PORT:-}"
    ' bash "$candidate_file")

    [[ "${candidate_runtime[0]:-}" == "$release_sha" ]] || {
        echo "Deployment rejected: unresolved candidate release ${candidate_runtime[0]:-UNKNOWN} differs from requested release $release_sha." >&2
        return 1
    }
    [[ "${candidate_runtime[1]:-}" == "$PLATFORM_SOURCE_SHA" \
        && "${candidate_runtime[2]:-}" == "$GATEWAY_SOURCE_SHA" ]] || {
        echo "Candidate resume rejected: component source identity drifted." >&2
        return 1
    }
    [[ "${candidate_runtime[3]:-}" == "$PLATFORM_IMAGE" \
        && "${candidate_runtime[4]:-}" == "$GATEWAY_IMAGE" ]] || {
        echo "Candidate resume rejected: immutable runtime image identity drifted." >&2
        return 1
    }
    [[ "${candidate_runtime[5]:-}" == "${GAME_WORLD_ID:-}" \
        && "${candidate_runtime[6]:-}" == "${GAME_WORLD_SLUG:-}" \
        && "${candidate_runtime[7]:-}" == "${GAME_WORLD_NAME:-}" \
        && "${candidate_runtime[8]:-}" == "${GAME_WORLD_REGION:-}" \
        && "${candidate_runtime[9]:-}" == "${GAME_WORLD_HOST:-}" \
        && "${candidate_runtime[10]:-}" == "${GAME_WORLD_PORT:-}" ]] || {
        echo "Candidate resume rejected: staged world identity drifted." >&2
        return 1
    }

    schema_state="$(_oteryn_read_state_key "$schema_file" SCHEMA_STATE)" || return 1
    schema_id="$(_oteryn_read_state_key "$schema_file" SCHEMA_COMPATIBILITY_ID)" || return 1
    schema_target="$(_oteryn_read_state_key "$schema_file" MIGRATION_TARGET_RELEASE_SHA)" || return 1
    candidate_schema="$(_oteryn_read_state_key "$candidate_file" SCHEMA_COMPATIBILITY_ID)" || return 1
    candidate_accepts="$(_oteryn_read_state_key "$candidate_file" APP_ACCEPTS_SCHEMA_IDS)" || return 1

    [[ "$schema_state" == known && "$schema_target" == "$release_sha" && "$schema_id" == "$candidate_schema" ]] || {
        echo "Candidate resume rejected: prior migration is not proven complete for the exact candidate release." >&2
        return 1
    }
    _oteryn_schema_list_contains "$schema_id" "$candidate_accepts" || {
        echo "Candidate resume rejected: candidate application does not accept the proven schema." >&2
        return 1
    }

    OTERYN_SAME_RELEASE_REDEPLOY=1
    export OTERYN_SAME_RELEASE_REDEPLOY
    echo "Resuming exact candidate release $release_sha after a previously successful migration; preserving recovery evidence until health checks pass."
    return 0
}

_oteryn_before_platform_migrate() {
    local state_dir release_sha backup_dir backup_file backup_meta current_file old_sha old_schema
    state_dir="$(_oteryn_deploy_state_dir)"
    release_sha="$(_oteryn_release_sha)" || return 1

    # Drop retired durable Marketplace state keys before any schema change.
    if ! { [[ -f "$ENV_FILE" ]] && grep -q '^MARKETPLACE_ENABLED=' "$ENV_FILE"; }; then
        _oteryn_migrate_legacy_marketplace_state quiesce || return 1
    fi

    if [[ -f "$state_dir/candidate-release.env" ]]; then
        _oteryn_resume_candidate_if_safe "$state_dir" "$release_sha" || return 1
        return 0
    fi

    _oteryn_load_candidate_contract || return 1
    mkdir -p "$state_dir/backups"
    chmod 700 "$state_dir" "$state_dir/backups"

    _oteryn_bootstrap_legacy_current_release "$state_dir" || return 1

    bash "$SCRIPT_DIR/release-state.sh" write "$state_dir/candidate-release.env" "$release_sha" \
        "$PLATFORM_SOURCE_SHA" "$GATEWAY_SOURCE_SHA" \
        "$OTERYN_SCHEMA_COMPATIBILITY_ID" "$OTERYN_APP_ACCEPTS_SCHEMA_IDS" \
        "$PLATFORM_IMAGE" "$GATEWAY_IMAGE" 1

    current_file="$state_dir/current-release.env"
    if [[ -f "$current_file" ]]; then
        bash "$SCRIPT_DIR/release-state.sh" validate "$current_file"
        old_sha="$(_oteryn_read_state_key "$current_file" RELEASE_SHA)" || return 1
        old_schema="$(_oteryn_known_schema_identity "$state_dir")" || return 1

        if [[ "$old_sha" == "$release_sha" ]]; then
            _oteryn_schema_list_contains "$old_schema" "$OTERYN_APP_ACCEPTS_SCHEMA_IDS" || {
                echo "Same-release redeploy rejected: candidate application does not accept the known current schema." >&2
                return 1
            }
            OTERYN_SAME_RELEASE_REDEPLOY=1
            export OTERYN_SAME_RELEASE_REDEPLOY
            return 0
        fi

        cp "$current_file" "$state_dir/last-good-release.env.tmp"
        chmod 600 "$state_dir/last-good-release.env.tmp"
        mv "$state_dir/last-good-release.env.tmp" "$state_dir/last-good-release.env"

        _oteryn_quiesce_platform_db_consumers || return 1

        backup_dir="$state_dir/backups/${old_sha}-before-${release_sha}"
        mkdir -p "$backup_dir"
        chmod 700 "$backup_dir"
        # pg_dump -Fc by the read-only dump role, validated with pg_restore --list (plan P4 work 5).
        backup_file="$backup_dir/platform.dump"
        oteryn_postgres_dump_platform command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" "$backup_file" || return 1
        backup_meta="$backup_dir/evidence.env"
        {
            # A backup without this key is MariaDB-era; recover-schema.sh refuses it.
            printf 'BACKUP_DB_ENGINE=postgres\n'
            printf 'BACKUP_FROM_RELEASE_SHA=%s\n' "$old_sha"
            printf 'BACKUP_BEFORE_RELEASE_SHA=%s\n' "$release_sha"
            printf 'BACKUP_SCHEMA_COMPATIBILITY_ID=%s\n' "$old_schema"
            printf 'BACKUP_COMPOSE_PROJECT_NAME=%s\n' "${COMPOSE_PROJECT_NAME:-oteryn-staging}"
            printf 'BACKUP_PLATFORM_DB_NAME=%s\n' "$PLATFORM_DB_NAME"
            printf 'BACKUP_SHA256=%s\n' "$(sha256sum "$backup_file" | awk '{print $1}')"
        } >"$backup_meta.tmp"
        chmod 600 "$backup_meta.tmp"
        mv "$backup_meta.tmp" "$backup_meta"
    fi

    {
        printf 'SCHEMA_STATE=unknown\n'
        printf 'MIGRATION_TARGET_RELEASE_SHA=%s\n' "$release_sha"
    } >"$state_dir/schema-state.env.tmp"
    chmod 600 "$state_dir/schema-state.env.tmp"
    mv "$state_dir/schema-state.env.tmp" "$state_dir/schema-state.env"
}

_oteryn_after_platform_migrate() {
    local state_dir release_sha
    state_dir="$(_oteryn_deploy_state_dir)"
    release_sha="$(_oteryn_release_sha)" || return 1
    _oteryn_write_schema_state_known "$state_dir" "$OTERYN_SCHEMA_COMPATIBILITY_ID" "$release_sha"
    _oteryn_restore_quiesced_consumers_after_migrate || return 1
}

_oteryn_finalize_release_on_exit() {
    local rc="$?" state_dir
    [[ "$rc" -eq 0 ]] || return "$rc"
    [[ "$(basename -- "${0:-}")" == "deploy.sh" ]] || return 0
    state_dir="$(_oteryn_deploy_state_dir)"
    bash "$SCRIPT_DIR/release-state.sh" validate "$state_dir/candidate-release.env"
    cp "$state_dir/candidate-release.env" "$state_dir/current-release.env.tmp"
    chmod 600 "$state_dir/current-release.env.tmp"
    mv "$state_dir/current-release.env.tmp" "$state_dir/current-release.env"
    rm -f "$state_dir/candidate-release.env"
    return 0
}

docker() {
    local -a args=("$@")
    local i joined rc
    for i in "${!args[@]}"; do
        case "${args[$i]}" in
            alpine:3.22) args[$i]="$OTERYN_HEALTH_ALPINE_IMAGE" ;;
            python:3.12-alpine) args[$i]="$OTERYN_HEALTH_PYTHON_IMAGE" ;;
        esac
    done

    if [[ "$(basename -- "${0:-}")" == "deploy.sh" ]]; then
        joined=" ${args[*]} "
        if [[ "${OTERYN_MIGRATION_PREPARED:-0}" != 1 && "$joined" == *" up -d platform "* ]]; then
            _oteryn_before_platform_migrate || return 1
            command docker "${args[@]}"
            rc=$?
            [[ "$rc" -eq 0 ]] || return "$rc"
            OTERYN_MIGRATION_PREPARED=1
            export OTERYN_MIGRATION_PREPARED
            return 0
        fi
        if [[ "$joined" == *" exec -T platform php artisan migrate --force --no-interaction "* \
            || "$joined" == *" ${OTERYN_POSTGRES_MIGRATE_ARGS[*]} "* ]]; then
            [[ "${OTERYN_MIGRATION_PREPARED:-0}" == 1 ]] || _oteryn_before_platform_migrate || return 1
            if [[ "${OTERYN_SAME_RELEASE_REDEPLOY:-0}" == 1 ]]; then
                echo "Same-release redeploy: skipping migration; the exact current image already proved this schema transition."
                OTERYN_SAME_RELEASE_REDEPLOY=0
                OTERYN_MIGRATION_PREPARED=0
                export OTERYN_SAME_RELEASE_REDEPLOY OTERYN_MIGRATION_PREPARED
                _oteryn_reconcile_marketplace_scheduler_after_runtime_change || return 1
                return 0
            fi
            command docker "${args[@]}"
            rc=$?
            if [[ "$rc" -eq 0 ]]; then
                _oteryn_after_platform_migrate || return 1
                OTERYN_MIGRATION_PREPARED=0
                export OTERYN_MIGRATION_PREPARED
            fi
            return "$rc"
        fi
    fi
    command docker "${args[@]}"
}

# P6a D8 guard: on PostgreSQL, picks the open attempt intent for a first trust publication. Prints nothing
# when no fence is needed (a profile already exists) and the base64url fence otherwise. It writes nothing;
# the producer repeats every check from the evidence directory, so this is an early, clear refusal and not
# the security boundary. Argument: the Platform container id.
oteryn_first_publication_guard() {
    local container_id="${1:-}"
    local evidence_dir="${OTERYN_ATTEMPT_EVIDENCE_DIR:-/var/lib/oteryn-attempt-evidence}"
    local tool observation parsed fence
    tool="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/attempt_evidence.py"

    if ! observation="$(docker exec "$container_id" /usr/local/bin/oteryn-platform-entrypoint \
        php artisan game-auth:native-evidence:observe-trust --trust=fresh --no-interaction 2>&1)"; then
        printf '%s\n' "$observation" >&2
        echo "First-publication guard: the read-only trust observation failed." >&2
        return 1
    fi
    observation="$(printf '%s\n' "$observation" | tail -n 1)"
    if ! parsed="$(printf '%s' "$observation" | python3 -I -c '
import json, sys
o = json.load(sys.stdin)
if o.get("result") != "observed":
    print("refuse:observation " + str(o.get("result")))
elif (o.get("profile") or {}).get("present") is True:
    print("nofence")
else:
    inflight = ((o.get("database") or {}).get("in_flight")) or {}
    ok = (
        o.get("fence_required") is True
        and (o.get("store") or {}).get("binding") == "match"
        and (o.get("floor") or {}).get("value") is None
        and o.get("database") is not None
        and inflight.get("sessions") == 0
        and inflight.get("prepared") == 0
        and (o.get("key") or {}).get("configured_key_id")
    )
    if not ok:
        print("refuse:state")
    else:
        print("fence")
        print((o["store"]).get("marker") or "")
        print(o["key"]["configured_key_id"])
        print(json.dumps(o["scope"], sort_keys=True, separators=(",", ":")))
        print(json.dumps(o["database"]["identity"], sort_keys=True, separators=(",", ":")))
' 2>/dev/null)"; then
        echo "First-publication guard: the trust observation is not valid JSON." >&2
        return 1
    fi
    case "${parsed%%$'\n'*}" in
        nofence) return 0 ;;
        fence) ;;
        *)
            echo "First-publication guard refused (${parsed%%$'\n'*}): no profile exists and the fence setting, witness, floor, binding, database or key state does not allow a first publication." >&2
            return 1
            ;;
    esac
    local lines store key_id scope identity
    mapfile -t lines <<<"$parsed"
    store="${lines[1]}"
    key_id="${lines[2]}"
    scope="${lines[3]}"
    identity="${lines[4]}"
    if ! fence="$(python3 -I "$tool" require-open-attempt --dir "$evidence_dir" --store "$store" \
        --key-id "$key_id" --scope "$scope" --identity "$identity")"; then
        echo "First-publication guard: no open, anchored attempt intent for this store, key and scope (attempt_evidence.py require-open-attempt)." >&2
        return 1
    fi
    python3 -I -c 'import base64, sys; print(base64.urlsafe_b64encode(sys.stdin.buffer.read().strip()).decode().rstrip("="))' <<<"$fence"
}

# Owner decision D1006: with native admission switched on, every Platform (re)creation for a
# release, rollback or Marketplace switch trusts the PUBLIC half of the
# Platform issuer's current signing key in the deployment's own signing trust registry (PostgreSQL),
# after the key exists and before the signing self-check verifies against that registry. The
# command derives the public key from the loaded private key, refuses outside testing or
# preproduction, keeps every registry rule (a kid is bound to one key forever, revoked kids stay
# revoked, at most two trusted keys, high-water witness on every write) and is idempotent. It
# prints one JSON line with the trust scope, key id and public key only: the values Game's trust
# configuration needs.
# The witness directory lives on the platform_storage volume shared by Platform and the
# marketplace scheduler. Once it is configured, every identity security revocation writes through
# it, so it is created owner-only on every deploy, whatever the native admission switch says. A
# missing witness for a registry with trust history fails closed.
oteryn_publish_native_admission_trust() {
    local container_id="${1:-}"
    local flag output

    [[ -n "$container_id" ]] || { echo "Native admission trust publication needs the Platform container." >&2; return 1; }
    if ! flag="$(docker exec "$container_id" sh -ec '
        test -n "${GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY:-}"
        umask 077
        mkdir -p -- "$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY"
        test ! -L "$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY"
        chmod 700 -- "$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY"
        test -O "$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY"
        test "$(stat -c %a -- "$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY")" = 700
        printf %s "${GAME_AUTH_NATIVE_ADMISSION_ENABLED:-}"
    ')"; then
        echo "Unable to prepare the native evidence witness directory (owner-only, not a link, owned by the Platform user) or read the Platform native admission flag." >&2
        return 1
    fi
    if [[ "$flag" != true ]]; then
        echo "Native admission is off; no signing key is trusted."
        return 0
    fi
    # Through the image entrypoint, exactly like the signing self-check: it exports the signing key id
    # and file path from the key volume, which a bare docker exec does not see.
    # P6a D8 on the only engine since P7 (PostgreSQL): every publication passes the first-publication guard.
    local fence="" fence_args=()
    fence="$(oteryn_first_publication_guard "$container_id")" || return 1
    [[ -z "$fence" ]] || fence_args=("--first-publication-fence=$fence")
    if ! output="$(docker exec "$container_id" /usr/local/bin/oteryn-platform-entrypoint \
        php artisan game-auth:native-admission:publish-trust --no-interaction ${fence_args[@]+"${fence_args[@]}"} 2>&1)"; then
        printf '%s\n' "$output" >&2
        echo "Native admission signing key trust publication refused; deploy stops before the signing self-check." >&2
        return 1
    fi
    if [[ ! "$output" =~ \"result\":\"(published|already_trusted)\" ]]; then
        echo "Native admission trust publication returned no trust result." >&2
        return 1
    fi
    echo "Native admission signing key trusted (PUBLIC values for Game's trust configuration): $output"
}

# NATIVE-LOGIN-E2E-1: publishes the Game world's native route record (game-auth:native-route:publish)
# as part of the deploy, driven by repository variables (no secret, no key). Opt-in:
# OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED=true (repository variable OTERYN_STAGING_NATIVE_ROUTE_PUBLISH_ENABLED).
# When on, every required value must be present and well-formed or the deploy refuses (fail closed);
# nothing is guessed. Order: the Registry row must exist (game-auth:world:ensure ran), then
# game-auth:native-topology:issue (idempotent) issues the WorldId/ChannelId, which must equal the
# configured native World (OTERYN_STAGING_NATIVE_WORLD_ID, an optional input needed only by this step), then the route is
# published and read back. Descriptor reference: Game deploy/synology-game/preprod-topology.toml
# ([scope] channel_key, [route] host, port, tls_server_name). Arguments: the route's Compose
# command words.
oteryn_native_route_publish_validate() {
    local missing=() name
    for name in OTERYN_NATIVE_CHANNEL_KEY OTERYN_NATIVE_ROUTE_HOST OTERYN_NATIVE_ROUTE_PORT OTERYN_NATIVE_ROUTE_TLS_SERVER_NAME GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_WORLD_ID; do
        [[ -n "${!name:-}" ]] || missing+=("$name")
    done
    if ((${#missing[@]} > 0)); then
        echo "Native route publication is enabled but required values are missing: ${missing[*]} (repository variables OTERYN_STAGING_NATIVE_CHANNEL_KEY, OTERYN_STAGING_NATIVE_ROUTE_HOST, OTERYN_STAGING_NATIVE_ROUTE_PORT, OTERYN_STAGING_NATIVE_ROUTE_TLS_SERVER_NAME, OTERYN_STAGING_NATIVE_WORLD_ID)." >&2
        return 1
    fi
    if [[ "${OTERYN_NATIVE_ADMISSION_ENABLED:-false}" != true ]]; then
        echo "Native route publication needs native admission enabled (OTERYN_STAGING_NATIVE_ADMISSION_ENABLED=true)." >&2
        return 1
    fi
    if [[ ! "$OTERYN_NATIVE_CHANNEL_KEY" =~ ^[a-z0-9][a-z0-9-]{0,63}$ ]]; then
        echo "OTERYN_STAGING_NATIVE_CHANNEL_KEY must be a lower-case channel key of at most 64 characters (a-z, 0-9, hyphen)." >&2
        return 1
    fi
    if [[ ! "$OTERYN_NATIVE_ROUTE_PORT" =~ ^[1-9][0-9]{0,4}$ ]] || ((OTERYN_NATIVE_ROUTE_PORT > 65535)); then
        echo "OTERYN_STAGING_NATIVE_ROUTE_PORT must be a TCP port from 1 to 65535." >&2
        return 1
    fi
    if [[ ! "$OTERYN_NATIVE_ROUTE_HOST" =~ ^[A-Za-z0-9]([A-Za-z0-9.-]{0,251}[A-Za-z0-9])?$ ]]; then
        echo "OTERYN_STAGING_NATIVE_ROUTE_HOST must be a host name or IPv4 address without scheme, port or path." >&2
        return 1
    fi
    if [[ ! "$OTERYN_NATIVE_ROUTE_TLS_SERVER_NAME" =~ ^[A-Za-z0-9]([A-Za-z0-9.-]{0,251}[A-Za-z0-9])?$ ]]; then
        echo "OTERYN_STAGING_NATIVE_ROUTE_TLS_SERVER_NAME must be a DNS name the Game client verifies." >&2
        return 1
    fi
    case "${OTERYN_NATIVE_ROUTE_LOGIN_ENABLED:-true}" in
        true | false) ;;
        *) echo "OTERYN_STAGING_NATIVE_ROUTE_LOGIN_ENABLED must be true or false." >&2; return 1 ;;
    esac
    if [[ ! "${GAME_WORLD_ID:-}" =~ ^[1-9][0-9]*$ ]]; then
        echo "GAME_WORLD_ID (the Registry world row) must be a positive integer for native route publication." >&2
        return 1
    fi
}

oteryn_publish_native_route() {
    local issued published issued_world
    (($# > 0)) || { echo "Native route publication needs the Compose command." >&2; return 1; }
    case "${OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED:-false}" in
        true) ;;
        false | '') echo "Native route publication is off (OTERYN_STAGING_NATIVE_ROUTE_PUBLISH_ENABLED is not true)."; return 0 ;;
        *) echo "OTERYN_STAGING_NATIVE_ROUTE_PUBLISH_ENABLED must be true or false." >&2; return 1 ;;
    esac
    oteryn_native_route_publish_validate || return 1

    if ! issued="$("$@" exec -T platform php artisan game-auth:native-topology:issue \
        --world-row-id="$GAME_WORLD_ID" --channel-key="$OTERYN_NATIVE_CHANNEL_KEY" --no-interaction 2>&1)"; then
        printf '%s\n' "$issued" >&2
        echo "Native topology issuance/readback failed; the native route was not published." >&2
        return 1
    fi
    issued_world="$(sed -n 's/.*"world_id":"\([^"]*\)".*/\1/p' <<<"$issued" | head -n1)"
    if [[ -z "$issued_world" ]]; then
        echo "Native topology issuance returned no WorldId; the native route was not published." >&2
        return 1
    fi
    if [[ "$issued_world" != "$GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_WORLD_ID" ]]; then
        echo "The Registry issued native WorldId $issued_world, which differs from OTERYN_STAGING_NATIVE_WORLD_ID; set that repository variable to the issued WorldId (not a secret) and redeploy. The native route was not published." >&2
        return 1
    fi

    echo "Native topology (non-secret receipt for the Game topology descriptor [scope]): $issued"
    if ! published="$("$@" exec -T platform php artisan game-auth:native-route:publish \
        --world-row-id="$GAME_WORLD_ID" --channel-key="$OTERYN_NATIVE_CHANNEL_KEY" \
        --host="$OTERYN_NATIVE_ROUTE_HOST" --port="$OTERYN_NATIVE_ROUTE_PORT" \
        --tls-server-name="$OTERYN_NATIVE_ROUTE_TLS_SERVER_NAME" \
        --login-enabled="${OTERYN_NATIVE_ROUTE_LOGIN_ENABLED:-true}" --no-interaction 2>&1)"; then
        printf '%s\n' "$published" >&2
        echo "Native route publication refused; native login has no route record." >&2
        return 1
    fi
    if [[ ! "$published" =~ \"route_revision\":\"[^\"]+\" ]]; then
        echo "Native route publication returned no route revision." >&2
        return 1
    fi
    echo "Native route record published (non-secret receipt for the Game topology descriptor): $published"
}

# Native admission grants reach Platform only through the internal proxy and Gateway (ingress):
# NativeAdmissionGrantIssuer reads the trusted key without consulting the witness. So every route
# that (re)creates Platform follows one order: stop ingress, (re)create Platform, pass the trust
# gate below, and only then start ingress. No request can reach a Platform whose trust publication
# and witness are unproven, so no grant can be issued from ambiguous trust state.
#
# oteryn_native_admission_ingress_stop <compose command words>: prints the ids of the ingress
# containers that were running (start order), then stops Gateway and the internal proxy.
#
# The Game-node mTLS listener (internal-mtls) is ingress too: it passes Game-node requests straight
# to Platform's FastCGI pool. While the listener is on it is stopped here with the rest, so every
# route that replaces Platform through this helper (deploy.sh fast, full and candidate paths,
# rollback.sh, deploy-impact.sh Platform reconcile, generate-native-admission-key.sh promotion,
# marketplace-staging.sh, the Marketplace runtime recreation and the failure restore below) keeps
# it stopped until the gates pass. Its id is printed last and also kept in
# OTERYN_PLATFORM_MTLS_STOPPED_ID (only for a direct call; a later stop that finds it already
# stopped keeps the first id), so it comes back through whichever way the route returns ingress:
# oteryn_native_admission_ingress_start (given ids or the kept id), the self-check gate below, or
# oteryn_reconcile_platform_mtls, which recreates it and forgets the kept id. While the setting is
# off, any existing listener is removed here whatever its state (a no-op when there is none): one left
# from before the setting was turned off must not keep answering for the Platform being replaced, and
# a stopped container with `restart: always` would come back on the next Docker daemon or NAS restart.
OTERYN_PLATFORM_MTLS_STOPPED_ID=''
oteryn_native_admission_ingress_stop() {
    local service id
    local -a running=()
    (($# > 0)) || { echo "Native admission ingress stop needs the Compose command." >&2; return 1; }
    for service in internal-proxy gateway; do
        id="$("$@" ps -q "$service")" || return 1
        [[ -z "$id" ]] || running+=("$id")
    done
    "$@" stop gateway internal-proxy >&2 || return 1
    if [[ "${OTERYN_PLATFORM_MTLS_ENABLED:-false}" == true ]]; then
        id="$("$@" ps -q internal-mtls)" || return 1
        if [[ -n "$id" ]]; then
            "$@" stop internal-mtls >&2 || return 1
            running+=("$id")
            OTERYN_PLATFORM_MTLS_STOPPED_ID="$id"
        fi
    else
        "$@" rm -s -f internal-mtls >&2 || return 1
    fi
    ((${#running[@]} == 0)) || printf '%s\n' "${running[@]}"
}

# Fail-closed companion of oteryn_native_admission_ingress_stop: oteryn_platform_mtls_remove_fail_closed
# <compose command words>. Every gate failure that removes Platform, the internal proxy and Gateway
# calls it first, so the Game-node mTLS listener the ingress stop only stopped is removed too (a
# stopped container with `restart: always` would come back on the next Docker daemon or NAS restart,
# in front of a Platform that never passed the gates). Whatever the setting, a no-op when there is
# no listener; the kept id is dropped, so nothing later starts a removed container. Best effort: the
# caller fails closed either way, and the next deploy recreates the listener only behind the gates.
oteryn_platform_mtls_remove_fail_closed() {
    OTERYN_PLATFORM_MTLS_STOPPED_ID=''
    "$@" rm -s -f internal-mtls >&2 || \
        echo "Unable to remove the Game-node mTLS listener after a failed gate." >&2
    return 0
}

# Starts the given ingress container ids again (the output of oteryn_native_admission_ingress_stop,
# or a route's preserved ids), and the Game-node mTLS listener this shell's ingress stop kept.
# Only call it after the trust gate passed.
oteryn_native_admission_ingress_start() {
    local id
    local -a ids=()
    for id in "$@" "$OTERYN_PLATFORM_MTLS_STOPPED_ID"; do
        [[ -z "$id" || " ${ids[*]} " == *" $id "* ]] || ids+=("$id")
    done
    ((${#ids[@]} == 0)) || command docker start "${ids[@]}" >/dev/null || return 1
    OTERYN_PLATFORM_MTLS_STOPPED_ID=''
}

# The trust gate. The arguments are the route's Compose command words (for example
# "${compose[@]}"); ingress must already be stopped. When witness preparation or publication fails,
# Platform, the internal proxy and Gateway are removed (fail closed): a stopped container with
# `restart: always` would come back on the next Docker daemon or NAS restart. The caller aborts
# before ingress, health-check, promotion or scheduler restart. Volumes are kept; a later deploy or
# rollback recreates the containers only through this gate.
oteryn_publish_native_admission_trust_or_stop() {
    (($# > 0)) || { echo "Native admission trust gate needs the Compose command." >&2; return 1; }
    if oteryn_publish_native_admission_trust "$("$@" ps -q platform)"; then
        return 0
    fi
    echo "Native admission trust gate failed; removing the Platform, internal proxy and Gateway containers (fail closed)." >&2
    oteryn_platform_mtls_remove_fail_closed "$@"
    "$@" rm -s -f gateway internal-proxy platform >&2 || \
        echo "Unable to remove Platform, internal proxy and Gateway after the failed trust gate." >&2
    return 1
}

# Self-check gate for the Platform that will really serve: oteryn_native_admission_self_check_or_stop
# <compose command words>. The running Platform container's own environment decides whether admission
# is on (a failed docker call is a gate failure, never "disabled"); when it is on the signing and
# projection self-check must pass. Any failure removes Platform, the internal proxy and Gateway (they
# are `restart: always`), so no restartable ingress stays behind an unproven Platform.
oteryn_native_admission_self_check_or_stop() {
    local id env_out
    (($# > 0)) || { echo "Native admission self-check gate needs the Compose command." >&2; return 1; }
    if id="$("$@" ps -q platform)" && [[ -n "$id" ]] \
        && env_out="$(command docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$id")" \
        && { ! grep -qx 'GAME_AUTH_NATIVE_ADMISSION_ENABLED=true' <<<"$env_out" || oteryn_run_native_admission_self_check "$id"; }; then
        # This is the last gate of every route that runs it, on the Platform that will serve: the
        # Game-node mTLS listener this shell's ingress stop kept returns now (marketplace-staging.sh
        # returns the internal proxy and Gateway by hand, without oteryn_native_admission_ingress_start).
        oteryn_native_admission_ingress_start || {
            echo "Native admission self-check passed, but the Game-node mTLS listener could not be started again." >&2
            return 1
        }
        return 0
    fi
    echo "Native admission self-check gate failed; removing the Platform, internal proxy and Gateway containers (fail closed)." >&2
    oteryn_platform_mtls_remove_fail_closed "$@"
    "$@" rm -s -f gateway internal-proxy platform >&2 || \
        echo "Unable to remove Platform, internal proxy and Gateway after the failed self-check gate." >&2
    return 1
}

# Failure restore of the previous Platform for the fast-deploy and Platform-reconcile traps:
# oteryn_restore_previous_platform_behind_trust_gate <label>, with the previous release's images
# exported. Same order as a release: ingress stopped, previous Platform recreated, trust gate, then
# the internal proxy and Gateway (started again, or recreated when a failed forward gate removed
# them) and the scheduler. Any failure leaves Platform, internal proxy and Gateway removed.
oteryn_restore_previous_platform_behind_trust_gate() {
    local label="$1"
    local -a base=(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")
    if ! oteryn_native_admission_ingress_stop "${base[@]}" >/dev/null 2>&1; then
        oteryn_platform_mtls_remove_fail_closed "${base[@]}" 2>/dev/null
        "${base[@]}" rm -s -f gateway internal-proxy platform >/dev/null 2>&1 || true
        echo "$label could not stop the internal proxy and Gateway; it removed the runtime containers (fail closed)." >&2
        return 1
    fi
    "${base[@]}" up -d --no-deps --force-recreate platform >/dev/null 2>&1 || \
        echo "$label could not recreate the previous Platform container." >&2
    if ! oteryn_publish_native_admission_trust_or_stop "${base[@]}"; then
        echo "$label left Platform, internal proxy and Gateway removed: the previous Platform failed the native admission trust gate." >&2
        return 1
    fi
    local rc=0
    # The Game-node mTLS listener is reconciled, not just started: a failed forward gate may already
    # have removed it (oteryn_platform_mtls_remove_fail_closed), leaving no kept id and no container.
    # Recreated while the setting is on, removed while it is off.
    { "${base[@]}" up -d --no-deps internal-proxy gateway && oteryn_reconcile_platform_mtls "${base[@]}"; } >/dev/null 2>&1 || {
        echo "$label could not start the internal proxy, Gateway and Game-node mTLS listener." >&2
        rc=1
    }
    _oteryn_reconcile_marketplace_scheduler_after_runtime_change >/dev/null 2>&1 || {
        echo "$label could not reconcile the previous Marketplace runtime." >&2
        rc=1
    }
    return "$rc"
}

# Native (protocol_version 2) login needs the Gateway branch and the Platform issuer switched
# together. A Gateway that accepts the branch while the issuer cannot sign would pass deploy with
# every login answering NATIVE_LOGIN_UNAVAILABLE. A login probe with an unknown ticket is rejected
# before the issuer touches its signing key, so it cannot prove issuance works; only the Platform
# signing self-check (oteryn_run_native_admission_self_check) does, and deploy and health-check
# both run it whenever the flag is true.
oteryn_assert_native_login_pairing() {
    local platform_flag="$1"
    local gateway_flag="$2"

    case "$platform_flag" in
        true | false) ;;
        *)
            echo "Platform native admission flag is missing or invalid; the deployment must set it explicitly." >&2
            return 1
            ;;
    esac
    if [[ "$gateway_flag" != "$platform_flag" ]]; then
        echo "Gateway native login (${gateway_flag:-unset}) does not match Platform native admission ($platform_flag)." >&2
        return 1
    fi
}

# Creates the issuer signing key volume content when it does not exist yet and proves the key id and
# public key can be read back. Prints only key ids and PUBLIC keys (for Game's trust config), never
# the private key. Fails closed.
oteryn_ensure_native_admission_key() {
    local script_dir="${SCRIPT_DIR:-$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)}"
    local output

    if ! output="$(PLATFORM_IMAGE="$PLATFORM_IMAGE" bash "$script_dir/generate-native-admission-key.sh" ensure)"; then
        echo "Native admission signing key custody is not usable; refusing to deploy." >&2
        return 1
    fi
    if ! grep -Eq '^current_key_id=[A-Za-z0-9._-]{1,64}$' <<<"$output" \
        || ! grep -Eq '^current_public_key=[A-Za-z0-9_-]{43}$' <<<"$output"; then
        echo "Native admission signing key custody did not report a current key id and public key." >&2
        return 1
    fi
    echo "Native admission signing key present; publish these PUBLIC values to Game's trust config:"
    printf '%s\n' "$output"
}

# Runs the Platform signing self-check in the given running Platform container: signs a throwaway
# grant with the configured key and verifies it against the trust registry. Exit 0 only when the
# issuer can really sign. The command prints one JSON line with the key id or one reason line, never
# key material.
oteryn_run_native_admission_self_check() {
    local container_id="$1"
    local output

    if [[ -z "$container_id" ]]; then
        echo "Native admission self-check needs the running Platform container." >&2
        return 1
    fi
    if ! output="$(docker exec "$container_id" /usr/local/bin/oteryn-platform-entrypoint \
        php artisan game-auth:native-admission:self-check --no-interaction 2>&1)"; then
        echo "Native admission signing self-check failed; refusing to enable native game login: ${output}" >&2
        return 1
    fi
    echo "Native admission signing self-check passed: ${output}"

    # The deploy gate only proves the identity variables are non-empty. Prove the running Platform really
    # accepts them with the application's own settings logic (valid JSON, scope format, no overlap with
    # another purpose's identity): a null result means ingestion answers 503 and no scope routes.
    if ! docker exec "$container_id" /usr/local/bin/oteryn-platform-entrypoint php -r '
        require "vendor/autoload.php";
        $app = require "bootstrap/app.php";
        $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap();
        exit(App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusSettings::current() !== null
            && App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentSettings::current() !== null ? 0 : 1);
    ' >/dev/null 2>&1; then
        echo "Native admission projection identities are invalid in the running Platform (runtime status or scope assignment settings are not usable); refusing to enable native game login." >&2
        return 1
    fi
}

# Proves the running Platform holds a fresh account-character projection (watermark received from the
# Character Authority within the freshness bound) before native admission is left enabled while the D171
# fallback is off: without it every authenticated login is refused although the unknown-ticket health
# probe passes. Polls the application's own read model (feedEvidence) for up to
# OTERYN_NATIVE_ACCOUNT_CHARACTERS_FEED_WAIT_SECONDS (default 90, 0..600) and fails closed otherwise.
# Not part of the stop gate: the Game node can only push while Platform and its mTLS listener are up.
oteryn_wait_native_account_characters_feed() {
    local container_id="$1" wait_seconds="${OTERYN_NATIVE_ACCOUNT_CHARACTERS_FEED_WAIT_SECONDS:-90}"
    local deadline evidence=unavailable

    if [[ -z "$container_id" ]]; then
        echo "Account-character feed check needs the running Platform container." >&2
        return 1
    fi
    if [[ ! "$wait_seconds" =~ ^[0-9]+$ ]] || ((wait_seconds > 600)); then
        echo "OTERYN_NATIVE_ACCOUNT_CHARACTERS_FEED_WAIT_SECONDS must be an integer from 0 to 600." >&2
        return 1
    fi
    deadline=$((SECONDS + wait_seconds))
    while :; do
        evidence="$(docker exec "$container_id" /usr/local/bin/oteryn-platform-entrypoint php -r '
            require "vendor/autoload.php";
            $app = require "bootstrap/app.php";
            $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap();
            echo (new App\GameAuth\NativeAccountCharacters\NativeAccountCharactersReadModel())->feedEvidence(time());
        ' 2>/dev/null | tail -n 1)" || evidence=unavailable
        if [[ "$evidence" == live ]]; then
            echo "Account-character projection feed is live (fresh watermark received)."
            return 0
        fi
        ((SECONDS < deadline)) || break
        sleep 3
    done
    echo "Account-character projection feed is ${evidence:-unavailable}: no fresh watermark from the Character Authority within ${wait_seconds}s, so every native login would be refused; refusing to report the deployment healthy. Check the Game node projection push (CN=oteryn-preprod-character-projection)." >&2
    return 1
}

# Validates the projection settings native login needs besides a signing key: the Game runtime-status
# and scope-assignment identities that make a scope routable. No native World UUID is needed: account
# characters are on in compose.yml, so the D171 fallback is off. Reads the loaded environment; prints the reason to stderr and fails when incomplete.
oteryn_native_admission_projection_ready() {
    if [[ "${GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED:-}" != true || -z "${GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES:-}" \
        || "${GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED:-}" != true || -z "${GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES:-}" ]]; then
        echo "Native admission needs OTERYN_STAGING_NATIVE_RUNTIME_STATUS_IDENTITIES and OTERYN_STAGING_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES (repository variables or deploy/synology/native-defaults.env); without them no scope is routable and every login would fail." >&2
        return 1
    fi
}

# Prints the native admission flag a run of the given Platform image may use: true only when the
# environment asks for native admission AND the image can both run the self-check command and export
# the signing keys from the key volume (entrypoint load_native_admission_keys). An image that has the
# command but predates the key export would fail the self-check, so rollback and previous-candidate
# recovery of such a release run with native login off.
oteryn_native_admission_flag_for_image() {
    local image="$1"

    if [[ "${OTERYN_NATIVE_ADMISSION_ENABLED:-false}" == true ]] \
        && oteryn_native_admission_projection_ready 2>/dev/null \
        && docker run --rm --network none --entrypoint grep "$image" -q load_native_admission_keys \
            /usr/local/bin/oteryn-platform-entrypoint 2>/dev/null \
        && docker run --rm --network none --entrypoint php "$image" artisan list --raw 2>/dev/null \
            | awk '/^game-auth:native-admission:self-check /{a=1} /^game-auth:native-admission:publish-trust /{b=1} END{exit !(a&&b)}'; then
        echo true
    else
        echo false
    fi
}

# Pins the compatibility flag for an image into a bounded runtime env file, so every Compose command
# that reads that file (including the Marketplace reconciler, which builds its own command from
# OTERYN_ENV_FILE) recreates Platform and Gateway with the same flag.
oteryn_pin_native_admission_flag() {
    local env_file="$1" image="$2" flag
    flag="$(oteryn_native_admission_flag_for_image "$image")"
    { grep -v '^OTERYN_NATIVE_ADMISSION_ENABLED=' "$env_file" || true; printf 'OTERYN_NATIVE_ADMISSION_ENABLED=%s\n' "$flag"; } >"$env_file.pin"
    chmod 600 "$env_file.pin"
    mv "$env_file.pin" "$env_file"
}

trap _oteryn_finalize_release_on_exit EXIT
