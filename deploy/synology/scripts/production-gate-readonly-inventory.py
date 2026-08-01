#!/usr/bin/env python3
"""Collect a sanitized, read-only inventory from the Synology Docker host."""

from __future__ import annotations

import datetime
import ipaddress
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
from typing import Any

PROJECT = os.environ.get("OTERYN_COMPOSE_PROJECT_NAME", "oteryn-staging")
EVIDENCE_PATH = Path(
    os.environ.get(
        "OTERYN_PRODUCTION_GATE_EVIDENCE_PATH",
        "/tmp/production-gate-synology-readonly.json",
    )
)
OBSERVER_SOURCE_SHA = os.environ.get("OBSERVER_SOURCE_SHA", "UNKNOWN")
SERVICES = ("mariadb", "redis", "canary", "platform", "internal-proxy", "gateway")


def command(
    arguments: list[str],
    *,
    check: bool = True,
    stdout: Any = subprocess.PIPE,
    stderr: Any = subprocess.PIPE,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        arguments,
        check=check,
        stdout=stdout,
        stderr=stderr,
        text=True,
    )


def output(arguments: list[str]) -> str:
    return command(arguments).stdout.strip()


def inspect(container_id: str, template: str) -> str:
    return output(["docker", "inspect", "--format", template, container_id])


def quiet_success(arguments: list[str]) -> bool:
    return (
        command(
            arguments,
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        ).returncode
        == 0
    )


def classify_binding(binding: dict[str, str]) -> str:
    host = binding.get("HostIp", "")
    port = binding.get("HostPort", "")
    if not host or host in {"0.0.0.0", "::"}:
        scope = "wildcard"
    else:
        try:
            address = ipaddress.ip_address(host)
        except ValueError:
            scope = "invalid"
        else:
            if address.is_loopback:
                scope = "loopback"
            elif address.is_private:
                scope = "private-ip"
            else:
                scope = "public-ip"
    return f"{scope}:{port}"


def published_ports(container_id: str) -> dict[str, list[str]]:
    parsed = json.loads(
        inspect(container_id, "{{json .NetworkSettings.Ports}}") or "{}"
    )
    return {
        container_port: [classify_binding(item) for item in bindings]
        for container_port, bindings in parsed.items()
        if bindings
    }


def image_digests(image_ref: str) -> list[str]:
    result = command(
        ["docker", "image", "inspect", "--format", "{{json .RepoDigests}}", image_ref],
        check=False,
    )
    if result.returncode != 0:
        return []
    try:
        return json.loads(result.stdout.strip() or "[]") or []
    except json.JSONDecodeError:
        return []


def bounded_log_marker_count(container_id: str) -> int:
    result = command(
        ["docker", "logs", "--since", "30m", container_id],
        check=False,
    )
    combined = (result.stdout or "") + "\n" + (result.stderr or "")
    return len(
        re.findall(
            r"(?i)\b(?:fatal|panic|critical|exception|error)\b",
            combined,
        )
    )


def registered_tunnel_connection_count(container_id: str) -> int:
    result = command(
        ["docker", "logs", "--since", "30m", container_id],
        check=False,
    )
    combined = (result.stdout or "") + "\n" + (result.stderr or "")
    return len(
        re.findall(
            r"(?i)(?:registered tunnel connection|connection .* registered)",
            combined,
        )
    )


def container_ids_for_service(service: str, *, running_only: bool = False) -> list[str]:
    arguments = [
        "docker",
        "ps",
        "-q" if running_only else "-aq",
        "--filter",
        f"label=com.docker.compose.project={PROJECT}",
        "--filter",
        f"label=com.docker.compose.service={service}",
    ]
    return [line for line in output(arguments).splitlines() if line]


def collect_services() -> dict[str, dict[str, Any]]:
    inventory: dict[str, dict[str, Any]] = {}
    for service in SERVICES:
        ids = container_ids_for_service(service)
        entry: dict[str, Any] = {"container_count": len(ids)}
        if len(ids) == 1:
            container_id = ids[0]
            image_ref = inspect(container_id, "{{.Config.Image}}")
            networks = [
                item
                for item in inspect(
                    container_id,
                    "{{range $name, $_ := .NetworkSettings.Networks}}"
                    "{{println $name}}{{end}}",
                ).splitlines()
                if item
            ]
            config_files_raw = inspect(
                container_id,
                '{{index .Config.Labels "com.docker.compose.project.config_files"}}',
            )
            config_files = [
                PurePosixPath(item.strip()).name
                for item in config_files_raw.split(",")
                if item.strip() and item.strip() != "<no value>"
            ]
            entry.update(
                {
                    "container_name": inspect(container_id, "{{.Name}}").lstrip("/"),
                    "running": inspect(container_id, "{{.State.Running}}") == "true",
                    "status": inspect(container_id, "{{.State.Status}}"),
                    "health": inspect(
                        container_id,
                        "{{if .State.Health}}{{.State.Health.Status}}"
                        "{{else}}not-configured{{end}}",
                    ),
                    "restart_count": int(
                        inspect(container_id, "{{.RestartCount}}")
                    ),
                    "restart_policy": inspect(
                        container_id,
                        "{{.HostConfig.RestartPolicy.Name}}",
                    ),
                    "started_at_utc": inspect(
                        container_id,
                        "{{.State.StartedAt}}",
                    ),
                    "image_ref": image_ref,
                    "image_id": inspect(container_id, "{{.Image}}"),
                    "image_repo_digests": image_digests(image_ref),
                    "network_mode": inspect(
                        container_id,
                        "{{.HostConfig.NetworkMode}}",
                    ),
                    "networks": sorted(networks),
                    "published_ports": published_ports(container_id),
                    "compose_config_files": sorted(config_files),
                    "critical_log_markers_last_30m": bounded_log_marker_count(
                        container_id
                    ),
                }
            )
        inventory[service] = entry
    return inventory


