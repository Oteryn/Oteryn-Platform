#!/usr/bin/env bash
set -euo pipefail

: "${DB_DATABASE:?DB_DATABASE is required}"
: "${CANARY_DB_DATABASE:?CANARY_DB_DATABASE is required}"
: "${MARIADB_ROOT_PASSWORD:?MARIADB_ROOT_PASSWORD is required}"
: "${CANARY_DB_USERNAME:?CANARY_DB_USERNAME is required}"
: "${CANARY_DB_PASSWORD:?CANARY_DB_PASSWORD is required}"
: "${CANARY_PROVISIONING_DB_USERNAME:?CANARY_PROVISIONING_DB_USERNAME is required}"
: "${CANARY_PROVISIONING_DB_PASSWORD:?CANARY_PROVISIONING_DB_PASSWORD is required}"
: "${CANARY_CHARACTER_CREATE_DB_USERNAME:?CANARY_CHARACTER_CREATE_DB_USERNAME is required}"
: "${CANARY_CHARACTER_CREATE_DB_PASSWORD:?CANARY_CHARACTER_CREATE_DB_PASSWORD is required}"
: "${CANARY_RUNTIME_REDIS_USERNAME:?CANARY_RUNTIME_REDIS_USERNAME is required}"
: "${CANARY_RUNTIME_REDIS_PASSWORD:?CANARY_RUNTIME_REDIS_PASSWORD is required}"

db_admin_host="${ACCEPTANCE_DB_ADMIN_HOST:-${DB_HOST:-127.0.0.1}}"
db_admin_port="${ACCEPTANCE_DB_ADMIN_PORT:-${DB_PORT:-3306}}"
redis_admin_host="${ACCEPTANCE_REDIS_ADMIN_HOST:-${CANARY_RUNTIME_REDIS_HOST:-127.0.0.1}}"
redis_admin_port="${ACCEPTANCE_REDIS_ADMIN_PORT:-${CANARY_RUNTIME_REDIS_PORT:-6379}}"

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

if [[ ! "$CANARY_DB_DATABASE" =~ ^[A-Za-z0-9_]*acceptance[A-Za-z0-9_]*$ ]]; then
    echo "Refusing to recreate a Canary database whose name is not acceptance-scoped." >&2
    exit 1
fi

bootstrap_stage="mariadb-readiness"
for attempt in $(seq 1 30); do
    if mariadb --protocol=tcp -h"$db_admin_host" -P"$db_admin_port" -uroot -p"$MARIADB_ROOT_PASSWORD" -e 'SELECT 1' >/dev/null 2>&1; then
        break
    fi
    sleep 1
done

