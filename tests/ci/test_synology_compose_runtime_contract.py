"""Staging Compose runtime contract: no Redis, scheduler always on (Platform review #1560 D-4, D-6)."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SYNOLOGY = ROOT / "deploy" / "synology"
COMPOSE = (SYNOLOGY / "compose.yml").read_text(encoding="utf-8")
MARKETPLACE = (SYNOLOGY / "compose.marketplace.yml").read_text(encoding="utf-8")
SCHEDULER = COMPOSE[COMPOSE.index("\n  marketplace-scheduler:\n") : COMPOSE.index("\nvolumes:\n")]


def test_staging_stack_has_no_redis() -> None:
    assert "\n  redis:" not in COMPOSE
    assert "redis_data" not in COMPOSE
    assert "REDIS_PASSWORD" not in COMPOSE
    # deploy.sh alone keeps one bounded transition that removes the retired container by label.
    scripts = [p for p in sorted((SYNOLOGY / "scripts").glob("*.sh")) if p.name != "deploy.sh"]
    for path in (SYNOLOGY / ".env.example", *scripts):
        assert not re.search(r"redis", path.read_text(encoding="utf-8"), re.IGNORECASE), path.name


def test_app_drivers_are_not_redis() -> None:
    for service in (COMPOSE[COMPOSE.index("\n  platform:\n") : COMPOSE.index("\n  tls-init:\n")], SCHEDULER):
        assert "SESSION_DRIVER: file\n" in service
        assert "CACHE_STORE: file\n" in service
        assert "QUEUE_CONNECTION: sync\n" in service
        assert "redis" not in service.lower()


def test_scheduler_is_an_always_on_base_service() -> None:
    assert "    image: ${PLATFORM_IMAGE}\n" in SCHEDULER
    assert "    restart: always\n" in SCHEDULER
    assert "      - schedule:work\n" in SCHEDULER
    assert "    healthcheck:\n" in SCHEDULER
    assert "    ports:" not in SCHEDULER
    assert "      mariadb:\n        condition: service_healthy\n" in SCHEDULER


def test_marketplace_overlay_no_longer_defines_the_scheduler_process() -> None:
    overlay = MARKETPLACE[MARKETPLACE.index("\n  marketplace-scheduler:\n") :]
    for owned_by_base in ("image:", "command:", "restart:", "healthcheck:", "volumes:", "depends_on:"):
        assert owned_by_base not in overlay
    assert "MARKETPLACE_ENABLED: ${MARKETPLACE_ENABLED:-false}" in overlay


def test_deploy_scripts_keep_the_scheduler_running_across_reconciliation() -> None:
    lib = (SYNOLOGY / "scripts" / "lib.sh").read_text(encoding="utf-8")
    assert "_oteryn_recreate_base_scheduler()" in lib
    assert "marketplace-scheduler" in (SYNOLOGY / "scripts" / "health-check.sh").read_text(encoding="utf-8")
    deploy = (SYNOLOGY / "scripts" / "deploy.sh").read_text(encoding="utf-8")
    gateway = deploy.index('"${compose[@]}" up -d gateway\n')
    reconcile = deploy.index("_oteryn_reconcile_marketplace_scheduler_after_runtime_change\n\nOTERYN_ENV_FILE")
    assert gateway < reconcile < deploy.index('OTERYN_ENV_FILE="$ENV_FILE" bash "$SCRIPT_DIR/health-check.sh"')


def test_deploy_removes_the_retired_redis_container_without_an_orphan_sweep() -> None:
    deploy = (SYNOLOGY / "scripts" / "deploy.sh").read_text(encoding="utf-8")
    removal = deploy.index("label=com.docker.compose.service=redis")
    assert removal < deploy.index('"${compose[@]}" up -d mariadb\n')
    assert "--remove-orphans" not in deploy


def test_candidate_finalization_creates_the_scheduler_before_the_health_contract() -> None:
    deploy = (SYNOLOGY / "scripts" / "deploy.sh").read_text(encoding="utf-8")
    start = deploy.index("finalize_previous_candidate_if_healthy()")
    body = deploy[start:]
    gateway = body.index("up -d --no-deps --force-recreate internal-proxy gateway")
    scheduler = body.index('"${candidate_compose[@]}" up -d --no-deps --force-recreate marketplace-scheduler')
    health = body.index('OTERYN_ENV_FILE="$candidate_env" bash "$SCRIPT_DIR/health-check.sh"')
    assert gateway < scheduler < health