def collect_effective_profile(
    platform_ids: list[str],
) -> dict[str, bool | int | None]:
    profile: dict[str, bool | int | None] = {
        "app_env_production": False,
        "app_env_staging": False,
        "app_debug_disabled": False,
        "session_driver_file": False,
        "cache_store_file": False,
        "queue_connection_sync": False,
        "mail_transport_array": False,
        "production_verify_configuration_exit_code": None,
    }
    if len(platform_ids) != 1:
        return profile

    platform_id = platform_ids[0]
    tests = {
        "app_env_production": 'test "$APP_ENV" = production',
        "app_env_staging": 'test "$APP_ENV" = staging',
        "app_debug_disabled": 'test "$APP_DEBUG" = false',
        "session_driver_file": 'test "$SESSION_DRIVER" = file',
        "cache_store_file": 'test "$CACHE_STORE" = file',
        "queue_connection_sync": 'test "$QUEUE_CONNECTION" = sync',
        "mail_transport_array": 'test "$MAIL_MAILER" = array',
    }
    for key, shell_test in tests.items():
        profile[key] = quiet_success(
            ["docker", "exec", platform_id, "sh", "-ec", shell_test]
        )

    profile["production_verify_configuration_exit_code"] = command(
        [
            "docker",
            "exec",
            platform_id,
            "php",
            "artisan",
            "production:verify-configuration",
        ],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    ).returncode
    return profile


def wget_probe(
    *,
    network: str,
    url: str,
    host_header: str,
) -> bool:
    return quiet_success(
        [
            "docker",
            "run",
            "--rm",
            "--pull",
            "never",
            "--network",
            network,
            "alpine:3.22",
            "wget",
            "-qO-",
            "-T",
            "5",
            f"--header=Host: {host_header}",
            url,
        ]
    )


def collect_probes(
    platform_ids: list[str],
    gateway_ids: list[str],
) -> dict[str, bool]:
    probes = {
        "platform_container_health": False,
        "platform_host_loopback_health": False,
        "gateway_container_health": False,
        "gateway_container_ready": False,
        "gateway_container_version": False,
        "gateway_host_loopback_health": False,
        "gateway_host_loopback_ready": False,
        "gateway_host_loopback_version": False,
    }
    if len(platform_ids) == 1:
        probes["platform_container_health"] = wget_probe(
            network=f"container:{platform_ids[0]}",
            url="http://127.0.0.1:8000/health",
            host_header="oteryn.molehill.cloud",
        )
    if len(gateway_ids) == 1:
        for key, path in (
            ("gateway_container_health", "/health"),
            ("gateway_container_ready", "/ready"),
            ("gateway_container_version", "/version"),
        ):
            probes[key] = wget_probe(
                network=f"container:{gateway_ids[0]}",
                url=f"http://127.0.0.1:8080{path}",
                host_header="login.oteryn.molehill.cloud",
            )

    for key, port, path, host_header in (
        (
            "platform_host_loopback_health",
            8000,
            "/health",
            "oteryn.molehill.cloud",
        ),
        (
            "gateway_host_loopback_health",
            8080,
            "/health",
            "login.oteryn.molehill.cloud",
        ),
        (
            "gateway_host_loopback_ready",
            8080,
            "/ready",
            "login.oteryn.molehill.cloud",
        ),
        (
            "gateway_host_loopback_version",
            8080,
            "/version",
            "login.oteryn.molehill.cloud",
        ),
    ):
        probes[key] = wget_probe(
            network="host",
            url=f"http://127.0.0.1:{port}{path}",
            host_header=host_header,
        )
    return probes


