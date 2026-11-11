#!/usr/bin/env bash
set -euo pipefail

: "${VALIDATION_SHA:?VALIDATION_SHA is required}"
: "${BASE_SHA:?BASE_SHA is required}"
: "${MARIADB_ROOT_PASSWORD:?MARIADB_ROOT_PASSWORD is required}"
: "${GITHUB_ENV:?GITHUB_ENV is required}"
: "${GITHUB_STEP_SUMMARY:?GITHUB_STEP_SUMMARY is required}"

UPGRADE_DATABASE="oteryn_upgrade"
UPGRADE_EMAIL="phase7-upgrade@staging.oteryn"
UPGRADE_NEWS_SLUG="phase7-upgrade-fixture"
UPGRADE_NEWS_TITLE="Phase 7 existing-data upgrade fixture"
UPGRADE_NEWS_BODY="Synthetic non-production row used to validate existing-data upgrade and rollback compatibility."
BASE_RELEASE="/tmp/phase7-releases/${BASE_SHA}"
CANDIDATE_RELEASE="/tmp/phase7-releases/${VALIDATION_SHA}"
CURRENT_LINK="/tmp/phase7-current"

for release in "$BASE_RELEASE" "$CANDIDATE_RELEASE"; do
  if [ ! -f "$release/artisan" ]; then
    echo "Expected prepared release is missing: $release" >&2
    exit 1
  fi
done

mysql_root() {
  mariadb --protocol=tcp -N -h127.0.0.1 -uroot -p"$MARIADB_ROOT_PASSWORD" "$@"
}

read_dataset_fingerprint() {
  mysql_root -e "SELECT SHA2(CONCAT((SELECT email FROM ${UPGRADE_DATABASE}.identities WHERE email='${UPGRADE_EMAIL}' LIMIT 1),'|',(SELECT SHA2(password, 256) FROM ${UPGRADE_DATABASE}.identities WHERE email='${UPGRADE_EMAIL}' LIMIT 1),'|',(SELECT slug FROM ${UPGRADE_DATABASE}.news_posts WHERE slug='${UPGRADE_NEWS_SLUG}' LIMIT 1),'|',(SELECT title FROM ${UPGRADE_DATABASE}.news_posts WHERE slug='${UPGRADE_NEWS_SLUG}' LIMIT 1),'|',(SELECT body FROM ${UPGRADE_DATABASE}.news_posts WHERE slug='${UPGRADE_NEWS_SLUG}' LIMIT 1)), 256);"
}

assert_dataset() {
  local expected_fingerprint="$1"
  local phase="$2"
  local identity_count news_count fingerprint

  identity_count="$(mysql_root -e "SELECT COUNT(*) FROM ${UPGRADE_DATABASE}.identities WHERE email='${UPGRADE_EMAIL}';")"
  news_count="$(mysql_root -e "SELECT COUNT(*) FROM ${UPGRADE_DATABASE}.news_posts WHERE slug='${UPGRADE_NEWS_SLUG}' AND title='${UPGRADE_NEWS_TITLE}' AND body='${UPGRADE_NEWS_BODY}';")"
  fingerprint="$(read_dataset_fingerprint)"

  test "$identity_count" = "1"
  test "$news_count" = "1"
  test -n "$fingerprint"
  test "$fingerprint" = "$expected_fingerprint"
  echo "Representative dataset intact after ${phase}."
}

run_release_smoke() {
  local release_sha="$1"
  local phase="$2"
  local port="$3"
  local release_dir="/tmp/phase7-releases/${release_sha}"
  local log_file="/tmp/phase7-upgrade-${phase}.log"
  local body_file="/tmp/phase7-upgrade-${phase}.body"

  ln -sfn "$release_dir" "$CURRENT_LINK"
  test "$(readlink -f "$CURRENT_LINK")" = "$release_dir"

  (
    cd "$CURRENT_LINK"
    DB_DATABASE="$UPGRADE_DATABASE" php artisan production:verify-configuration
    DB_DATABASE="$UPGRADE_DATABASE" php artisan migrate:status --no-interaction >/dev/null
    DB_DATABASE="$UPGRADE_DATABASE" php artisan serve --host=127.0.0.1 --port="$port" >"$log_file" 2>&1 &
    server_pid=$!
    trap 'kill "$server_pid" 2>/dev/null || true; wait "$server_pid" 2>/dev/null || true' EXIT

    for attempt in $(seq 1 30); do
      if curl -fsS "http://127.0.0.1:${port}/health" >/dev/null; then
        break
      fi
      sleep 1
    done

    curl -fsS "http://127.0.0.1:${port}/health" >/dev/null
    curl -fsS "http://127.0.0.1:${port}/news/${UPGRADE_NEWS_SLUG}" >"$body_file"
    grep -Fq "$UPGRADE_NEWS_TITLE" "$body_file"
    grep -Fq "$UPGRADE_NEWS_BODY" "$body_file"
    ! grep -qi 'stack trace' "$body_file"
    ! grep -Fq "${APP_KEY:-not-present}" "$body_file"
    ! grep -Fq "${APP_KEY:-not-present}" "$log_file"
  )
}

