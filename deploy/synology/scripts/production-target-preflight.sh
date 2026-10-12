#!/usr/bin/env bash
set -euo pipefail

project_name="${OTERYN_COMPOSE_PROJECT_NAME:-oteryn-staging}"
validation_sha="${VALIDATION_SHA:-UNKNOWN}"
evidence_path="${OTERYN_PREFLIGHT_EVIDENCE_PATH:-/tmp/synology-production-target-preflight-evidence.json}"
state_dir="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"
run_restore_drill="${OTERYN_RUN_RESTORE_DRILL:-true}"
platform_db="${OTERYN_PLATFORM_DB_NAME:-oteryn_platform}"
work_dir="${OTERYN_PREFLIGHT_WORK_DIR:-/tmp/oteryn-synology-production-target-preflight}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

mariadb_root_password="${MARIADB_ROOT_PASSWORD:-}"

declare -A containers=()

fail() {
    echo "synology-target-preflight failure: $*" >&2
    exit 1
}

cleanup() {
    status=$?
    rm -rf "$work_dir"
    exit "$status"
}
trap cleanup EXIT

for command in docker python3 stat date grep tail; do
    command -v "$command" >/dev/null 2>&1 || fail "required command is missing: $command"
done

platform_repo="$(bash "$script_dir/repository-ghcr-image.sh" oteryn-platform)" \
    || fail "unable to resolve current repository Platform GHCR coordinate"
gateway_repo="$(bash "$script_dir/repository-ghcr-image.sh" oteryn-game-gateway)" \
    || fail "unable to resolve current repository Gateway GHCR coordinate"

[[ "$project_name" =~ ^[A-Za-z0-9][A-Za-z0-9_.-]{0,63}$ ]] \
    || fail "OTERYN_COMPOSE_PROJECT_NAME is not a bounded Compose project name"
[[ "$platform_db" =~ ^[A-Za-z0-9_]{1,64}$ ]] || fail "OTERYN_PLATFORM_DB_NAME is invalid"
[[ "$run_restore_drill" == "true" || "$run_restore_drill" == "false" ]] \
    || fail "OTERYN_RUN_RESTORE_DRILL must be true or false"
[[ -n "$mariadb_root_password" ]] || fail "MARIADB_ROOT_PASSWORD is required"

# The Platform database engine of the checked-out release contract (ADR 0046, plan P4): mariadb or
# postgres. On postgres the Platform database is the postgres service; MariaDB keeps running until the
# cutover retires it, so it is still checked. The PostgreSQL checks need no secret: they run as the local
# superuser over the socket (peer), and the Platform probe uses the Platform container's own settings.
platform_db_engine="$(sed -n 's/^PLATFORM_DB_ENGINE=//p' "$script_dir/../release-contract.env")"
[[ "$platform_db_engine" == mariadb || "$platform_db_engine" == postgres ]] \
    || fail "release-contract.env PLATFORM_DB_ENGINE must be mariadb or postgres"
[[ "$platform_db_engine" == mariadb || "$platform_db" == oteryn_platform ]] \
    || fail "the PostgreSQL Platform database is oteryn_platform"
services=(mariadb platform internal-proxy gateway marketplace-scheduler)
healthy_services=(mariadb marketplace-scheduler)
unpublished_services=(mariadb marketplace-scheduler internal-proxy)
if [[ "$platform_db_engine" == postgres ]]; then
    services+=(postgres)
    healthy_services+=(postgres)
    unpublished_services+=(postgres)
fi

rm -rf "$work_dir"
mkdir -p "$work_dir" "$(dirname "$evidence_path")"
chmod 700 "$work_dir"

