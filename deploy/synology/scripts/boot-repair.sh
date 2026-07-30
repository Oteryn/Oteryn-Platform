#!/bin/sh
# Oteryn Synology boot repair.
# Intended for DSM Task Scheduler: Triggered Task -> Boot-up -> User root.
# Targets only the oteryn-staging Compose project and the
# oteryn-synology-staging GitHub Actions runner.

set -u

LOG_FILE="/var/log/oteryn-synology-boot.log"
exec >>"$LOG_FILE" 2>&1

echo "============================================================"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Oteryn boot repair started"

find_docker() {
    for candidate in \
        /usr/local/bin/docker \
        /var/packages/ContainerManager/target/usr/bin/docker \
        /var/packages/Docker/target/usr/bin/docker; do
        if [ -x "$candidate" ]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done

    if command -v docker >/dev/null 2>&1; then
        command -v docker
        return 0
    fi

    return 1
}

DOCKER="$(find_docker || true)"
if [ -z "$DOCKER" ]; then
    echo "ERROR: Docker/Container Manager CLI was not found."
    exit 1
fi

echo "Docker CLI: $DOCKER"

# Container Manager may need time to initialize after DSM starts.
attempt=0
while ! "$DOCKER" info >/dev/null 2>&1; do
    attempt=$((attempt + 1))
    if [ "$attempt" -ge 60 ]; then
        echo "ERROR: Docker daemon did not become ready within 300 seconds."
        exit 1
    fi
    sleep 5
done

echo "Docker daemon is ready."

set_policy_and_start_ids() {
    description="$1"
    ids="$2"

    if [ -z "$ids" ]; then
        echo "WARNING: No container found for $description"
        return 0
    fi

    for id in $ids; do
        name="$($DOCKER inspect --format '{{.Name}}' "$id" 2>/dev/null | sed 's#^/##')"
        echo "Repairing $description: ${name:-$id}"
        "$DOCKER" update --restart always "$id" >/dev/null
        "$DOCKER" unpause "$id" >/dev/null 2>&1 || true
        "$DOCKER" start "$id" >/dev/null 2>&1 || true
    done
}

project_ids_for_service() {
    service="$1"
    "$DOCKER" ps -aq \
        --filter "label=com.docker.compose.project=oteryn-staging" \
        --filter "label=com.docker.compose.service=$service"
}

# Start stateful dependencies first.
set_policy_and_start_ids "Oteryn MariaDB" "$(project_ids_for_service mariadb)"
set_policy_and_start_ids "Oteryn Redis" "$(project_ids_for_service redis)"

sleep 20

set_policy_and_start_ids "Oteryn Canary" "$(project_ids_for_service canary)"
set_policy_and_start_ids "Oteryn Platform" "$(project_ids_for_service platform)"
set_policy_and_start_ids "Oteryn internal proxy" "$(project_ids_for_service internal-proxy)"
set_policy_and_start_ids "Oteryn gateway" "$(project_ids_for_service gateway)"

# tls-init is intentionally excluded because it is a one-shot bootstrap service.

runner_ids="$($DOCKER ps -aq --filter 'name=oteryn-synology-staging')"
set_policy_and_start_ids "GitHub Actions runner oteryn-synology-staging" "$runner_ids"

echo "--- Oteryn staging containers ---"
"$DOCKER" ps -a \
    --filter "label=com.docker.compose.project=oteryn-staging" \
    --format 'table {{.Names}}\t{{.Status}}\t{{.Image}}' || true

echo "--- Oteryn runner ---"
"$DOCKER" ps -a \
    --filter 'name=oteryn-synology-staging' \
    --format 'table {{.Names}}\t{{.Status}}\t{{.Image}}' || true

echo "[$(date '+%Y-%m-%d %H:%M:%S')] Oteryn boot repair completed"