mysql_root -e "DROP DATABASE IF EXISTS ${UPGRADE_DATABASE}; CREATE DATABASE ${UPGRADE_DATABASE} CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;"

(
  cd "$BASE_RELEASE"
  DB_DATABASE="$UPGRADE_DATABASE" php artisan migrate --force --no-interaction
)

BASE_MIGRATION_COUNT="$(mysql_root -e "SELECT COUNT(*) FROM ${UPGRADE_DATABASE}.migrations;")"
UPGRADE_PASSWORD_HASH="$(php -r 'echo password_hash("phase7-upgrade-non-user-credential", PASSWORD_ARGON2ID);')"

mysql_root "$UPGRADE_DATABASE" <<SQL
INSERT INTO identities (email, password, created_at, updated_at)
VALUES ('${UPGRADE_EMAIL}', '${UPGRADE_PASSWORD_HASH}', NOW(), NOW());

INSERT INTO news_posts (slug, title, body, published_at, created_at, updated_at)
VALUES ('${UPGRADE_NEWS_SLUG}', '${UPGRADE_NEWS_TITLE}', '${UPGRADE_NEWS_BODY}', NOW(), NOW(), NOW());
SQL

BASELINE_FINGERPRINT="$(read_dataset_fingerprint)"
test -n "$BASELINE_FINGERPRINT"
assert_dataset "$BASELINE_FINGERPRINT" "BASE_SHA seed"

(
  cd "$CANDIDATE_RELEASE"
  DB_DATABASE="$UPGRADE_DATABASE" php artisan migrate --force --no-interaction
  DB_DATABASE="$UPGRADE_DATABASE" php artisan migrate:status --no-interaction >/dev/null
)

CANDIDATE_MIGRATION_COUNT="$(mysql_root -e "SELECT COUNT(*) FROM ${UPGRADE_DATABASE}.migrations;")"
test "$CANDIDATE_MIGRATION_COUNT" -ge "$BASE_MIGRATION_COUNT"
assert_dataset "$BASELINE_FINGERPRINT" "candidate migration"
run_release_smoke "$VALIDATION_SHA" "candidate" 8081

assert_dataset "$BASELINE_FINGERPRINT" "pre-rollback"
run_release_smoke "$BASE_SHA" "rollback" 8082
assert_dataset "$BASELINE_FINGERPRINT" "rollback smoke"

(
  cd "$CANDIDATE_RELEASE"
  DB_DATABASE="$UPGRADE_DATABASE" php artisan migrate --force --no-interaction
)
assert_dataset "$BASELINE_FINGERPRINT" "candidate redeploy"
run_release_smoke "$VALIDATION_SHA" "redeploy" 8083
assert_dataset "$BASELINE_FINGERPRINT" "redeploy smoke"

ln -sfn "$CANDIDATE_RELEASE" "$CURRENT_LINK"
test "$(readlink -f "$CURRENT_LINK")" = "$CANDIDATE_RELEASE"

cat > /tmp/phase7-existing-data-upgrade-evidence.json <<JSON
{
  "classification": "STAGING_PROVEN",
  "validation_sha": "${VALIDATION_SHA}",
  "rollback_sha": "${BASE_SHA}",
  "dataset": "synthetic_identity_and_published_news",
  "base_migration_count": ${BASE_MIGRATION_COUNT},
  "candidate_migration_count": ${CANDIDATE_MIGRATION_COUNT},
  "existing_data_upgrade": "PASS",
  "candidate_application_smoke": "PASS",
  "rollback_code_with_post_upgrade_database": "PASS",
  "rollback_application_smoke": "PASS",
  "redeploy_application_smoke": "PASS",
  "dataset_fingerprint_preserved": "PASS"
}
JSON

{
  echo "### Phase 7 existing-data upgrade and rollback evidence"
  echo "- validation_sha: \`${VALIDATION_SHA}\`"
  echo "- rollback_sha: \`${BASE_SHA}\`"
  echo "- base_migration_count: ${BASE_MIGRATION_COUNT}"
  echo "- candidate_migration_count: ${CANDIDATE_MIGRATION_COUNT}"
  echo "- synthetic_existing_data_upgrade: PASS"
  echo "- candidate_application_smoke: PASS"
  echo "- rollback_code_with_post_upgrade_database: PASS"
  echo "- rollback_application_smoke: PASS"
  echo "- redeploy_application_smoke: PASS"
  echo "- dataset_fingerprint_preserved: PASS"
  echo "- classification: STAGING_PROVEN"
} >> "$GITHUB_STEP_SUMMARY"
