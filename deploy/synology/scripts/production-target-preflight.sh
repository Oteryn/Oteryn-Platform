#!/usr/bin/env bash
set -euo pipefail

project_name="${OTERYN_COMPOSE_PROJECT_NAME:-oteryn-staging}"
validation_sha="${VALIDATION_SHA:-UNKNOWN}"
evidence_path="${OTERYN_PREFLIGHT_EVIDENCE_PATH:-/tmp/synology-production-target-preflight-evidence.json}"
state_dir="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"
run_restore_drill="${OTERYN_RUN_RESTORE_DRILL:-true}"
platform_db="${OTERYN_PLATFORM_DB_NAME:-oteryn_platform}"
canary_db="${OTERYN_CANARY_DB_NAME:-canary}"
work_dir="${OTERYN_PREFLIGHT_WORK_DIR:-/tmp/oteryn-synology-production-target-preflight}"

mariadb_root_password="${MARIADB_ROOT_PASSWORD:-}"
redis_password="${REDIS_PASSWORD:-}"
runtime_redis_password="${CANARY_RUNTIME_REDIS_PASSWORD:-}"

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

[[ "$project_name" =~ ^[A-Za-z0-9][A-Za-z0-9_.-]{0,63}$ ]] \
    || fail "OTERYN_COMPOSE_PROJECT_NAME is not a bounded Compose project name"
[[ "$platform_db" =~ ^[A-Za-z0-9_]{1,64}$ ]] || fail "OTERYN_PLATFORM_DB_NAME is invalid"
[[ "$canary_db" =~ ^[A-Za-z0-9_]{1,64}$ ]] || fail "OTERYN_CANARY_DB_NAME is invalid"
[[ "$run_restore_drill" == "true" || "$run_restore_drill" == "false" ]] \
    || fail "OTERYN_RUN_RESTORE_DRILL must be true or false"
[[ -n "$mariadb_root_password" ]] || fail "MARIADB_ROOT_PASSWORD is required"
[[ -n "$redis_password" ]] || fail "REDIS_PASSWORD is required"
[[ -n "$runtime_redis_password" ]] || fail "CANARY_RUNTIME_REDIS_PASSWORD is required"

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

for service in mariadb redis canary platform internal-proxy gateway; do
    find_single_container "$service"
done

for service in mariadb redis canary platform internal-proxy gateway; do
    id="${containers[$service]}"
    [[ "$(docker inspect --format '{{.State.Running}}' "$id")" == "true" ]] \
        || fail "$service is not running"
    [[ "$(docker inspect --format '{{.HostConfig.RestartPolicy.Name}}' "$id")" == "unless-stopped" ]] \
        || fail "$service does not use restart policy unless-stopped"
done

for service in mariadb redis; do
    id="${containers[$service]}"
    health="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}' "$id")"
    [[ "$health" == "healthy" ]] || fail "$service health status is $health"
done

expected_network="${project_name}_private"
[[ "$(docker network inspect --format '{{.Driver}}' "$expected_network")" == "bridge" ]] \
    || fail "$expected_network is not a bridge network"
for service in mariadb redis canary platform internal-proxy gateway; do
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
[[ "$(published_binding canary 7171)" == "127.0.0.1:7171" ]] \
    || fail "Canary legacy login is not bound exactly to 127.0.0.1:7171"

game_binding="$(published_binding canary 7172)"
python3 - "$game_binding" <<'PY'
import ipaddress
import sys

try:
    host, port = sys.argv[1].rsplit(':', 1)
    address = ipaddress.ip_address(host)
except ValueError as exc:
    raise SystemExit('invalid Canary game binding') from exc

if port != '7172':
    raise SystemExit('Canary game port must remain 7172')
if address.version != 4 or not (address.is_private or address.is_loopback):
    raise SystemExit('Canary game binding must remain loopback or one exact private IPv4 address')
PY

assert_no_published_ports() {
    local service="$1"
    local published
    published="$(docker inspect --format '{{range $port, $bindings := .NetworkSettings.Ports}}{{if $bindings}}{{println $port}}{{end}}{{end}}' "${containers[$service]}")"
    [[ -z "$published" ]] || fail "$service unexpectedly publishes a host port"
}