def collect_cloudflared() -> list[dict[str, Any]]:
    candidates: list[dict[str, Any]] = []
    all_ids = [line for line in output(["docker", "ps", "-aq"]).splitlines() if line]
    for container_id in all_ids:
        name = inspect(container_id, "{{.Name}}").lstrip("/")
        image_ref = inspect(container_id, "{{.Config.Image}}")
        if "cloudflared" not in name.lower() and "cloudflared" not in image_ref.lower():
            continue
        networks = [
            item
            for item in inspect(
                container_id,
                "{{range $name, $_ := .NetworkSettings.Networks}}"
                "{{println $name}}{{end}}",
            ).splitlines()
            if item
        ]
        candidates.append(
            {
                "container_name": name,
                "image_ref": image_ref,
                "image_id": inspect(container_id, "{{.Image}}"),
                "running": inspect(container_id, "{{.State.Running}}") == "true",
                "status": inspect(container_id, "{{.State.Status}}"),
                "restart_count": int(
                    inspect(container_id, "{{.RestartCount}}")
                ),
                "restart_policy": inspect(
                    container_id,
                    "{{.HostConfig.RestartPolicy.Name}}",
                ),
                "network_mode": inspect(
                    container_id,
                    "{{.HostConfig.NetworkMode}}",
                ),
                "networks": sorted(networks),
                "published_ports": published_ports(container_id),
                "registered_connections_last_30m": (
                    registered_tunnel_connection_count(container_id)
                ),
            }
        )
    return candidates


def assert_secret_safe(evidence: dict[str, Any]) -> None:
    serialized = json.dumps(evidence, sort_keys=True, separators=(",", ":"))
    forbidden_patterns = (
        r"(?i)authorization",
        r"(?i)bearer",
        r"(?i)connection[_-]?string",
        r"(?i)cookie",
        r"(?i)totp",
        r"(?i)recovery[_-]?code",
    )
    for pattern in forbidden_patterns:
        if re.search(pattern, serialized):
            raise SystemExit(f"forbidden evidence pattern: {pattern}")


def main() -> int:
    command(["docker", "version"], stdout=subprocess.DEVNULL)
    inventory = collect_services()
    platform = inventory["platform"]
    gateway = inventory["gateway"]
    canary = inventory["canary"]

    platform_match = re.fullmatch(
        r"ghcr\.io/blakinio/oteryn-platform:sha-([0-9a-f]{40})",
        str(platform.get("image_ref", "")),
    )
    gateway_match = re.fullmatch(
        r"ghcr\.io/blakinio/oteryn-game-gateway:sha-([0-9a-f]{40})",
        str(gateway.get("image_ref", "")),
    )
    platform_release = platform_match.group(1) if platform_match else "UNKNOWN"
    gateway_release = gateway_match.group(1) if gateway_match else "UNKNOWN"

    platform_ids = container_ids_for_service("platform", running_only=True)
    gateway_ids = container_ids_for_service("gateway", running_only=True)
    profile = collect_effective_profile(platform_ids)
    probes = collect_probes(platform_ids, gateway_ids)

    runtime_classification = "UNKNOWN"
    if profile["app_env_staging"] and not profile["app_env_production"]:
        runtime_classification = "STAGING_TARGET"
    elif profile["app_env_production"]:
        runtime_classification = "PRODUCTION_TARGET"

    evidence = {
        "schema_version": 1,
        "observed_at_utc": datetime.datetime.now(
            datetime.timezone.utc
        ).isoformat(),
        "observer_source_sha": OBSERVER_SOURCE_SHA,
        "target": "synology-self-hosted-runner",
        "compose_project": PROJECT,
        "runtime_classification": runtime_classification,
        "production_environment_proven": (
            runtime_classification == "PRODUCTION_TARGET"
        ),
        "deployed_release_sha": platform_release,
        "gateway_release_sha": gateway_release,
        "platform_gateway_release_match": (
            platform_release != "UNKNOWN"
            and platform_release == gateway_release
        ),
        "canary_image_digest_ref": canary.get("image_ref", "UNKNOWN"),
        "services": inventory,
        "effective_application_profile": profile,
        "health_probes": probes,
        "cloudflared_containers": collect_cloudflared(),
        "cloudflared_host_process_state": "UNKNOWN",
        "cloudflared_ingress_state": "PROVEN_SEPARATELY_BY_RUN_30700054602",
        "database_restore_drill": "NOT_RUN",
        "production_mutation": "NONE",
    }
    assert_secret_safe(evidence)

    EVIDENCE_PATH.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE_PATH.write_text(
        json.dumps(evidence, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    EVIDENCE_PATH.chmod(0o600)

    exact_release = re.fullmatch(r"[0-9a-f]{40}", platform_release)
    singletons_running = all(
        inventory[name]["container_count"] == 1
        and inventory[name].get("running", False)
        for name in SERVICES
    )
    probes_pass = all(probes.values())
    if not exact_release:
        print("production-gate inventory failure: exact Platform release unknown", file=sys.stderr)
        return 21
    if not evidence["platform_gateway_release_match"]:
        print("production-gate inventory failure: Platform/Gateway release mismatch", file=sys.stderr)
        return 22
    if not singletons_running:
        print("production-gate inventory failure: service singleton/running invariant", file=sys.stderr)
        return 23
    if not probes_pass:
        print("production-gate inventory failure: bounded local health probe", file=sys.stderr)
        return 24

    print(
        "PRODUCTION_GATE_SYNOLOGY_READONLY "
        f"release={platform_release} "
        f"runtime={runtime_classification} "
        "health=PASS mutation=NONE"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
