#!/usr/bin/env bash
# Clean-slate Platform bootstrap on PostgreSQL (ADR 0046; PLATFORM_POSTGRES_MIGRATION_PLAN.md, P3).
#
# A fresh PostgreSQL database plus an empty native-evidence witness directory reach a working
# Platform with one procedure. No data is carried over from MariaDB (owner answer Q1). The runbook
# is docs/operations/PLATFORM_POSTGRES_BOOTSTRAP.md.
#
#   bootstrap-postgres.sh record-witness [--local]
#       At the freeze, with the platform and scheduler stopped: records the MariaDB-era witness
#       directory (its witness-store.id and a manifest of every file) in the bootstrap state
#       directory. Writes nothing in the witness directory.
#   bootstrap-postgres.sh run [--local] [--with-native-admission (--local only)]
#       Runs plan steps 1 to 8 against the PostgreSQL engine. Every refusal check runs before the
#       first change. Refuses on a non-empty target database, on any witness binding, trust or
#       native evidence row, on a witness directory that is not empty and does not match the
#       freeze record exactly, and when the new witness directory is not empty after the move.
#
# --local runs php artisan in a Platform checkout (OTERYN_PG_BOOTSTRAP_PLATFORM_ROOT, default this
# repository) with the caller's environment and a local witness directory; CI and the tests use it.
# Without it the script drives the Synology Compose stack (OTERYN_ENV_FILE, default
# deploy/synology/.env). The script never drops, empties or deletes a database or a witness
# directory, and it never edits the MariaDB-era witness; it only moves it aside to a dated name.
#
# Exit status: 0 bootstrapped; 1 refused before any change; 2 failed after a change began, so only
# a clean-slate re-run (emptied database) is possible, and only before the first witnessed write.
set -euo pipefail
umask 077

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
REPOSITORY_ROOT="$(cd -- "$DEPLOY_DIR/../.." && pwd)"

usage() {
    echo "Usage: bootstrap-postgres.sh record-witness [--local]" >&2
    echo "       bootstrap-postgres.sh run [--local] [--with-native-admission]" >&2
}

refuse() {
    printf 'Refused: %s\n' "$*" >&2
    exit 1
}

subcommand="${1:-}"
case "$subcommand" in
    record-witness | run) shift ;;
    *) usage; exit 1 ;;
esac

local_mode=false
with_native_admission=false
while (($# > 0)); do
    case "$1" in
        --local) local_mode=true ;;
        --with-native-admission)
            [[ "$subcommand" == run ]] || { usage; exit 1; }
            with_native_admission=true
            ;;
        *) usage; exit 1 ;;
    esac
    shift
done

if [[ "$local_mode" == true ]]; then
    PLATFORM_ROOT="${OTERYN_PG_BOOTSTRAP_PLATFORM_ROOT:-$REPOSITORY_ROOT}"
    PHP_BIN="${OTERYN_PG_BOOTSTRAP_PHP:-php}"
    [[ -f "$PLATFORM_ROOT/artisan" ]] || refuse "no Platform checkout at $PLATFORM_ROOT."
    state_dir="${OTERYN_PG_BOOTSTRAP_STATE_DIR:-}"
    [[ -n "$state_dir" ]] || refuse "--local needs OTERYN_PG_BOOTSTRAP_STATE_DIR."