assert_no_published_ports mariadb
assert_no_published_ports redis
assert_no_published_ports internal-proxy

python3 - \
    "$(docker inspect --format '{{json .NetworkSettings.Ports}}' "${containers[platform]}")" \
    "$(docker inspect --format '{{json .NetworkSettings.Ports}}' "${containers[gateway]}")" \
    "$(docker inspect --format '{{json .NetworkSettings.Ports}}' "${containers[canary]}")" <<'PY'
import json
import sys

expected_sets = [
    {'8000/tcp'},
    {'8080/tcp'},
    {'7171/tcp', '7172/tcp'},
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
canary_image="$(docker inspect --format '{{.Config.Image}}' "${containers[canary]}")"

[[ "$platform_image" =~ ^ghcr\.io/blakinio/oteryn-platform@sha256:([a-f0-9]{64})$ ]] \
    || fail "Platform is not deployed by immutable digest"
platform_digest="sha256:${BASH_REMATCH[1]}"
[[ "$gateway_image" =~ ^ghcr\.io/blakinio/oteryn-game-gateway@sha256:([a-f0-9]{64})$ ]] \
    || fail "Gateway is not deployed by immutable digest"
gateway_digest="sha256:${BASH_REMATCH[1]}"
[[ "$canary_image" =~ @sha256:([a-f0-9]{64})$ ]] \
    || fail "Canary is not deployed by immutable digest"
canary_digest="sha256:${BASH_REMATCH[1]}"

deployed_release_sha=""
for service in platform gateway canary; do
    image_id="$(docker inspect --format '{{.Image}}' "${containers[$service]}")"
    [[ "$image_id" =~ ^sha256:[a-f0-9]{64}$ ]] || fail "$service image ID is not immutable"

    if [[ "$service" == "platform" || "$service" == "gateway" ]]; then
        image_revision="$(docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "$image_id")"
        [[ "$image_revision" =~ ^[a-f0-9]{40}$ ]] \
            || fail "$service image is missing an exact org.opencontainers.image.revision"
        if [[ -z "$deployed_release_sha" ]]; then
            deployed_release_sha="$image_revision"
        elif [[ "$image_revision" != "$deployed_release_sha" ]]; then
            fail "Gateway OCI revision does not match the Platform release SHA"
        fi
    fi
done
[[ -n "$deployed_release_sha" ]] || fail "unable to recover deployed release SHA from immutable image metadata"

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
assert_named_volume redis /data
assert_named_volume platform /var/www/html/storage
assert_named_volume canary /data
assert_named_volume internal-proxy /etc/nginx/tls

[[ -d "$state_dir" ]] || fail "persistent runner state directory is missing"
[[ "$(stat -c '%a' "$state_dir")" == "700" ]] \
    || fail "persistent runner state directory must have mode 700"
state_file="$state_dir/last-good.env"
[[ -f "$state_file" ]] || fail "last-good runtime image snapshot is missing"
[[ "$(stat -c '%a' "$state_file")" == "600" ]] \
    || fail "last-good runtime image snapshot must have mode 600"
for marker in PLATFORM_IMAGE GATEWAY_IMAGE CANARY_IMAGE; do
    grep -Eq "^${marker}=" "$state_file" || fail "last-good snapshot is missing $marker"
done

docker exec "${containers[platform]}" sh -ec '
    test "$APP_ENV" = staging
    test "$APP_DEBUG" = false
    test "$LOG_CHANNEL" = stderr_json
    test "$SESSION_DRIVER" = file
    test "$SESSION_HTTP_ONLY" = true
    test "$CACHE_STORE" = file
    test "$QUEUE_CONNECTION" = sync
    test "$MAIL_MAILER" = array
'

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

docker run --rm \
    --network "container:${containers[canary]}" \
    alpine:3.22 \
    /bin/sh -ec 'nc -z -w 3 127.0.0.1 7172' \
    || fail "Canary game TCP is not reachable in its network namespace"

docker exec "${containers[platform]}" php artisan canary:verify-db-privileges >/dev/null
docker exec "${containers[platform]}" php artisan canary:verify-provisioning-db-privileges >/dev/null
docker exec "${containers[platform]}" php artisan canary:verify-character-create-db-privileges >/dev/null

appendonly="$(docker exec -e REDISCLI_AUTH="$redis_password" "${containers[redis]}" \
    redis-cli --no-auth-warning --raw CONFIG GET appendonly | tail -n 1)"
[[ "$appendonly" == "yes" ]] || fail "Redis append-only persistence is not enabled"

docker exec "${containers[redis]}" redis-cli --no-auth-warning \
    --user oteryn_runtime --pass "$runtime_redis_password" PING \
    | grep -qx 'PONG' \
    || fail "dedicated runtime Redis ACL user cannot execute PING"

redis_write_output="$(docker exec "${containers[redis]}" redis-cli --no-auth-warning \
    --user oteryn_runtime --pass "$runtime_redis_password" \
    SET oteryn:preflight:write-denial forbidden 2>&1 || true)"
[[ "$redis_write_output" == *NOPERM* ]] \
    || fail "dedicated runtime Redis ACL user was not denied a write command"

restore_result="NOT_RUN"
restore_duration_ms=0
platform_table_count=0
canary_table_count=0

if [[ "$run_restore_drill" == "true" ]]; then
    export OTERYN_PREFLIGHT_MARIADB_CONTAINER="${containers[mariadb]}"
    export OTERYN_PREFLIGHT_MARIADB_ROOT_PASSWORD="$mariadb_root_password"
    export OTERYN_PREFLIGHT_PLATFORM_DB="$platform_db"
    export OTERYN_PREFLIGHT_CANARY_DB="$canary_db"
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
canary_db = os.environ['OTERYN_PREFLIGHT_CANARY_DB']
summary_path = os.environ['OTERYN_PREFLIGHT_RESTORE_SUMMARY']
suffix = re.sub(r'[^0-9_]', '_', os.environ['OTERYN_PREFLIGHT_RUN_SUFFIX'])
restore_databases = {
    platform_db: f'oteryn_preflight_platform_{suffix}',
    canary_db: f'oteryn_preflight_canary_{suffix}',
}

for name in [container, platform_db, canary_db, *restore_databases.values()]:
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
            'canary_table_count': counts[canary_db],
        },
        handle,
        separators=(',', ':'),
    )