find_single_container() {
    local service="$1"
    local ids=()
    local id

    while IFS= read -r id; do
        [[ -n "$id" ]] && ids+=("$id")
    done < <(
        docker ps -aq \
            --filter "label=com.docker.compose.project=$project_name" \
            --filter "label=com.docker.compose.service=$service"
    )

    (( ${#ids[@]} == 1 )) \
        || fail "expected exactly one $service container for Compose project $project_name, found ${#ids[@]}"
    containers["$service"]="${ids[0]}"
}

for service in "${services[@]}"; do
    find_single_container "$service"
done

for service in "${services[@]}"; do
    id="${containers[$service]}"
    [[ "$(docker inspect --format '{{.State.Running}}' "$id")" == "true" ]] \
        || fail "$service is not running"
done

for service in "${services[@]}"; do
    id="${containers[$service]}"
    [[ "$(docker inspect --format '{{.HostConfig.RestartPolicy.Name}}' "$id")" == "always" ]] \
        || fail "$service does not use restart policy always"
done

for service in "${healthy_services[@]}"; do
    id="${containers[$service]}"
    health="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}' "$id")"
    [[ "$health" == "healthy" ]] || fail "$service health status is $health"
done

expected_network="${project_name}_private"
[[ "$(docker network inspect --format '{{.Driver}}' "$expected_network")" == "bridge" ]] \
    || fail "$expected_network is not a bridge network"
for service in "${services[@]}"; do
    docker inspect --format '{{range $name, $_ := .NetworkSettings.Networks}}{{println $name}}{{end}}' \
        "${containers[$service]}" \
        | grep -Fxq "$expected_network" \
        || fail "$service is not connected to $expected_network"
done

published_binding() {
    local service="$1"
    local container_port="$2"
    docker inspect --format \
        "{{with index .NetworkSettings.Ports \"${container_port}/tcp\"}}{{if eq (len .) 1}}{{(index . 0).HostIp}}:{{(index . 0).HostPort}}{{end}}{{end}}" \
        "${containers[$service]}"
}

[[ "$(published_binding platform 8000)" == "127.0.0.1:8000" ]] \
    || fail "Platform is not bound exactly to 127.0.0.1:8000"
[[ "$(published_binding gateway 8080)" == "127.0.0.1:8080" ]] \
    || fail "Gateway is not bound exactly to 127.0.0.1:8080"
assert_no_published_ports() {
    local service="$1"
    local published
    published="$(docker inspect --format '{{range $port, $bindings := .NetworkSettings.Ports}}{{if $bindings}}{{println $port}}{{end}}{{end}}' "${containers[$service]}")"
    [[ -z "$published" ]] || fail "$service unexpectedly publishes a host port"
}

for service in "${unpublished_services[@]}"; do
    assert_no_published_ports "$service"
done

python3 - \
    "$(docker inspect --format '{{json .NetworkSettings.Ports}}' "${containers[platform]}")" \
    "$(docker inspect --format '{{json .NetworkSettings.Ports}}' "${containers[gateway]}")" <<'PY'
import json
import sys

expected_sets = [
    {'8000/tcp'},
    {'8080/tcp'},
]

for raw, expected in zip(sys.argv[1:], expected_sets, strict=True):
    ports = json.loads(raw)
    published = {key: value for key, value in ports.items() if value}
    if set(published) != expected:
        raise SystemExit('unexpected published port set')
    for bindings in published.values():
        if len(bindings) != 1:
            raise SystemExit('published port has multiple host bindings')
        if bindings[0].get('HostIp') in {'0.0.0.0', '::', ''}:
            raise SystemExit('wildcard host binding detected')
PY

platform_image="$(docker inspect --format '{{.Config.Image}}' "${containers[platform]}")"
gateway_image="$(docker inspect --format '{{.Config.Image}}' "${containers[gateway]}")"

platform_prefix="${platform_repo}@sha256:"
[[ "$platform_image" == "$platform_prefix"* ]] \
    || fail "Platform is not deployed by immutable digest from the current repository owner"
platform_digest_hex="${platform_image#"$platform_prefix"}"
[[ "$platform_digest_hex" =~ ^[a-f0-9]{64}$ ]] \
    || fail "Platform immutable image digest is malformed"
platform_digest="sha256:${platform_digest_hex}"

gateway_prefix="${gateway_repo}@sha256:"
[[ "$gateway_image" == "$gateway_prefix"* ]] \
    || fail "Gateway is not deployed by immutable digest from the current repository owner"
gateway_digest_hex="${gateway_image#"$gateway_prefix"}"
[[ "$gateway_digest_hex" =~ ^[a-f0-9]{64}$ ]] \
    || fail "Gateway immutable image digest is malformed"
gateway_digest="sha256:${gateway_digest_hex}"

current_release_file="$state_dir/current-release.env"
[[ -f "$current_release_file" ]] || fail "managed current release state is missing"
bash "$script_dir/release-state.sh" validate "$current_release_file" \
    || fail "managed current release state is invalid"
mapfile -t release_state < <(bash -c '
    set -euo pipefail
    source "$1"
    printf "%s\n" \
        "$RELEASE_SHA" "$PLATFORM_SOURCE_SHA" "$GATEWAY_SOURCE_SHA" \
        "$PLATFORM_IMAGE" "$GATEWAY_IMAGE"
' bash "$current_release_file")

deployed_release_sha="${release_state[0]:-}"
platform_source_sha="${release_state[1]:-}"
gateway_source_sha="${release_state[2]:-}"
persisted_platform_image="${release_state[3]:-}"
persisted_gateway_image="${release_state[4]:-}"
[[ "$deployed_release_sha" =~ ^[a-f0-9]{40}$ ]] || fail "persisted overall release SHA is invalid"
[[ "$platform_source_sha" =~ ^[a-f0-9]{40}$ ]] || fail "persisted Platform source SHA is invalid"
[[ "$gateway_source_sha" =~ ^[a-f0-9]{40}$ ]] || fail "persisted Gateway source SHA is invalid"
[[ "$platform_image" == "$persisted_platform_image" ]] || fail "running Platform digest differs from current release state"
[[ "$gateway_image" == "$persisted_gateway_image" ]] || fail "running Gateway digest differs from current release state"
scheduler_image="$(docker inspect --format '{{.Config.Image}}' "${containers[marketplace-scheduler]}")"
[[ "$scheduler_image" == "$persisted_platform_image" ]] || fail "running scheduler image differs from current Platform release state"

for service in platform gateway marketplace-scheduler; do
    image_id="$(docker inspect --format '{{.Image}}' "${containers[$service]}")"
    [[ "$image_id" =~ ^sha256:[a-f0-9]{64}$ ]] || fail "$service image ID is not immutable"
done

platform_revision="$(docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "$(docker inspect --format '{{.Image}}' "${containers[platform]}")")"
gateway_revision="$(docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "$(docker inspect --format '{{.Image}}' "${containers[gateway]}")")"
scheduler_revision="$(docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "$(docker inspect --format '{{.Image}}' "${containers[marketplace-scheduler]}")")"
[[ "$scheduler_revision" == "$platform_source_sha" ]] \
    || fail "scheduler OCI revision does not match persisted Platform source SHA"
[[ "$platform_revision" == "$platform_source_sha" ]] \
    || fail "Platform OCI revision does not match persisted Platform source SHA"
[[ "$gateway_revision" == "$gateway_source_sha" ]] \
    || fail "Gateway OCI revision does not match persisted Gateway source SHA"

assert_named_volume() {
    local service="$1"
    local destination="$2"
    local mount
    local type
    local name

    mount="$(docker inspect --format \
        "{{range .Mounts}}{{if eq .Destination \"$destination\"}}{{.Type}}|{{.Name}}{{end}}{{end}}" \
        "${containers[$service]}")"
    IFS='|' read -r type name <<<"$mount"
    [[ "$type" == "volume" && -n "$name" ]] \
        || fail "$service destination $destination is not backed by a named volume"
    [[ "$(docker volume inspect --format '{{.Driver}}' "$name")" == "local" ]] \
        || fail "$service volume for $destination does not use the local volume driver"
}

assert_named_volume mariadb /var/lib/mysql
assert_named_volume platform /var/www/html/storage
assert_named_volume internal-proxy /etc/nginx/tls
[[ "$platform_db_engine" != postgres ]] || assert_named_volume postgres /var/lib/postgresql/data

[[ -d "$state_dir" ]] || fail "persistent runner state directory is missing"
[[ "$(stat -c '%a' "$state_dir")" == "700" ]] \
    || fail "persistent runner state directory must have mode 700"
state_file="$state_dir/last-good.env"
[[ -f "$state_file" ]] || fail "last-good runtime image snapshot is missing"
[[ "$(stat -c '%a' "$state_file")" == "600" ]] \
    || fail "last-good runtime image snapshot must have mode 600"
for marker in PLATFORM_IMAGE GATEWAY_IMAGE; do
    grep -Eq "^${marker}=" "$state_file" || fail "last-good snapshot is missing $marker"
done

docker exec "${containers[platform]}" sh -ec '
    # Synology runs as preproduction; a stack not yet redeployed since the cut-over still reports staging.
    test "$APP_ENV" = preproduction || test "$APP_ENV" = staging
    test "$APP_DEBUG" = false
    test "$LOG_CHANNEL" = stderr_json
    test "$SESSION_DRIVER" = file
    test "$SESSION_HTTP_ONLY" = true
    test "$CACHE_STORE" = file
    test "$QUEUE_CONNECTION" = sync
    test "$MAIL_MAILER" = array
'

postgres_tls_and_roles="NOT_APPLICABLE"
if [[ "$platform_db_engine" == postgres ]]; then
    postgres_sql() {
        docker exec -u postgres "${containers[postgres]}" \
            psql -X -q -t -A -v ON_ERROR_STOP=1 -d "$platform_db" -c "$1"
    }

    [[ "$(postgres_sql 'SHOW ssl')" == on ]] || fail "PostgreSQL ssl is not on"
    [[ "$(postgres_sql 'SHOW ssl_min_protocol_version')" == TLSv1.3 ]] \
        || fail "PostgreSQL does not require TLS 1.3"
    [[ "$(postgres_sql 'SHOW password_encryption')" == scram-sha-256 ]] \
        || fail "PostgreSQL password_encryption is not scram-sha-256"
    # Every pg_hba rule parses and is a reject, a TLS-only SCRAM login or the superuser's local peer login.
    [[ "$(postgres_sql "SELECT count(*) FROM pg_hba_file_rules WHERE error IS NOT NULL
        OR NOT (auth_method = 'reject'
            OR (type = 'hostssl' AND auth_method = 'scram-sha-256')
            OR (type = 'local' AND auth_method = 'peer' AND user_name = ARRAY['postgres']))")" == 0 ]] \
        || fail "PostgreSQL pg_hba.conf allows a login without TLS and SCRAM"
    [[ "$(postgres_sql "SELECT count(*) FROM pg_hba_file_rules WHERE type = 'hostssl'")" =~ ^[1-9][0-9]*$ ]] \
        || fail "PostgreSQL pg_hba.conf has no TLS login rule"
    # The role split (plan P4): the owner owns the database and schema public, the runtime, dump and
    # backup roles own nothing, no Platform role is elevated and only the backup role replicates.
    platform_roles="'oteryn_platform_owner','oteryn_platform_app','oteryn_platform_backup','oteryn_platform_dump'"
    [[ "$(postgres_sql "SELECT count(*) FROM pg_roles WHERE rolname IN ($platform_roles) AND rolcanlogin")" == 4 ]] \
        || fail "a Platform PostgreSQL role is missing"
    [[ "$(postgres_sql "
        SELECT (SELECT count(*) FROM pg_class c JOIN pg_roles r ON r.oid = c.relowner
                 WHERE r.rolname IN ('oteryn_platform_app', 'oteryn_platform_dump', 'oteryn_platform_backup'))
             + (SELECT count(*) FROM pg_roles WHERE rolname IN ($platform_roles)
                 AND (rolsuper OR rolcreatedb OR rolcreaterole OR rolbypassrls))
             + (SELECT count(*) FROM pg_roles WHERE rolname IN ($platform_roles)
                 AND rolreplication <> (rolname = 'oteryn_platform_backup'))
             + (SELECT count(*) FROM pg_namespace n JOIN pg_roles r ON r.oid = n.nspowner
                 WHERE n.nspname = 'public' AND r.rolname <> 'oteryn_platform_owner')
             + (SELECT count(*) FROM pg_database d JOIN pg_roles r ON r.oid = d.datdba
                 WHERE d.datname = '$platform_db' AND r.rolname <> 'oteryn_platform_owner')")" == 0 ]] \
        || fail "the PostgreSQL role split does not hold (ownership or role attributes)"
    [[ "$(postgres_sql "SELECT count(*) FROM pg_stat_activity a JOIN pg_stat_ssl s ON s.pid = a.pid
        WHERE a.backend_type = 'client backend' AND a.client_addr IS NOT NULL AND NOT s.ssl")" == 0 ]] \
        || fail "a PostgreSQL client session over TCP is not encrypted"

    # The Platform runs on the app role over verify-full TLS and never holds the owner or any PostgreSQL
    # secret; the names are checked, never the values.
    docker exec "${containers[platform]}" sh -ec '
        test "$DB_CONNECTION" = pgsql
        test "$DB_HOST" = postgres
        test "$DB_USERNAME" = oteryn_platform_app
        test "$DB_SSLMODE" = verify-full
        test -s "$DB_SSLROOTCERT"
        test -z "${DB_MIGRATIONS_PASSWORD+set}"
        test -z "${POSTGRES_PASSWORD+set}"
        if env | grep -q "^PLATFORM_POSTGRES_"; then exit 1; fi
    ' || fail "Platform does not run on the PostgreSQL app role with verify-full TLS only"
    probe_status=0
    docker exec "${containers[platform]}" php -r '
        $dsn = sprintf("pgsql:host=%s;port=%s;dbname=%s;sslmode=%s;sslrootcert=%s",
            getenv("DB_HOST"), getenv("DB_PORT"), getenv("DB_DATABASE"), getenv("DB_SSLMODE"), getenv("DB_SSLROOTCERT"));
        try {
            $pdo = new PDO($dsn, getenv("DB_USERNAME"), getenv("DB_PASSWORD"), [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION]);
        } catch (Throwable $e) {
            exit(3);
        }
        $row = $pdo->query("SELECT current_user AS role, s.ssl::int AS ssl, s.version FROM pg_stat_ssl s WHERE s.pid = pg_backend_pid()")->fetch(PDO::FETCH_ASSOC);
        if ($row["role"] !== "oteryn_platform_app" || (int) $row["ssl"] !== 1 || $row["version"] !== "TLSv1.3") {
            exit(4);
        }
        $pdo->beginTransaction();
        try {
            $pdo->exec("CREATE TABLE oteryn_preflight_privilege_probe (id integer)");
            $pdo->rollBack();
            exit(5);
        } catch (PDOException $e) {
            $pdo->rollBack();
            exit($e->getCode() === "42501" ? 0 : 6);
        }
    ' || probe_status=$?
    case "$probe_status" in
        0) ;;
        3) fail "Platform cannot log in to PostgreSQL as the app role" ;;
        4) fail "the Platform PostgreSQL session is not the app role over TLS 1.3" ;;
        5) fail "the Platform PostgreSQL app role can create tables" ;;
        *) fail "the Platform PostgreSQL privilege probe failed ($probe_status)" ;;
    esac
    postgres_tls_and_roles="PASS"