else
    # The test seam below exists only for --local; the Synology stack never honours it.
    [[ -z "${OTERYN_PG_BOOTSTRAP_LOCAL_TEST_PLANT_AFTER_MOVE:-}" ]] \
        || refuse "OTERYN_PG_BOOTSTRAP_LOCAL_TEST_PLANT_AFTER_MOVE is a --local test seam only."
    ENV_FILE="${OTERYN_ENV_FILE:-$DEPLOY_DIR/.env}"
    COMPOSE_FILE="$DEPLOY_DIR/compose.yml"
    # shellcheck source=deploy/synology/scripts/lib.sh
    source "$SCRIPT_DIR/lib.sh"
    load_oteryn_env_file "$ENV_FILE"
    compose=(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")
    state_dir="${OTERYN_PG_BOOTSTRAP_STATE_DIR:-${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}/postgres-bootstrap}"
fi

[[ "$state_dir" == /* ]] || refuse "the bootstrap state directory must be an absolute path."
# Validate without mutating: a refused invocation must leave the filesystem as it found it, so the
# directory is created, chmodded and locked only by prepare_state_dir, after every refusal check.
[[ ! -L "$state_dir" ]] \
    || refuse "the bootstrap state directory $state_dir must be an owner-only directory, not a link."
[[ ! -e "$state_dir" ]] || [[ -d "$state_dir" && -O "$state_dir" ]] \
    || refuse "the bootstrap state directory $state_dir must be an owner-only directory, not a link."
freeze_file="$state_dir/witness-freeze.env"
moved_file="$state_dir/witness-moved-aside.env"
complete_file="$state_dir/bootstrap-complete.env"
lock_dir="$state_dir/run.lock"

witnessed_write_started=false
failed() {
    printf 'Bootstrap failed: %s\n' "$*" >&2
    if [[ "$witnessed_write_started" == true ]]; then
        echo "The first witnessed write (native admission trust publication) already happened: there is no rollback and no re-run. Only forward reconciliation by the operator is possible (runbook). Never edit or reuse the moved-aside MariaDB-era directory." >&2
    else
        echo "A change already began. Before the first witnessed write the only way forward is a clean-slate re-run: empty the PostgreSQL database and the new witness directory (runbook). Never edit or reuse the moved-aside MariaDB-era directory." >&2
    fi
    exit 2
}

# The first mutation of either subcommand. Still a refusal (exit 1): nothing else has changed yet.
prepare_state_dir() {
    # Take the lock before the mode change: a refused run (stale or held lock) must not have chmodded an
    # existing directory. A missing directory is created owner-only from the start (umask 077), and no lock
    # can exist in a directory that did not exist.
    if [[ ! -e "$state_dir" ]]; then
        (umask 077; mkdir -p -- "$state_dir") || refuse "cannot create the bootstrap state directory $state_dir."
    fi
    [[ -d "$state_dir" && ! -L "$state_dir" && -O "$state_dir" ]] \
        || refuse "the bootstrap state directory $state_dir must be an owner-only directory, not a link."
    if ! mkdir -- "$lock_dir" 2>/dev/null; then
        refuse "another bootstrap-postgres.sh holds $lock_dir (or a stale lock is left); remove it only when no bootstrap is running. Nothing was changed."
    fi
    trap 'rmdir -- "$lock_dir" 2>/dev/null || true' EXIT
}

# After the lock and every re-check: the first change to an existing state directory.
restrict_state_dir() {
    chmod 700 -- "$state_dir" || refuse "cannot restrict the bootstrap state directory $state_dir to its owner."
}

# --- Witness directory access ---------------------------------------------------------------
# One POSIX sh program, run in the Platform image (Compose: a one-off container on the
# platform_storage volume, as the image user) or locally. It reads the directory from
# GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY and prints key=value lines.
# shellcheck disable=SC2016 # expanded by the witness shell, not here
WITNESS_PROGRAM='
set -eu
export LC_ALL=C
dir="${GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY:-}"

fail() { echo "witness_error=$1"; exit 3; }

inspect() {
    [ -n "$dir" ] || fail unset
    case "$dir" in /*) ;; *) fail relative ;; esac
    case "$dir" in */) fail trailing_slash ;; esac
    [ ! -L "$dir" ] || fail symlink
    if [ ! -e "$dir" ]; then
        state=empty; store_id=none; manifest=none
        return 0
    fi
    [ -d "$dir" ] || fail not_directory
    lines=""
    count=0
    # A glob loop, never a parsed ls: a file name made of newlines (or starting with a dash) must be seen,
    # and refused, not dropped by command substitution. The patterns cover every dotfile.
    for entry in "$dir"/.[!.]* "$dir"/..?* "$dir"/*; do
        [ -e "$entry" ] || [ -L "$entry" ] || continue
        name="${entry#"$dir"/}"
        case "$name" in "" | *[!A-Za-z0-9._-]*) fail unsupported_name ;; esac
        if [ -L "$entry" ] || [ ! -f "$entry" ]; then fail unsupported_entry; fi
        digest="$(sha256sum < "$entry" | cut -d " " -f 1)"
        lines="${lines}${name} ${digest}
"
        count=$((count + 1))
    done
    if [ "$count" -eq 0 ]; then
        state=empty; store_id=none; manifest=none
        return 0
    fi
    state=present
    manifest="$(printf %s "$lines" | sort | sha256sum | cut -d " " -f 1)"
    store_id=none
    if [ -e "$dir/witness-store.id" ]; then
        store_id="$(tr -d " \n\r\t" < "$dir/witness-store.id")"
        printf %s "$store_id" | grep -Eq "^[0-9a-f]{64}$" || fail malformed_store_id
    fi
}

report() {
    echo "witness_directory=$dir"
    echo "witness_state=$state"
    echo "witness_store_id=$store_id"
    echo "witness_manifest=$manifest"
}

new_directory() {
    mkdir -m 700 -- "$dir" || fail new_directory_exists
    if [ -n "${OTERYN_PG_BOOTSTRAP_LOCAL_TEST_PLANT_AFTER_MOVE:-}" ]; then
        : > "$dir/$OTERYN_PG_BOOTSTRAP_LOCAL_TEST_PLANT_AFTER_MOVE"
    fi
}

verify_new_directory() {
    [ -d "$dir" ] && [ ! -L "$dir" ] || fail new_directory_missing
    [ -O "$dir" ] || fail new_directory_owner
    [ "$(stat -c %a -- "$dir")" = 700 ] || fail new_directory_mode
    for entry in "$dir"/.[!.]* "$dir"/..?* "$dir"/*; do
        if [ -e "$entry" ] || [ -L "$entry" ]; then fail new_directory_not_empty; fi
    done
    echo "witness_new_directory=empty"
}

action="${1:-}"
case "$action" in
    inspect)
        inspect
        report
        ;;
    move-aside)
        expected_manifest="${2:-}"; expected_store_id="${3:-}"; target="${4:-}"
        inspect
        [ "$state" = present ] || fail not_present
        [ "$manifest" = "$expected_manifest" ] && [ "$store_id" = "$expected_store_id" ] || fail changed_since_check
        case "$target" in "$dir".mariadb-era-*) ;; *) fail bad_target ;; esac
        [ ! -e "$target" ] && [ ! -L "$target" ] || fail target_exists
        mv -- "$dir" "$target"
        echo "witness_moved_aside_to=$target"
        new_directory
        verify_new_directory
        ;;
    verify-aside)
        # The preserved MariaDB-era directory a retry depends on: still there, a real owned directory, and
        # byte-identical to what the freeze recorded (store id and manifest). A name alone proves nothing.
        target="${2:-}"; expected_manifest="${3:-}"; expected_store_id="${4:-}"
        case "$target" in "$dir".mariadb-era-*) ;; *) fail bad_target ;; esac
        dir="$target"
        [ -d "$dir" ] && [ ! -L "$dir" ] || fail aside_missing
        [ -O "$dir" ] || fail aside_owner
        inspect
        [ "$state" = present ] && [ "$manifest" = "$expected_manifest" ] && [ "$store_id" = "$expected_store_id" ] \
            || fail aside_changed
        echo "witness_aside=verified"
        ;;
    find-aside)
        # Recovery after a move whose record could not be written: the preserved directories of this witness
        # that still verify byte-identical against the freeze. Exactly one is adoptable.
        expected_manifest="${2:-}"; expected_store_id="${3:-}"
        found=0; found_path=""
        for candidate in "$dir".mariadb-era-*; do
            [ -e "$candidate" ] || [ -L "$candidate" ] || continue
            if (
                dir="$candidate"
                [ -d "$dir" ] && [ ! -L "$dir" ] && [ -O "$dir" ] || exit 1
                inspect
                [ "$state" = present ] && [ "$manifest" = "$expected_manifest" ] && [ "$store_id" = "$expected_store_id" ]
            ) >/dev/null 2>&1; then
                found=$((found + 1)); found_path="$candidate"
            fi
        done
        echo "witness_aside_count=$found"
        [ "$found" -ne 1 ] || echo "witness_aside_path=$found_path"
        ;;
    prepare-empty)
        inspect
        [ "$state" = empty ] || fail changed_since_check
        if [ ! -e "$dir" ]; then
            new_directory
        else
            chmod 700 -- "$dir"
        fi
        verify_new_directory
        ;;
    *)
        fail unknown_action
        ;;
esac
'

witness() {
    if [[ "$local_mode" == true ]]; then
        sh -c "$WITNESS_PROGRAM" witness "$@"
    else
        "${compose[@]}" run --rm --no-deps -T --entrypoint sh platform -c "$WITNESS_PROGRAM" witness "$@"
    fi
}

# Prints the value of key from key=value output, or nothing.
value_of() {
    local key="$1" text="$2"
    sed -n "s/^${key}=//p" <<<"$text" | tail -n 1
}

# --- Platform access ----------------------------------------------------------------------
# platform_oneoff: before the platform service runs (Compose: a one-off container that also gets
# the migration owner credentials). platform_exec: in the running platform service.
platform_oneoff() {
    if [[ "$local_mode" == true ]]; then
        (cd -- "$PLATFORM_ROOT" && "$PHP_BIN" "$@")
    else
        "${compose[@]}" run --rm --no-deps -T \
            -e DB_MIGRATIONS_USERNAME -e DB_MIGRATIONS_PASSWORD \
            -e OTERYN_PLATFORM_FASTCGI_ENABLED=false platform php "$@"
    fi
}

platform_exec() {
    if [[ "$local_mode" == true ]]; then
        (cd -- "$PLATFORM_ROOT" && "$PHP_BIN" "$@")
    else
        "${compose[@]}" exec -T platform php "$@"
    fi
}

# Read-only probe of the target engine and database, run through the Platform's own configuration
# (one-off, because it reads pending migrations on the pgsql_migrations owner connection). The one-off
# container runs the image entrypoint as PID 1, which with FastCGI enabled would start PHP-FPM beside the
# command and turn its normal exit into status 9; platform_oneoff therefore forces the pool off. It prints probe_<key>=<value> lines and never writes.
# shellcheck disable=SC2016 # PHP source
PROBE_PROGRAM='
$mode = $argv[1] ?? "";
require "vendor/autoload.php";
$app = require "bootstrap/app.php";
$app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap();
$emit = static function (string $key, string|int $value): void {
    echo "probe_{$key}=".preg_replace("/[^A-Za-z0-9._:-]/", "?", (string) $value).PHP_EOL;
};
$default = config("database.default");
$default = is_string($default) && preg_match("/^[A-Za-z0-9_]+$/", $default) === 1 ? $default : "";
$runtimeDriver = $default === "" ? "" : (string) config("database.connections.{$default}.driver");
$ownerDriver = (string) config("database.connections.pgsql_migrations.driver");
$emit("runtime_connection", $default === "" ? "invalid" : $default);
$emit("runtime_driver", $runtimeDriver === "" ? "none" : $runtimeDriver);
$emit("owner_driver", $ownerDriver === "" ? "none" : $ownerDriver);
$emit("app_env", (string) app()->environment());
$emit("native_admission", config("game-auth.native_admission.enabled") === true ? "true" : "false");
if ($mode === "preflight") {
    // The same host validation as App\Console\Commands\EnsureGameWorld::validHost (step 6), so a value it would
    // reject is refused before the first change.
    $host = (string) ($argv[2] ?? "");
    $emit("host_valid", $host !== "" && mb_strlen($host) <= 255 && !str_contains($host, "://") && !str_contains($host, "/")
        && (filter_var($host, FILTER_VALIDATE_IP) !== false || filter_var($host, FILTER_VALIDATE_DOMAIN, FILTER_FLAG_HOSTNAME) !== false)
        ? "true" : "false");
}
if ($runtimeDriver !== "pgsql" || $ownerDriver !== "pgsql") {
    $emit("result", "wrong_engine");
    exit(0);
}
try {
    $owner = Illuminate\Support\Facades\DB::connection("pgsql_migrations");
    $runtime = Illuminate\Support\Facades\DB::connection($default);
    $ownerDatabase = $owner->selectOne("select current_database() as name, current_setting(\x27server_version_num\x27) as version");
    $runtimeDatabase = $runtime->selectOne("select current_database() as name");
} catch (Throwable $exception) {
    $emit("result", "unreachable");
    $emit("error", class_basename($exception));
    exit(0);
}
$same = $ownerDatabase->name === $runtimeDatabase->name
    && (string) $owner->getConfig("host") === (string) $runtime->getConfig("host")
    && (string) $owner->getConfig("port") === (string) $runtime->getConfig("port");
$emit("same_database", $same ? "true" : "false");
$emit("server_version_num", (string) $ownerDatabase->version);
if ($mode === "preflight") {
    $userSchemas = "n.nspname not in (\x27pg_catalog\x27, \x27information_schema\x27) and n.nspname not like \x27pg\\_toast%\x27 and n.nspname not like \x27pg\\_temp\\_%\x27";
    $relations = $owner->selectOne("select count(*) as n from pg_catalog.pg_class c join pg_catalog.pg_namespace n on n.oid = c.relnamespace where c.relkind in (\x27r\x27, \x27p\x27, \x27v\x27, \x27m\x27, \x27S\x27, \x27f\x27) and {$userSchemas}");
    $emit("relations", (int) $relations->n);
    $rows = ["witness_binding_rows" => 0, "trust_rows" => 0, "evidence_rows" => 0];
    $tables = $owner->select("select n.nspname as schema_name, c.relname as table_name from pg_catalog.pg_class c join pg_catalog.pg_namespace n on n.oid = c.relnamespace where c.relkind in (\x27r\x27, \x27p\x27) and {$userSchemas} and (c.relname like \x27native\\_game\\_evidence\\_%\x27 or c.relname like \x27native\\_game\\_signing\\_trust\\_%\x27)");
    foreach ($tables as $table) {
        if (preg_match("/^[a-z0-9_]+$/", $table->schema_name) !== 1 || preg_match("/^[a-z0-9_]+$/", $table->table_name) !== 1) {
            $emit("native_rows", "unreadable");
            exit(0);
        }
        try {
            $present = (bool) $owner->selectOne("select exists (select 1 from \"{$table->schema_name}\".\"{$table->table_name}\") as present")->present;
        } catch (Throwable) {
            $emit("native_rows", "unreadable");
            exit(0);
        }
        $category = $table->table_name === "native_game_evidence_witness_stores" ? "witness_binding_rows"
            : (str_starts_with($table->table_name, "native_game_signing_trust_") ? "trust_rows" : "evidence_rows");
        $rows[$category] += $present ? 1 : 0;
    }
    foreach ($rows as $key => $count) {
        $emit($key, $count);
    }
    $emit("result", "ok");
    exit(0);
}
if ($mode === "verify") {
    $migrator = app("migrator");
    $migrator->setConnection("pgsql_migrations");
    $repository = $migrator->getRepository();
    $files = $migrator->getMigrationFiles(array_merge([database_path("migrations")], $migrator->paths()));
    $ran = $repository->repositoryExists() ? $repository->getRan() : [];
    $emit("pending_migrations", count(array_diff(array_keys($files), $ran)));
    $emit("admin_roles", $runtime->table("admin_roles")->count());
    $emit("admin_role_permissions", $runtime->table("admin_role_permissions")->count());
    $emit("oauth_clients", $runtime->table("oauth_clients")->count());
    $worldId = $argv[2] ?? "";
    $emit("world_row", preg_match("/^[1-9][0-9]*$/", $worldId) === 1 && $runtime->table("game_worlds")->where("id", (int) $worldId)->exists() ? "present" : "missing");
    $emit("witness_binding_rows", $runtime->table("native_game_evidence_witness_stores")->count());
    $emit("trust_profiles", $runtime->table("native_game_signing_trust_profiles")->count());
    $emit("result", "ok");
    exit(0);
}
$emit("result", "unknown_mode");
'

probe() {
    local output
    output="$(platform_oneoff -r "$PROBE_PROGRAM" "$@" 2>&1)" || true
    grep -E '^probe_[a-z_]+=' <<<"$output" | sed 's/^probe_//' || true
}

# --- Freeze record --------------------------------------------------------------------------
read_record_key() {
    local file="$1" key="$2"
    sed -n "s/^${key}=//p" "$file" | tail -n 1
}

# services_must_be_stopped <service>...: the witness writers (platform, marketplace-scheduler) and, for
# run, the ingress (internal-proxy, gateway, internal-mtls) must be stopped. The bootstrap starts only
# platform and leaves ingress to the deploy that follows, which returns it behind the trust gate.
services_must_be_stopped() {
    [[ "$local_mode" == true ]] && return 0
    local service id running
    for service in "$@"; do
        id="$("${compose[@]}" ps -q "$service")" || refuse "cannot read the state of the $service service."
        [[ -z "$id" ]] && continue
        running="$(docker inspect --format '{{.State.Running}}' "$id")" || refuse "cannot inspect the $service container."
        [[ "$running" != true ]] || refuse "the $service service is running; stop $* first (runbook, plan P6 step 1)."
    done
}

inspect_witness() {
    local output
    output="$(witness inspect)" || true
    if [[ -n "$(value_of witness_error "$output")" ]]; then
        refuse "the native evidence witness directory cannot be used ($(value_of witness_error "$output"))."
    fi
    witness_dir="$(value_of witness_directory "$output")"
    witness_state="$(value_of witness_state "$output")"
    witness_store_id="$(value_of witness_store_id "$output")"
    witness_manifest="$(value_of witness_manifest "$output")"
    [[ "$witness_state" =~ ^(empty|present)$ && -n "$witness_dir" ]] \
        || refuse "the native evidence witness directory inspection returned no state."
}

record_witness() {
    services_must_be_stopped platform marketplace-scheduler
    [[ ! -e "$moved_file" ]] \
        || refuse "this bootstrap already moved the MariaDB-era witness aside ($moved_file); the current directory is not the MariaDB-era one. After a P6 rollback, rename the state directory $state_dir before the next freeze (runbook)."
    inspect_witness
    if [[ "$witness_state" == present && "$witness_store_id" == none ]]; then
        refuse "the witness directory $witness_dir is not empty and carries no witness-store.id; only an empty directory or one with a store id can be recorded."
    fi
    local record
    record="$(printf 'RECORD_VERSION=1\nWITNESS_DIRECTORY=%s\nWITNESS_STATE=%s\nWITNESS_STORE_ID=%s\nWITNESS_MANIFEST_SHA256=%s\n' \
        "$witness_dir" "$witness_state" "$witness_store_id" "$witness_manifest")"
    if [[ -e "$freeze_file" ]]; then
        if [[ "$(grep -v '^FROZEN_AT=' "$freeze_file")" == "$record" ]]; then
            echo "MariaDB-era witness freeze record unchanged: $freeze_file"
            return 0
        fi
        refuse "a different freeze record already exists ($freeze_file). Remove it only after confirming that the MariaDB-era Platform, and nothing else, wrote the witness after that freeze (runbook)."
    fi
    [[ ! -e "$freeze_file.tmp" && ! -L "$freeze_file.tmp" ]] \
        || refuse "$freeze_file.tmp already exists; remove the stale temporary record first."
    prepare_state_dir
    restrict_state_dir
    # The state directory exists now: a write failure is a change that began, so it exits 2, removes its own
    # temporary file (a stale one would block the next attempt) and is safe to re-run.
    printf '%s\nFROZEN_AT=%s\n' "$record" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$freeze_file.tmp" \
        && chmod 600 "$freeze_file.tmp" \
        && mv -- "$freeze_file.tmp" "$freeze_file" \
        || { rm -f -- "$freeze_file.tmp"; failed "the freeze record $freeze_file could not be written; no witness was touched, and record-witness can be re-run."; }
    echo "Recorded the MariaDB-era witness: state=$witness_state store_id=$witness_store_id ($freeze_file)."
}

if [[ "$subcommand" == record-witness ]]; then
    record_witness
    exit 0
fi

# --- run: refusal checks (no change yet) ---------------------------------------------------
# The whole refusal block is a function so it can run twice: once before any change, and again with the lock
# held, because a concurrent run may have migrated the database or advanced the witness between the first pass
# and the lock. Decisions are never taken on a probe older than the lock.
run_refusal_checks() {
reasons=()
[[ -f "$freeze_file" ]] || refuse "no MariaDB-era witness freeze record ($freeze_file); run record-witness at the freeze first."
[[ ! -e "$complete_file" ]] \
    || refuse "this state directory already records a completed bootstrap ($complete_file); a re-run is refused."
frozen_dir="$(read_record_key "$freeze_file" WITNESS_DIRECTORY)"
frozen_state="$(read_record_key "$freeze_file" WITNESS_STATE)"
frozen_store_id="$(read_record_key "$freeze_file" WITNESS_STORE_ID)"
frozen_manifest="$(read_record_key "$freeze_file" WITNESS_MANIFEST_SHA256)"
[[ "$(read_record_key "$freeze_file" RECORD_VERSION)" == 1 \
    && "$frozen_state" =~ ^(empty|present)$ \
    && "$frozen_store_id" =~ ^([0-9a-f]{64}|none)$ \
    && "$frozen_manifest" =~ ^([0-9a-f]{64}|none)$ ]] || refuse "the freeze record $freeze_file is malformed."

services_must_be_stopped platform marketplace-scheduler internal-proxy gateway internal-mtls
inspect_witness
[[ "$witness_dir" == "$frozen_dir" ]] \
    || refuse "the witness directory $witness_dir differs from the one recorded at the freeze ($frozen_dir)."

witness_action=''
moved_to=''
adopt_aside_to=''
if [[ "$witness_state" == present ]]; then
    if [[ -e "$moved_file" ]]; then
        reasons+=("the witness directory is not empty although this bootstrap already moved the MariaDB-era one aside. Unless the operator proves that no trust publication or provenance init ran, a PostgreSQL witness wrote to it: the first witnessed write happened and a re-run is impossible (forward reconciliation only).")
    elif [[ "$witness_store_id" != none && "$witness_store_id" != "$frozen_store_id" ]]; then
        reasons+=("the witness directory carries witness-store.id $witness_store_id, not the MariaDB-era id recorded at the freeze ($frozen_store_id): a PostgreSQL witness wrote it (provenance init or trust publication), so a re-run is impossible.")
    elif [[ "$witness_store_id" == none ]]; then
        reasons+=("the witness directory is not empty and carries no witness-store.id; it is neither empty nor the recorded MariaDB-era directory (a failed trust publication can leave a floor without a database row).")
    elif [[ "$frozen_state" != present || "$witness_manifest" != "$frozen_manifest" ]]; then
        reasons+=("the witness directory carries the recorded store id but its files changed since the freeze: something advanced it after the freeze (for example a trust publication whose database transaction rolled back), so it is not moved aside.")
    else
        witness_action=move-aside
    fi
else
    if [[ "$frozen_state" == present && ! -e "$moved_file" ]]; then
        # An earlier attempt may have moved the directory aside and then failed to write its record (disk full,
        # I/O error). Adopt the preserved directory only when exactly one verifies byte-identical to the freeze.
        aside_found="$(witness find-aside "$frozen_manifest" "$frozen_store_id")" || true
        if [[ "$(value_of witness_aside_count "$aside_found")" == 1 && -n "$(value_of witness_aside_path "$aside_found")" ]]; then
            adopt_aside_to="$(value_of witness_aside_path "$aside_found")"
            moved_to="$adopt_aside_to"
        else
            reasons+=("the freeze recorded a non-empty MariaDB-era witness directory, but it is empty now and no intact preserved copy of it was found (verified copies: $(value_of witness_aside_count "$aside_found")); this bootstrap did not move it aside.")
        fi
    elif [[ -e "$moved_file" ]]; then
        moved_to="$(read_record_key "$moved_file" MOVED_ASIDE_TO)"
        if [[ "$moved_to" != "$frozen_dir".mariadb-era-* \
            || "$(read_record_key "$moved_file" WITNESS_STORE_ID)" != "$frozen_store_id" \
            || "$frozen_state" != present || "$frozen_store_id" == none || "$frozen_manifest" == none ]]; then
            reasons+=("the moved-aside record $moved_file is malformed or does not match the freeze record.")
        else
            # A retry after a failed attempt depends on the rollback witness still being intact.
            aside_output="$(witness verify-aside "$moved_to" "$frozen_manifest" "$frozen_store_id")" || true
            [[ "$(value_of witness_aside "$aside_output")" == verified ]] \
                || reasons+=("the preserved MariaDB-era witness $moved_to is missing, replaced or modified ($(value_of witness_error "$aside_output")); the rollback witness is lost, so the bootstrap is refused.")
        fi
    fi
    witness_action=prepare-empty
fi

# EnsureGameWorld trims every option (PHP trim: space, \t, \n, \r, NUL, \v) before validating it, so a
# whitespace-only value is empty there. Apply the same trim here, so the checks below see what step 6 sees.
trim_world_option() {
    local value="${!1:-}"
    value="${value#"${value%%[!$' \t\n\r\v']*}"}"
    value="${value%"${value##*[!$' \t\n\r\v']}"}"
    printf -v "$1" '%s' "$value"
}
for name in GAME_WORLD_ID GAME_WORLD_SLUG GAME_WORLD_NAME GAME_WORLD_REGION GAME_WORLD_HOST GAME_WORLD_PORT; do
    trim_world_option "$name"
done
probe_output="$(probe preflight "${GAME_WORLD_HOST:-}")"
runtime_driver="$(value_of runtime_driver "$probe_output")"
owner_driver="$(value_of owner_driver "$probe_output")"
probe_result="$(value_of result "$probe_output")"
app_env="$(value_of app_env "$probe_output")"
native_admission="$(value_of native_admission "$probe_output")"
if [[ -z "$probe_result" ]]; then
    reasons+=("the Platform could not inspect the target database.")
elif [[ "$probe_result" == wrong_engine ]]; then
    reasons+=("the target is not PostgreSQL (runtime connection driver ${runtime_driver:-none}, pgsql_migrations driver ${owner_driver:-none}); this bootstrap acts only on the PostgreSQL engine.")
elif [[ "$probe_result" == unreachable ]]; then
    reasons+=("the target PostgreSQL database is unreachable through the runtime or the pgsql_migrations connection ($(value_of error "$probe_output")).")
else
    [[ "$(value_of same_database "$probe_output")" == true ]] \
        || reasons+=("the runtime and pgsql_migrations connections do not point at the same PostgreSQL database.")
    native_rows="$(value_of native_rows "$probe_output")"
    [[ "$native_rows" != unreadable ]] || reasons+=("the native evidence and trust tables of the target database cannot be read.")
    [[ "$(value_of witness_binding_rows "$probe_output")" == 0 ]] \
        || reasons+=("the target database already has a native evidence witness-store binding (native_game_evidence_witness_stores).")
    [[ "$(value_of trust_rows "$probe_output")" == 0 ]] \
        || reasons+=("the target database already has native signing trust rows (native_game_signing_trust_*).")
    [[ "$(value_of evidence_rows "$probe_output")" == 0 ]] \
        || reasons+=("the target database already has native evidence rows (native_game_evidence_*).")
    relations="$(value_of relations "$probe_output")"
    [[ "$relations" == 0 ]] \
        || reasons+=("the target database is not empty (${relations:-unknown} relations); the bootstrap runs only on an empty database and a re-run is refused, never partially applied.")
fi

# The trust publication is the first witnessed write and needs the P6a safeguards first (a provenance-only
# witness-store init and a durable attempt record, plan "Trust safeguards (P6a)"). They are not implemented
# in this script, and cannot be done externally before it (preflight rejects an existing binding), so the
# operational (Compose) mode never publishes trust: native admission stays off and P6 step 4 owns the
# publication after P6a. --local keeps the mode so CI exercises the mechanics on a throwaway witness.
if [[ "$local_mode" != true && ( "$native_admission" == true || "$with_native_admission" == true ) ]]; then
    reasons+=("the operational bootstrap never publishes native admission trust: the P6a safeguards (provenance-only witness-store init, durable attempt record) are not part of this script. Turn native admission off for the bootstrap; trust is published in P6 step 4 after P6a. (--with-native-admission exists only with --local, for tests.)")
elif [[ "$native_admission" == true && "$with_native_admission" != true ]]; then
    reasons+=("native admission is on: its trust publication is the first witnessed write, after which no rollback or re-run is possible. Pass --with-native-admission only at that boundary (plan P6 step 4); otherwise turn native admission off.")
elif [[ "$native_admission" != true && "$with_native_admission" == true ]]; then
    reasons+=("--with-native-admission was given but native admission is off in the Platform configuration.")
fi

for name in GAME_WORLD_ID GAME_WORLD_SLUG GAME_WORLD_NAME GAME_WORLD_REGION GAME_WORLD_HOST GAME_WORLD_PORT; do
    [[ -n "${!name:-}" ]] || reasons+=("$name is not set (World Registry row for game-auth:world:ensure).")
done
# The same constraints as App\Console\Commands\EnsureGameWorld, checked before any trust
# publication: step 6 would otherwise reject a bad value after the first witnessed write.
if [[ -n "${GAME_WORLD_ID:-}" ]]; then
    [[ "$GAME_WORLD_ID" =~ ^[1-9][0-9]*$ ]] \
        || reasons+=("GAME_WORLD_ID must be a positive integer.")
    # Within the PHP integer range: beyond it EnsureGameWorld clamps the value and IssueNativeTopology
    # (FILTER_VALIDATE_INT) rejects it, after the first witnessed write.
    if [[ "$GAME_WORLD_ID" =~ ^[1-9][0-9]*$ ]] \
        && { (( ${#GAME_WORLD_ID} > 19 )) || { (( ${#GAME_WORLD_ID} == 19 )) && [[ "$GAME_WORLD_ID" > 9223372036854775807 ]]; }; }; then
        reasons+=("GAME_WORLD_ID must not exceed 9223372036854775807.")
    fi
fi
if [[ -n "${GAME_WORLD_PORT:-}" ]]; then
    { [[ "$GAME_WORLD_PORT" =~ ^[1-9][0-9]{0,4}$ ]] && (( 10#$GAME_WORLD_PORT <= 65535 )); } \
        || reasons+=("GAME_WORLD_PORT must be an integer from 1 to 65535.")
fi
if [[ -n "${GAME_WORLD_SLUG:-}" ]]; then
    [[ "$GAME_WORLD_SLUG" =~ ^[a-z0-9][a-z0-9-]{0,63}$ ]] \
        || reasons+=("GAME_WORLD_SLUG must be 1 to 64 lower-case letters, digits and hyphens, not starting with a hyphen.")
fi
if [[ -n "${GAME_WORLD_NAME:-}" ]]; then
    (( ${#GAME_WORLD_NAME} <= 100 )) \
        || reasons+=("GAME_WORLD_NAME must be at most 100 characters.")
fi
if [[ -n "${GAME_WORLD_REGION:-}" ]]; then
    [[ "$GAME_WORLD_REGION" =~ ^[A-Za-z0-9_-]{1,32}$ ]] \
        || reasons+=("GAME_WORLD_REGION must be a simple 1 to 32 character label of letters, digits, underscores and hyphens.")
fi
if [[ -n "${GAME_WORLD_HOST:-}" && -n "$probe_result" && "$(value_of host_valid "$probe_output")" != true ]]; then
    reasons+=("GAME_WORLD_HOST must be an IPv4/IPv6 address or DNS hostname without a scheme or path (the same check as game-auth:world:ensure).")
fi
case "${OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED:-false}" in
    true)
        [[ "${OTERYN_NATIVE_CHANNEL_KEY:-}" =~ ^[a-z0-9][a-z0-9-]{0,63}$ ]] \
            || reasons+=("native route publication is on but OTERYN_NATIVE_CHANNEL_KEY is not a valid channel key.")
        ;;
    false | '') ;;
    *) reasons+=("OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED must be true or false.") ;;
esac
if [[ "$local_mode" != true ]]; then
    [[ -n "${DB_MIGRATIONS_USERNAME:-}" && -n "${DB_MIGRATIONS_PASSWORD:-}" ]] \
        || reasons+=("DB_MIGRATIONS_USERNAME and DB_MIGRATIONS_PASSWORD (the migration owner role) are not in the environment file.")
fi

# The state records written after the first change must be writable now: a stale temporary path or an
# unwritable state directory would otherwise fail the write after the move-aside.
for record_tmp in "$moved_file.tmp" "$complete_file.tmp"; do
    if [[ -e "$record_tmp" || -L "$record_tmp" ]]; then
        reasons+=("$record_tmp already exists; remove the stale temporary record before the bootstrap.")
    fi
done
[[ -w "$state_dir" && -x "$state_dir" ]] || reasons+=("the bootstrap state directory $state_dir is not writable.")

if ((${#reasons[@]} > 0)); then
    for reason in "${reasons[@]}"; do
        printf 'Refused: %s\n' "$reason" >&2
    done
    echo "Nothing was changed." >&2
    exit 1
fi
}

run_refusal_checks
# All refusal checks passed: take the lock, then repeat every check under it (witness, services, database
# probe, records), and only then restrict the state directory mode.
prepare_state_dir
run_refusal_checks
restrict_state_dir
[[ ! -e "$complete_file" ]] || refuse "this state directory already records a completed bootstrap ($complete_file); a re-run is refused."
if [[ -n "$adopt_aside_to" ]]; then
    # Re-establish the move record of the earlier attempt (its write had failed); nothing else changes.
    printf 'MOVED_ASIDE_TO=%s\nWITNESS_STORE_ID=%s\nMOVED_AT=%s\n' "$adopt_aside_to" "$frozen_store_id" \
        "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$moved_file.tmp" \
        && chmod 600 "$moved_file.tmp" \
        && mv -- "$moved_file.tmp" "$moved_file" \
        || { rm -f -- "$moved_file.tmp"; refuse "the moved-aside record $moved_file for the preserved directory $adopt_aside_to could not be written; nothing else was changed."; }
    echo "Adopted the intact preserved MariaDB-era witness directory: $adopt_aside_to"
fi
[[ "$witness_action" != move-aside || ! -e "$moved_file" ]] \
    || refuse "this bootstrap already moved the MariaDB-era witness aside ($moved_file) in a concurrent run."

# --- run: steps 1 to 8 ---------------------------------------------------------------------
echo "Step 1/8: native evidence witness reset."
if [[ "$witness_action" == move-aside ]]; then
    target="$frozen_dir.mariadb-era-$(date -u +%Y%m%dT%H%M%SZ)"
    witness_output="$(witness move-aside "$frozen_manifest" "$frozen_store_id" "$target")" || true
    moved_to="$(value_of witness_moved_aside_to "$witness_output")"
    if [[ -n "$moved_to" ]]; then
        printf 'MOVED_ASIDE_TO=%s\nWITNESS_STORE_ID=%s\nMOVED_AT=%s\n' "$moved_to" "$frozen_store_id" \
            "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$moved_file.tmp" \
            && chmod 600 "$moved_file.tmp" \
            && mv -- "$moved_file.tmp" "$moved_file" \
            || { rm -f -- "$moved_file.tmp"; failed "the MariaDB-era witness was moved aside to $moved_to but its record $moved_file could not be written; a re-run adopts the intact preserved directory."; }
        echo "MariaDB-era witness directory moved aside, unedited: $moved_to"
    fi
else
    witness_output="$(witness prepare-empty)" || true
fi
if [[ "$(value_of witness_new_directory "$witness_output")" != empty ]]; then
    error="$(value_of witness_error "$witness_output")"
    if [[ -z "$moved_to" && "$witness_action" == move-aside ]]; then
        refuse "the witness directory could not be moved aside (${error:-no result}); nothing was changed."
    fi
    failed "the new witness directory is not an empty owner-only directory after the move (${error:-no result})."
fi
echo "New witness directory is empty: $frozen_dir"

echo "Step 2/8: schema (php artisan migrate on pgsql_migrations)."
platform_oneoff artisan migrate --force --no-interaction --database=pgsql_migrations \
    || failed "migrations did not complete on the empty database."

container_id=''
if [[ "$local_mode" != true ]]; then
    # Ingress was checked stopped above; only platform starts here.
    "${compose[@]}" up -d --no-deps platform || failed "the platform service did not start."
    container_id="$("${compose[@]}" ps -q platform)" || failed "the platform container cannot be read."
    [[ -n "$container_id" ]] || failed "the platform container is not running."
fi

echo "Step 3/8: Passport keys."
if [[ "$local_mode" == true ]]; then
    keys_dir="${LARAVEL_STORAGE_PATH:-$PLATFORM_ROOT/storage}"
    if [[ -s "$keys_dir/oauth-private.key" && -s "$keys_dir/oauth-public.key" ]]; then
        echo "Existing Passport keys kept."
    else
        platform_exec artisan passport:keys --no-interaction || failed "passport:keys failed."
    fi
elif "${compose[@]}" exec -T platform sh -ec 'test -s storage/oauth-private.key && test -s storage/oauth-public.key'; then
    echo "Existing Passport keys kept."
else
    "${compose[@]}" exec -T platform php artisan passport:keys --no-interaction || failed "passport:keys failed."
fi

run_self_check() {
    local output
    if [[ "$local_mode" == true ]]; then
        output="$(platform_exec artisan game-auth:native-admission:self-check --no-interaction 2>&1)" \
            || failed "native admission self-check refused: $output"
        echo "Native admission signing self-check passed: $output"
    else
        oteryn_run_native_admission_self_check "$container_id" || failed "native admission self-check refused."
    fi
}

echo "Step 4/8: native admission trust."
if [[ "$native_admission" == true ]]; then
    echo "Native admission is on: this trust publication is the first witnessed write. From here on there is no rollback and no re-run."
    witnessed_write_started=true
    if [[ "$local_mode" == true ]]; then
        trust_output="$(platform_exec artisan game-auth:native-admission:publish-trust --no-interaction 2>&1)" \
            || failed "native admission trust publication refused: $trust_output"
        [[ "$trust_output" =~ \"result\":\"(published|already_trusted)\" ]] \
            || failed "native admission trust publication returned no trust result."
        echo "Native admission signing key trusted (PUBLIC values): $trust_output"
    else
        oteryn_publish_native_admission_trust "$container_id" || failed "native admission trust publication refused."
    fi
    run_self_check
else
    echo "Native admission is off; trust publication is deferred (plan P6 step 4). No witnessed write happens in this run."
fi

echo "Step 5/8: OAuth clients."
oauth_output="$(platform_exec artisan game-auth:oauth-client:ensure --no-interaction 2>&1)" \
    || failed "game-auth:oauth-client:ensure failed: $oauth_output"
oauth_client_id="$(sed -n 's/.*OAuth client id: \([A-Za-z0-9-]*\).*/\1/p' <<<"$oauth_output" | tail -n 1)"
oauth_rust_output="$(platform_exec artisan game-auth:oauth-client:ensure --rust --no-interaction 2>&1)" \
    || failed "game-auth:oauth-client:ensure --rust failed: $oauth_rust_output"
oauth_rust_client_id="$(sed -n 's/.*OAuth client id: \([A-Za-z0-9-]*\).*/\1/p' <<<"$oauth_rust_output" | tail -n 1)"
[[ -n "$oauth_client_id" && -n "$oauth_rust_client_id" ]] || failed "the OAuth client ensure commands printed no client id."

echo "Step 6/8: World Registry row and native topology."
platform_exec artisan game-auth:world:ensure \
    --id="$GAME_WORLD_ID" --slug="$GAME_WORLD_SLUG" --name="$GAME_WORLD_NAME" \
    --region="$GAME_WORLD_REGION" --host="$GAME_WORLD_HOST" --port="$GAME_WORLD_PORT" \
    --status=online --login-enabled=1 --no-interaction || failed "game-auth:world:ensure failed."
native_world_id=not-issued
if [[ "${OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED:-false}" == true ]]; then
    topology_output="$(platform_exec artisan game-auth:native-topology:issue \
        --world-row-id="$GAME_WORLD_ID" --channel-key="$OTERYN_NATIVE_CHANNEL_KEY" --no-interaction 2>&1)" \
        || failed "game-auth:native-topology:issue failed: $topology_output"
    native_world_id="$(sed -n 's/.*"world_id":"\([^"]*\)".*/\1/p' <<<"$topology_output" | head -n 1)" || true
    [[ -n "$native_world_id" ]] || failed "native topology issuance returned no WorldId."
    echo "Native topology issued (non-secret): $topology_output"
else
    echo "Native route publication is off; no WorldId is issued and no route is published."
fi

echo "Step 7/8: administration and content are operator follow-ups (runbook): register, confirm MFA, then admin:bootstrap <email>."

echo "Step 8/8: checks."
if [[ "$app_env" == production ]]; then
    platform_exec artisan production:verify-configuration --no-interaction \
        || failed "production:verify-configuration failed."
else
    echo "production:verify-configuration not run: it requires APP_ENV=production and this Platform runs APP_ENV=${app_env:-unknown}."
fi
if [[ "$native_admission" == true ]]; then
    run_self_check
fi
verify_output="$(probe verify "$GAME_WORLD_ID")"
[[ "$(value_of result "$verify_output")" == ok ]] || failed "the bootstrap verification probe did not complete."
[[ "$(value_of pending_migrations "$verify_output")" == 0 ]] || failed "migrations are pending after the bootstrap."
[[ "$(value_of admin_roles "$verify_output")" =~ ^[1-9][0-9]*$ \
    && "$(value_of admin_role_permissions "$verify_output")" =~ ^[1-9][0-9]*$ ]] \
    || failed "the built-in admin roles and permissions were not seeded."
[[ "$(value_of oauth_clients "$verify_output")" =~ ^([2-9]|[1-9][0-9]+)$ ]] || failed "the OAuth clients are missing."
[[ "$(value_of world_row "$verify_output")" == present ]] || failed "the World Registry row is missing."
# The rollback witness this run depends on (moved aside now or by an earlier attempt) must still be intact.
if [[ -n "$moved_to" ]]; then
    aside_final="$(witness verify-aside "$moved_to" "$frozen_manifest" "$frozen_store_id")" || true
    [[ "$(value_of witness_aside "$aside_final")" == verified ]] \
        || failed "the preserved MariaDB-era witness $moved_to is no longer intact ($(value_of witness_error "$aside_final"))."
fi
# The witness directory must be inspectable now, whatever the mode: a failed inspection or a vanished
# directory is never read as success.
final_witness="$(witness inspect)" || failed "the final native evidence witness inspection failed."
[[ -z "$(value_of witness_error "$final_witness")" \
    && "$(value_of witness_directory "$final_witness")" == "$frozen_dir" \
    && "$(value_of witness_state "$final_witness")" =~ ^(empty|present)$ ]] \
    || failed "the final native evidence witness inspection returned no usable state ($(value_of witness_error "$final_witness"))."
if [[ "$native_admission" == true ]]; then
    # The trust publication wrote a PostgreSQL witness: a nonempty directory with its own valid store id,
    # not the MariaDB-era one that was moved aside.
    final_store_id="$(value_of witness_store_id "$final_witness")"
    [[ "$(value_of witness_state "$final_witness")" == present && "$final_store_id" =~ ^[0-9a-f]{64}$ ]] \
        || failed "native admission is on but the new witness directory holds no witness store after the trust publication."
    [[ "$final_store_id" != "$frozen_store_id" ]] \
        || failed "native admission is on but the witness store is still the MariaDB-era one."
    [[ "$(value_of trust_profiles "$verify_output")" =~ ^[1-9][0-9]*$ \
        && "$(value_of witness_binding_rows "$verify_output")" == 1 ]] \
        || failed "native admission is on but the trust profile or the witness binding is missing."
else
    [[ "$(value_of trust_profiles "$verify_output")" == 0 \
        && "$(value_of witness_binding_rows "$verify_output")" == 0 \
        && "$(value_of witness_state "$final_witness")" == empty ]] \
        || failed "native admission is off but a witnessed write happened (trust row, witness binding or witness file)."
fi
echo "Bootstrap checks passed. The deploy that follows runs health-check.sh with the full stack (runbook)."

printf 'BOOTSTRAPPED_AT=%s\nOAUTH_CLIENT_ID=%s\nOAUTH_RUST_CLIENT_ID=%s\nNATIVE_WORLD_ID=%s\nNATIVE_ADMISSION=%s\n' \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$oauth_client_id" "$oauth_rust_client_id" "$native_world_id" "$native_admission" \
    > "$complete_file.tmp" \
    && chmod 600 "$complete_file.tmp" \
    && mv -- "$complete_file.tmp" "$complete_file" \
    || { rm -f -- "$complete_file.tmp"; failed "the completion record $complete_file could not be written."; }

echo "Non-secret values for the operator:"
echo "bootstrap_result=complete"
echo "oauth_client_id=$oauth_client_id"
echo "oauth_rust_client_id=$oauth_rust_client_id"
echo "native_world_id=$native_world_id"
echo "witness_moved_aside_to=${moved_to:-none}"
