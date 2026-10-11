#!/usr/bin/env bash
set -euo pipefail

: "${DB_DATABASE:?DB_DATABASE is required}"
: "${MARIADB_ROOT_PASSWORD:?MARIADB_ROOT_PASSWORD is required}"

db_admin_host="${ACCEPTANCE_DB_ADMIN_HOST:-${DB_HOST:-127.0.0.1}}"
db_admin_port="${ACCEPTANCE_DB_ADMIN_PORT:-${DB_PORT:-3306}}"

bootstrap_stage="preflight"
bootstrap_diagnostic_file="${ACCEPTANCE_BOOTSTRAP_DIAGNOSTIC_FILE:-artifacts/acceptance/bootstrap-production-like.json}"

write_bootstrap_diagnostic() {
    local result="$1"
    local exit_code="$2"

    mkdir -p "$(dirname "$bootstrap_diagnostic_file")"
    printf '{"exact_tested_sha":"%s","stage":"%s","result":"%s","exit_code":%s}\n' \
        "${ACCEPTANCE_SHA:-local-unknown}" \
        "$bootstrap_stage" \
        "$result" \
        "$exit_code" \
        > "$bootstrap_diagnostic_file"
}

on_bootstrap_error() {
    local exit_code=$?
    trap - ERR
    write_bootstrap_diagnostic failure "$exit_code"
    exit "$exit_code"
}

trap on_bootstrap_error ERR

if [[ ! "$DB_DATABASE" =~ ^[A-Za-z0-9_]*acceptance[A-Za-z0-9_]*$ ]]; then
    echo "Refusing to recreate a Platform database whose name is not acceptance-scoped." >&2
    exit 1
fi

bootstrap_stage="mariadb-readiness"
for attempt in $(seq 1 30); do
    if mariadb --protocol=tcp -h"$db_admin_host" -P"$db_admin_port" -uroot -p"$MARIADB_ROOT_PASSWORD" -e 'SELECT 1' >/dev/null 2>&1; then
        break
    fi
    sleep 1
done

bootstrap_stage="platform-schema"
mariadb --protocol=tcp -h"$db_admin_host" -P"$db_admin_port" -uroot -p"$MARIADB_ROOT_PASSWORD" <<SQL
DROP DATABASE IF EXISTS \`$DB_DATABASE\`;
CREATE DATABASE \`$DB_DATABASE\` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
SQL

bootstrap_stage="laravel-config-clear"
php artisan config:clear --no-interaction

bootstrap_stage="platform-migrate-fresh"
migration_log="/tmp/oteryn-acceptance-platform-migrate-fresh.log"
if ! php artisan migrate:fresh --force --no-interaction >"$migration_log" 2>&1; then
    mkdir -p artifacts/acceptance
    tail -n 160 "$migration_log" > artifacts/acceptance/platform-migrate-fresh.log
    cat artifacts/acceptance/platform-migrate-fresh.log >&2
    write_bootstrap_diagnostic failure 1
    trap - ERR
    exit 1
fi
rm -f "$migration_log"

bootstrap_stage="complete"
write_bootstrap_diagnostic success 0
trap - ERR

echo "Acceptance production-like dependencies are ready for exact SHA ${ACCEPTANCE_SHA:-local-unknown}."