fi

probe_http() {
    local service="$1"
    local port="$2"
    local path="$3"
    local label="$4"

    docker run --rm \
        --network "container:${containers[$service]}" \
        alpine:3.22 \
        /bin/sh -ec "wget -qO- -T 5 'http://127.0.0.1:${port}${path}' >/dev/null" \
        || fail "$label failed"
}

probe_http platform 8000 /health "Platform health probe"
probe_http gateway 8080 /health "Gateway health probe"
probe_http gateway 8080 /ready "Gateway readiness probe"
probe_http gateway 8080 /version "Gateway version probe"

restore_result="NOT_RUN"
restore_duration_ms=0
platform_table_count=0

if [[ "$run_restore_drill" == "true" && "$platform_db_engine" == postgres ]]; then
    # PostgreSQL drill: a plain dump of the Platform database streams into a scratch database in one
    # transaction; both dumps must hash equal (pg_dump's per-run \restrict key lines excluded) and hold
    # the same base tables. The local superuser runs it over the socket (peer); nothing touches the disk.
    export OTERYN_PREFLIGHT_POSTGRES_CONTAINER="${containers[postgres]}"
    export OTERYN_PREFLIGHT_PLATFORM_DB="$platform_db"
    export OTERYN_PREFLIGHT_RUN_SUFFIX="${GITHUB_RUN_ID:-$(date +%s)}_${GITHUB_RUN_ATTEMPT:-1}"
    export OTERYN_PREFLIGHT_RESTORE_SUMMARY="$work_dir/restore-summary.json"

    python3 <<'PY'