bootstrap_stage="platform-and-canary-schema"
mariadb --protocol=tcp -h"$db_admin_host" -P"$db_admin_port" -uroot -p"$MARIADB_ROOT_PASSWORD" <<SQL
DROP DATABASE IF EXISTS \`$DB_DATABASE\`;
CREATE DATABASE \`$DB_DATABASE\` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
DROP DATABASE IF EXISTS \`$CANARY_DB_DATABASE\`;
CREATE DATABASE \`$CANARY_DB_DATABASE\` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE \`$CANARY_DB_DATABASE\`.accounts (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    password VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    creation BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE \`$CANARY_DB_DATABASE\`.players (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    group_id BIGINT NOT NULL DEFAULT 1,
    account_id BIGINT UNSIGNED NOT NULL,
    level BIGINT NOT NULL DEFAULT 1,
    vocation BIGINT NOT NULL DEFAULT 0,
    health BIGINT NOT NULL DEFAULT 100,
    healthmax BIGINT NOT NULL DEFAULT 100,
    experience BIGINT NOT NULL DEFAULT 0,
    lookbody BIGINT NOT NULL DEFAULT 0,
    lookfeet BIGINT NOT NULL DEFAULT 0,
    lookhead BIGINT NOT NULL DEFAULT 0,
    looklegs BIGINT NOT NULL DEFAULT 0,
    looktype BIGINT NOT NULL DEFAULT 0,
    lookaddons BIGINT NOT NULL DEFAULT 0,
    maglevel BIGINT NOT NULL DEFAULT 0,
    mana BIGINT NOT NULL DEFAULT 0,
    manamax BIGINT NOT NULL DEFAULT 0,
    manaspent BIGINT NOT NULL DEFAULT 0,
    soul BIGINT NOT NULL DEFAULT 0,
    town_id BIGINT NOT NULL DEFAULT 1,
    posx BIGINT NOT NULL DEFAULT 0,
    posy BIGINT NOT NULL DEFAULT 0,
    posz BIGINT NOT NULL DEFAULT 0,
    conditions BLOB NULL,
    cap BIGINT NOT NULL DEFAULT 0,
    sex BIGINT NOT NULL DEFAULT 0,
    pronoun BIGINT NOT NULL DEFAULT 0,
    lastlogin BIGINT UNSIGNED NOT NULL DEFAULT 0,
    lastlogout BIGINT UNSIGNED NOT NULL DEFAULT 0,
    boss_points BIGINT NOT NULL DEFAULT 0,
    comment VARCHAR(255) NOT NULL DEFAULT '',
    istutorial BIGINT NOT NULL DEFAULT 0,
    skill_fist BIGINT NOT NULL DEFAULT 10,
    skill_fist_tries BIGINT NOT NULL DEFAULT 0,
    skill_club BIGINT NOT NULL DEFAULT 10,
    skill_club_tries BIGINT NOT NULL DEFAULT 0,
    skill_sword BIGINT NOT NULL DEFAULT 10,
    skill_sword_tries BIGINT NOT NULL DEFAULT 0,
    skill_axe BIGINT NOT NULL DEFAULT 10,
    skill_axe_tries BIGINT NOT NULL DEFAULT 0,
    skill_dist BIGINT NOT NULL DEFAULT 10,
    skill_dist_tries BIGINT NOT NULL DEFAULT 0,
    skill_shielding BIGINT NOT NULL DEFAULT 10,
    skill_shielding_tries BIGINT NOT NULL DEFAULT 0,
    skill_fishing BIGINT NOT NULL DEFAULT 10,
    skill_fishing_tries BIGINT NOT NULL DEFAULT 0,
    deletion BIGINT NOT NULL DEFAULT 0,
    INDEX players_account_id_index (account_id),
    INDEX players_vocation_index (vocation),
    INDEX players_level_index (level),
    INDEX players_magic_index (maglevel)
);

CREATE TABLE \`$CANARY_DB_DATABASE\`.guilds (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    ownerid BIGINT UNSIGNED NOT NULL,
    level BIGINT NOT NULL DEFAULT 1,
    creationdata BIGINT NOT NULL DEFAULT 0,
    motd TEXT NOT NULL,
    residence BIGINT NOT NULL DEFAULT 0,
    points BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE \`$CANARY_DB_DATABASE\`.guild_ranks (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    guild_id BIGINT UNSIGNED NOT NULL,
    name VARCHAR(255) NOT NULL,
    level BIGINT NOT NULL DEFAULT 1
);

CREATE TABLE \`$CANARY_DB_DATABASE\`.guild_membership (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    player_id BIGINT UNSIGNED NOT NULL,
    guild_id BIGINT UNSIGNED NOT NULL,
    rank_id BIGINT UNSIGNED NOT NULL,
    nick VARCHAR(255) NOT NULL DEFAULT '',
    INDEX guild_membership_guild_index (guild_id)
);

CREATE TABLE \`$CANARY_DB_DATABASE\`.houses (
    id BIGINT UNSIGNED NOT NULL,
    channel_id BIGINT UNSIGNED NOT NULL DEFAULT 1,
    owner BIGINT UNSIGNED NOT NULL DEFAULT 0,
    name VARCHAR(255) NOT NULL,
    size BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (channel_id, id),
    INDEX houses_owner_index (owner)
);

CREATE TABLE \`$CANARY_DB_DATABASE\`.player_deaths (
    player_id BIGINT UNSIGNED NOT NULL,
    time BIGINT UNSIGNED NOT NULL DEFAULT 0,
    level BIGINT NOT NULL DEFAULT 1,
    killed_by VARCHAR(255) NOT NULL,
    is_player TINYINT NOT NULL DEFAULT 0,
    mostdamage_by VARCHAR(100) NOT NULL DEFAULT '',
    mostdamage_is_player TINYINT NOT NULL DEFAULT 0,
    unjustified TINYINT NOT NULL DEFAULT 0,
    mostdamage_unjustified TINYINT NOT NULL DEFAULT 0,
    participants TEXT NOT NULL,
    INDEX player_deaths_player_time_index (player_id, time),
    INDEX player_deaths_time_index (time),
    INDEX player_deaths_killed_by_index (killed_by)
);

CREATE TABLE \`$CANARY_DB_DATABASE\`.channels (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    pvp_type VARCHAR(64) NOT NULL DEFAULT 'open',
    max_players BIGINT NOT NULL DEFAULT 1000,
    maintenance TINYINT NOT NULL DEFAULT 0,
    maintenance_message VARCHAR(255) NULL,
    enabled TINYINT NOT NULL DEFAULT 1,
    sort_order BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE \`$CANARY_DB_DATABASE\`.cluster_sessions (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    player_id BIGINT UNSIGNED NOT NULL,
    channel_id BIGINT UNSIGNED NOT NULL,
    status VARCHAR(32) NOT NULL,
    expires_at BIGINT NOT NULL,
    INDEX cluster_sessions_player_status_expiry_index (player_id, status, expires_at)
);

INSERT INTO \`$CANARY_DB_DATABASE\`.accounts (id, name, password, email, creation)
VALUES (9001, 'acceptance-public-account', 'not-a-user-credential', 'sink@example.invalid', 0);
INSERT INTO \`$CANARY_DB_DATABASE\`.players (
    id, name, account_id, level, vocation, experience, maglevel, lastlogin, lastlogout,
    boss_points, comment, skill_fist, skill_club, skill_sword, skill_axe, skill_dist,
    skill_shielding, skill_fishing, deletion
)
VALUES
    (9001, 'Acceptance Hero', 9001, 42, 4, 4200000, 12, UNIX_TIMESTAMP() - 300, UNIX_TIMESTAMP() - 900, 77, 'A deterministic public hero comment.', 35, 45, 80, 50, 60, 75, 20, 0),
    (9002, 'Acceptance Guildmate', 9001, 35, 3, 2500000, 8, UNIX_TIMESTAMP() - 600, UNIX_TIMESTAMP() - 1200, 20, 'A second public character.', 30, 40, 45, 35, 78, 65, 18, 0),
    (9003, 'Acceptance Victim', 9001, 30, 2, 1500000, 18, UNIX_TIMESTAMP() - 1800, UNIX_TIMESTAMP() - 2400, 5, '', 20, 25, 30, 20, 40, 35, 15, 0);
INSERT INTO \`$CANARY_DB_DATABASE\`.guilds (id, name, ownerid, level, creationdata, motd, residence, points)
VALUES (9001, 'Acceptance Guild', 9001, 3, 0, 'Deterministic acceptance guild', 1, 100);
INSERT INTO \`$CANARY_DB_DATABASE\`.guild_ranks (id, guild_id, name, level)
VALUES (9001, 9001, 'Leader', 3), (9002, 9001, 'Member', 1);
INSERT INTO \`$CANARY_DB_DATABASE\`.guild_membership (player_id, guild_id, rank_id, nick)
VALUES (9001, 9001, 9001, ''), (9002, 9001, 9002, 'Acceptance');
INSERT INTO \`$CANARY_DB_DATABASE\`.houses (id, channel_id, owner, name, size)
VALUES (100, 1, 9001, 'Acceptance Hall', 42);
INSERT INTO \`$CANARY_DB_DATABASE\`.player_deaths (
    player_id, time, level, killed_by, is_player, mostdamage_by, mostdamage_is_player,
    unjustified, mostdamage_unjustified, participants
)
VALUES
    (9001, UNIX_TIMESTAMP() - 120, 42, 'Acceptance Dragon', 0, 'Acceptance Dragon', 0, 0, 0, '[]'),
    (9003, UNIX_TIMESTAMP() - 60, 30, 'Acceptance Hero', 1, 'Acceptance Hero', 1, 0, 0, '[]');
INSERT INTO \`$CANARY_DB_DATABASE\`.channels (id, name, pvp_type, max_players, maintenance, maintenance_message, enabled, sort_order)
VALUES (1, 'Acceptance', 'open', 1000, 0, NULL, 1, 1);
INSERT INTO \`$CANARY_DB_DATABASE\`.cluster_sessions (player_id, channel_id, status, expires_at)
VALUES (9001, 1, 'ONLINE', ROUND(UNIX_TIMESTAMP(CURRENT_TIMESTAMP(3)) * 1000) + 3600000);
SQL

bootstrap_stage="canary-readonly-principal"
sed \
    -e "s|{{OTERYN_CANARY_DB_USER}}|$CANARY_DB_USERNAME|g" \
    -e 's|{{OTERYN_CANARY_DB_HOST}}|%|g' \
    -e "s|{{OTERYN_CANARY_DB_PASSWORD}}|$CANARY_DB_PASSWORD|g" \
    -e "s|{{CANARY_DB_NAME}}|$CANARY_DB_DATABASE|g" \
    database/provisioning/canary-readonly.sql.template \
    | mariadb --protocol=tcp -h"$db_admin_host" -P"$db_admin_port" -uroot -p"$MARIADB_ROOT_PASSWORD" >/dev/null

bootstrap_stage="canary-provisioning-principal"
sed \
    -e "s|{{OTERYN_CANARY_PROVISIONING_DB_USER}}|$CANARY_PROVISIONING_DB_USERNAME|g" \
    -e 's|{{OTERYN_CANARY_PROVISIONING_DB_HOST}}|%|g' \
    -e "s|{{OTERYN_CANARY_PROVISIONING_DB_PASSWORD}}|$CANARY_PROVISIONING_DB_PASSWORD|g" \
    -e "s|{{CANARY_DB_NAME}}|$CANARY_DB_DATABASE|g" \
    database/provisioning/canary-provisioning.sql.template \
    | mariadb --protocol=tcp -h"$db_admin_host" -P"$db_admin_port" -uroot -p"$MARIADB_ROOT_PASSWORD" >/dev/null

bootstrap_stage="canary-character-create-principal"
sed \
    -e "s|{{OTERYN_CANARY_CHARACTER_CREATE_DB_USER}}|$CANARY_CHARACTER_CREATE_DB_USERNAME|g" \
    -e 's|{{OTERYN_CANARY_CHARACTER_CREATE_DB_HOST}}|%|g' \
    -e "s|{{OTERYN_CANARY_CHARACTER_CREATE_DB_PASSWORD}}|$CANARY_CHARACTER_CREATE_DB_PASSWORD|g" \
    -e "s|{{CANARY_DB_NAME}}|$CANARY_DB_DATABASE|g" \
    database/provisioning/canary-character-create.sql.template \
    | mariadb --protocol=tcp -h"$db_admin_host" -P"$db_admin_port" -uroot -p"$MARIADB_ROOT_PASSWORD" >/dev/null

bootstrap_stage="canary-runtime-redis"
redis-cli -h "$redis_admin_host" -p "$redis_admin_port" ACL SETUSER "$CANARY_RUNTIME_REDIS_USERNAME" reset on ">$CANARY_RUNTIME_REDIS_PASSWORD" resetkeys '~cluster:channel:*:runtime' -@all +hmget +pttl +ping +select >/dev/null
redis-cli -h "$redis_admin_host" -p "$redis_admin_port" HSET cluster:channel:1:runtime channel_id 1 status ONLINE players_online 1 >/dev/null
redis-cli -h "$redis_admin_host" -p "$redis_admin_port" PEXPIRE cluster:channel:1:runtime 3600000 >/dev/null

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