PY

    restore_result="$(python3 -c 'import json,os; print(json.load(open(os.environ["OTERYN_PREFLIGHT_RESTORE_SUMMARY"]))["result"])')"
    restore_duration_ms="$(python3 -c 'import json,os; print(json.load(open(os.environ["OTERYN_PREFLIGHT_RESTORE_SUMMARY"]))["duration_ms"])')"
    platform_table_count="$(python3 -c 'import json,os; print(json.load(open(os.environ["OTERYN_PREFLIGHT_RESTORE_SUMMARY"]))["platform_table_count"])')"
    canary_table_count="$(python3 -c 'import json,os; print(json.load(open(os.environ["OTERYN_PREFLIGHT_RESTORE_SUMMARY"]))["canary_table_count"])')"
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
  "platform_image": "$platform_image",
  "gateway_image": "$gateway_image",
  "canary_image_digest": "$canary_digest",
  "container_singleton_and_running": "PASS",
  "restart_policies": "PASS",
  "private_network_membership": "PASS",
  "host_bindings_fail_closed": "PASS",
  "database_and_redis_unpublished": "PASS",
  "immutable_runtime_images": "PASS",
  "named_persistent_volumes": "PASS",
  "runner_state_and_last_good_snapshot": "PASS",
  "application_and_gateway_health": "PASS",
  "canary_game_tcp": "PASS",
  "canary_effective_grant_verifiers": "PASS",
  "redis_aof_and_acl": "PASS",
  "database_restore_drill": "$restore_result",
  "restore_duration_ms": $restore_duration_ms,
  "platform_base_table_count": $platform_table_count,
  "canary_base_table_count": $canary_table_count,
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

echo "SYNOLOGY_PRODUCTION_TARGET_PREFLIGHT_STAGING_PROVEN release=$deployed_release_sha restore=$restore_result evidence=$evidence_path"