import hashlib
import json
import os
import re
import subprocess
import time

container = os.environ['OTERYN_PREFLIGHT_POSTGRES_CONTAINER']
platform_db = os.environ['OTERYN_PREFLIGHT_PLATFORM_DB']
summary_path = os.environ['OTERYN_PREFLIGHT_RESTORE_SUMMARY']
suffix = re.sub(r'[^0-9_]', '_', os.environ['OTERYN_PREFLIGHT_RUN_SUFFIX'])
target = f'oteryn_preflight_platform_{suffix}'

for name in [container, platform_db]:
    if not re.fullmatch(r'[A-Za-z0-9_.-]{1,128}', name):
        raise SystemExit('invalid bounded restore-drill identifier')
if not re.fullmatch(r'[a-z0-9_]{1,63}', target):
    raise SystemExit('invalid bounded restore-drill database name')

PSQL = ['docker', 'exec', '-i', '-u', 'postgres', container, 'psql', '-X', '-q', '-t', '-A', '-v', 'ON_ERROR_STOP=1']


def psql(database, sql):
    return subprocess.run(
        [*PSQL, '-d', database, '-c', sql],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=True,
    ).stdout


def drop_restore_database():
    try:
        psql('postgres', f'DROP DATABASE IF EXISTS "{target}" WITH (FORCE)')
    except subprocess.CalledProcessError:
        pass


