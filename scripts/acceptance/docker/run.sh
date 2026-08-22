#!/usr/bin/env bash
set -euo pipefail

cd /workspace

profile="${PORTAL_E2E_PROFILE:-smoke}"
artifacts_root="/workspace/artifacts/docker-portal-e2e"
mkdir -p "$artifacts_root"

case "$profile" in
  smoke|critical|full|account-lifecycle|portability|responsive|resilience|accessibility|coverage-strict) ;;
  *)
    echo "Unsupported PORTAL_E2E_PROFILE: $profile" >&2
    exit 2
    ;;
esac

export APP_KEY="${APP_KEY:-$(php -r 'echo "base64:".base64_encode(random_bytes(32));')}"
export ACCEPTANCE_RUN_ID="docker-${profile}-$(date +%s)"

composer install --no-interaction --prefer-dist --no-progress
mkdir -p storage/framework/cache/data storage/framework/sessions storage/framework/views storage/logs bootstrap/cache

bash scripts/acceptance/bootstrap-production-like.sh

# Legacy acceptance tests intentionally mutate the local admin endpoint at 127.0.0.1.
# In Docker, bridge that loopback contract to the isolated MariaDB service.
socat TCP-LISTEN:3306,bind=127.0.0.1,reuseaddr,fork TCP:${ACCEPTANCE_DB_ADMIN_HOST:-${DB_HOST}}:${ACCEPTANCE_DB_ADMIN_PORT:-${DB_PORT:-3306}} &
db_proxy_pid=$!
socat TCP-LISTEN:6379,bind=127.0.0.1,reuseaddr,fork TCP:${ACCEPTANCE_REDIS_ADMIN_HOST:-${CANARY_RUNTIME_REDIS_HOST}}:${ACCEPTANCE_REDIS_ADMIN_PORT:-${CANARY_RUNTIME_REDIS_PORT:-6379}} &
redis_proxy_pid=$!

server_log="$artifacts_root/laravel-${profile}.log"
php artisan serve --host=127.0.0.1 --port=8080 >"$server_log" 2>&1 &
server_pid=$!
cleanup_server() {
  kill "$db_proxy_pid" 2>/dev/null || true
  kill "$redis_proxy_pid" 2>/dev/null || true
  kill "$server_pid" 2>/dev/null || true
  wait "$server_pid" 2>/dev/null || true
}
trap cleanup_server EXIT INT TERM

for attempt in $(seq 1 30); do
  if curl -fsS http://127.0.0.1:8080/health >/dev/null; then
    break
  fi
  if [[ "$attempt" -eq 30 ]]; then
    echo "Laravel acceptance runtime did not become healthy." >&2
    tail -n 120 "$server_log" >&2 || true
    exit 1
  fi
  sleep 1
done

run_npm_profile() {
  local label="$1"
  local npm_status=0
  shift
  echo "==> portal-e2e profile: $label"
  npm --prefix scripts/acceptance run "$@" || npm_status=$?
  if [[ -f artifacts/acceptance/junit.xml ]]; then
    cp artifacts/acceptance/junit.xml "$artifacts_root/junit-${label}.xml"
  fi
  return "$npm_status"
}

started_at="$(date +%s)"
status=0
set +e
case "$profile" in
  smoke)
    run_npm_profile smoke test:smoke
    status=$?
    ;;
  portability)
    run_npm_profile portability test:portability
    status=$?
    ;;
  responsive)
    run_npm_profile responsive test:responsive
    status=$?
    ;;
  resilience)
    run_npm_profile resilience test:resilience
    status=$?
    ;;
  accessibility)
    run_npm_profile accessibility test:accessibility
    status=$?
    ;;
  account-lifecycle)
    run_npm_profile account-lifecycle test:account-lifecycle
    status=$?
    ;;
  coverage-strict)
    run_npm_profile coverage-strict test:coverage-contract:strict
    status=$?
    ;;
  critical)
    export ACCEPTANCE_PROFILE=critical
    run_npm_profile smoke test:smoke || status=$?
    if [[ "$status" -eq 0 ]]; then run_npm_profile portability test:portability || status=$?; fi
    if [[ "$status" -eq 0 ]]; then run_npm_profile responsive test:responsive || status=$?; fi
    if [[ "$status" -eq 0 ]]; then run_npm_profile resilience test:resilience || status=$?; fi
    if [[ "$status" -eq 0 ]]; then run_npm_profile accessibility test:accessibility || status=$?; fi
    ;;
  full)
    export ACCEPTANCE_PROFILE=full
    run_npm_profile full test || status=$?
    if [[ "$status" -eq 0 ]]; then run_npm_profile resilience test:resilience || status=$?; fi
    if [[ "$status" -eq 0 ]]; then run_npm_profile accessibility test:accessibility || status=$?; fi
    ;;
esac
set -e

duration="$(( $(date +%s) - started_at ))"
result="FAIL"
if [[ "$status" -eq 0 ]]; then result="PASS"; fi
printf '{"profile":"%s","exact_tested_sha":"%s","result":"%s","duration_seconds":%s}\n' \
  "$profile" "${ACCEPTANCE_SHA:-local-unknown}" "$result" "$duration" \
  > "$artifacts_root/result-${profile}.json"

exit "$status"