def dump_command(database):
    return [
        'docker', 'exec', '-u', 'postgres', container,
        'pg_dump', '--format=plain', '--no-owner', '--no-privileges', '--dbname', database,
    ]


def digest_line(digest, line):
    # pg_dump wraps a plain dump in \restrict / \unrestrict lines with a random per-run key.
    if not line.startswith((b'\\restrict ', b'\\unrestrict ')):
        digest.update(line)


def stream_dump_into_restore(source):
    source_process = subprocess.Popen(dump_command(source), stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    restore_process = subprocess.Popen(
        [*PSQL, '--single-transaction', '-d', target],
        stdin=subprocess.PIPE,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    digest = hashlib.sha256()
    assert source_process.stdout is not None
    assert restore_process.stdin is not None
    for line in source_process.stdout:
        digest_line(digest, line)
        restore_process.stdin.write(line)
    restore_process.stdin.close()
    source_rc = source_process.wait()
    restore_rc = restore_process.wait()
    if source_rc != 0 or restore_rc != 0:
        raise SystemExit('streaming database restore failed')
    return digest.hexdigest()


def dump_digest(database):
    process = subprocess.Popen(dump_command(database), stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    digest = hashlib.sha256()
    assert process.stdout is not None
    for line in process.stdout:
        digest_line(digest, line)
    if process.wait() != 0:
        raise SystemExit('restored database verification dump failed')
    return digest.hexdigest()


def table_count(database):
    raw = psql(database, "SELECT count(*) FROM pg_tables WHERE schemaname NOT IN ('pg_catalog', 'information_schema')")
    return int(raw.decode().strip())


started = time.monotonic_ns()
drop_restore_database()
try:
    psql('postgres', f"CREATE DATABASE \"{target}\" TEMPLATE template0 ENCODING 'UTF8' LOCALE_PROVIDER libc LOCALE 'C.UTF-8'")
    # Same schema public owner as the source (asserted above), so both dumps carry the same header.
    psql(target, 'ALTER SCHEMA public OWNER TO oteryn_platform_owner')
    source_digest = stream_dump_into_restore(platform_db)
    if source_digest != dump_digest(target):
        raise SystemExit('restored database content digest differs from streamed source dump')
    source_count = table_count(platform_db)
    if source_count != table_count(target):
        raise SystemExit('restored database base-table count differs')
finally:
    drop_restore_database()

elapsed_ms = (time.monotonic_ns() - started) // 1_000_000
with open(summary_path, 'w', encoding='utf-8') as handle:
    json.dump(
        {'result': 'PASS', 'duration_ms': elapsed_ms, 'platform_table_count': source_count},
        handle,
        separators=(',', ':'),
    )
PY
elif [[ "$run_restore_drill" == "true" ]]; then
    export OTERYN_PREFLIGHT_MARIADB_CONTAINER="${containers[mariadb]}"
    export OTERYN_PREFLIGHT_MARIADB_ROOT_PASSWORD="$mariadb_root_password"
    export OTERYN_PREFLIGHT_PLATFORM_DB="$platform_db"
    export OTERYN_PREFLIGHT_RUN_SUFFIX="${GITHUB_RUN_ID:-$(date +%s)}_${GITHUB_RUN_ATTEMPT:-1}"
    export OTERYN_PREFLIGHT_RESTORE_SUMMARY="$work_dir/restore-summary.json"

    python3 <<'PY'
import hashlib
import json
import os
import re
import subprocess
import time

container = os.environ['OTERYN_PREFLIGHT_MARIADB_CONTAINER']
password = os.environ['OTERYN_PREFLIGHT_MARIADB_ROOT_PASSWORD']
platform_db = os.environ['OTERYN_PREFLIGHT_PLATFORM_DB']
summary_path = os.environ['OTERYN_PREFLIGHT_RESTORE_SUMMARY']
suffix = re.sub(r'[^0-9_]', '_', os.environ['OTERYN_PREFLIGHT_RUN_SUFFIX'])
restore_databases = {
    platform_db: f'oteryn_preflight_platform_{suffix}',
}

for name in [container, platform_db, *restore_databases.values()]:
    if not re.fullmatch(r'[A-Za-z0-9_.-]{1,128}', name):
        raise SystemExit('invalid bounded restore-drill identifier')


def docker_mysql(*args, database=None, input_bytes=None):
    command = [
        'docker', 'exec', '-i', '-e', f'MYSQL_PWD={password}', container,
        'mariadb', '-uroot', '--batch', '--skip-column-names',
    ]
    if database:
        command.append(database)
    command.extend(args)
    return subprocess.run(
        command,
        input=input_bytes,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=True,
    ).stdout


def quote_identifier(value):
    return '`' + value.replace('`', '``') + '`'


def drop_restore_databases():
    for target in restore_databases.values():
        try:
            docker_mysql('-e', f'DROP DATABASE IF EXISTS {quote_identifier(target)};')
        except subprocess.CalledProcessError:
            pass


def dump_command(database):
    return [
        'docker', 'exec', '-e', f'MYSQL_PWD={password}', container,
        'mariadb-dump', '-uroot', '--single-transaction', '--quick',
        '--skip-lock-tables', '--routines', '--events', '--triggers',
        '--skip-comments', database,
    ]


def stream_dump_into_restore(source, target):
    source_process = subprocess.Popen(
        dump_command(source),
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    restore_process = subprocess.Popen(
        [
            'docker', 'exec', '-i', '-e', f'MYSQL_PWD={password}', container,
            'mariadb', '-uroot', target,
        ],
        stdin=subprocess.PIPE,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    digest = hashlib.sha256()
    assert source_process.stdout is not None
    assert restore_process.stdin is not None
    while True:
        chunk = source_process.stdout.read(1024 * 1024)
        if not chunk:
            break
        digest.update(chunk)
        restore_process.stdin.write(chunk)
    restore_process.stdin.close()
    source_rc = source_process.wait()
    restore_rc = restore_process.wait()
    if source_rc != 0 or restore_rc != 0:
        raise SystemExit('streaming database restore failed')
    return digest.hexdigest()


def dump_digest(database):
    process = subprocess.Popen(
        dump_command(database),
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    digest = hashlib.sha256()
    assert process.stdout is not None
    while True:
        chunk = process.stdout.read(1024 * 1024)
        if not chunk:
            break
        digest.update(chunk)
    if process.wait() != 0:
        raise SystemExit('restored database verification dump failed')
    return digest.hexdigest()


def table_count(database):
    query = (
        "SELECT COUNT(*) FROM information_schema.tables "
        f"WHERE table_schema='{database}' AND table_type='BASE TABLE';"
    )
    raw = docker_mysql('-e', query).decode().strip()
    return int(raw)


started = time.monotonic_ns()
drop_restore_databases()
try:
    counts = {}
    for source, target in restore_databases.items():
        docker_mysql(
            '-e',
            f'CREATE DATABASE {quote_identifier(target)} '
            'CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;',
        )
        source_digest = stream_dump_into_restore(source, target)
        restored_digest = dump_digest(target)
        if source_digest != restored_digest:
            raise SystemExit('restored database content digest differs from streamed source dump')
        source_count = table_count(source)
        restored_count = table_count(target)
        if source_count != restored_count:
            raise SystemExit('restored database base-table count differs')
        counts[source] = source_count
finally:
    drop_restore_databases()

elapsed_ms = (time.monotonic_ns() - started) // 1_000_000
with open(summary_path, 'w', encoding='utf-8') as handle:
    json.dump(
        {
            'result': 'PASS',
            'duration_ms': elapsed_ms,
            'platform_table_count': counts[platform_db],
        },
        handle,
        separators=(',', ':'),
    )
PY
fi
if [[ "$run_restore_drill" == "true" ]]; then
    restore_result="$(python3 -c 'import json,os; print(json.load(open(os.environ["OTERYN_PREFLIGHT_RESTORE_SUMMARY"]))["result"])')"
    restore_duration_ms="$(python3 -c 'import json,os; print(json.load(open(os.environ["OTERYN_PREFLIGHT_RESTORE_SUMMARY"]))["duration_ms"])')"
    platform_table_count="$(python3 -c 'import json,os; print(json.load(open(os.environ["OTERYN_PREFLIGHT_RESTORE_SUMMARY"]))["platform_table_count"])')"
fi

validated_at_utc="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
cat > "$evidence_path" <<EOF
{
  "classification": "STAGING_PROVEN",
  "validated_at_utc": "$validated_at_utc",
  "workflow_source_sha": "$validation_sha",
  "target": "local-synology",
  "compose_project": "$project_name",
  "deployed_release_sha": "$deployed_release_sha",
  "platform_source_sha": "$platform_source_sha",
  "gateway_source_sha": "$gateway_source_sha",
  "platform_ghcr_repository": "$platform_repo",
  "gateway_ghcr_repository": "$gateway_repo",
  "platform_image": "$platform_image",
  "gateway_image": "$gateway_image",
  "container_singleton_and_running": "PASS",
  "restart_policies": "PASS",
  "private_network_membership": "PASS",
  "host_bindings_fail_closed": "PASS",
  "database_and_scheduler_unpublished": "PASS",
  "immutable_runtime_images": "PASS",
  "persisted_component_provenance": "PASS",
  "named_persistent_volumes": "PASS",
  "runner_state_and_last_good_snapshot": "PASS",
  "application_and_gateway_health": "PASS",
  "scheduler_running_and_healthy": "PASS",
  "platform_db_engine": "$platform_db_engine",
  "postgres_tls_and_roles": "$postgres_tls_and_roles",
  "database_restore_drill": "$restore_result",
  "restore_duration_ms": $restore_duration_ms,
  "platform_base_table_count": $platform_table_count,
  "local_session_cache_queue_profile": "SINGLE_INSTANCE_FILE_FILE_SYNC",
  "local_mail_profile": "ARRAY_NON_DELIVERY",
  "production_environment_proven": false,
  "remaining_public_production_gaps": [
    "public_dns_tls_cloudflare_waf_and_origin",
    "real_mail_provider_and_sender_domain",
    "external_logging_monitoring_alerting_and_on_call",
    "dsm_backup_schedule_retention_encryption_and_restore_ownership",
    "authoritative_game_login_if_required_by_launch_scope"
  ]
}
EOF
chmod 600 "$evidence_path"

echo "SYNOLOGY_PRODUCTION_TARGET_PREFLIGHT_STAGING_PROVEN release=$deployed_release_sha platform_source=$platform_source_sha gateway_source=$gateway_source_sha restore=$restore_result evidence=$evidence_path"
