#!/usr/bin/env python3
from __future__ import annotations

import argparse
import getpass
import json
import os
import shutil
import subprocess
from pathlib import Path

from tibia_linux_reference import HarnessError
from tibia_linux_reference.preflight import run_preflight
from tibia_linux_reference.security import read_proc_bytes

CI_KEYS = (
    "CI",
    "GITHUB_ACTIONS",
    "GITLAB_CI",
    "BUILDKITE",
    "CIRCLECI",
    "JENKINS_URL",
    "TF_BUILD",
)
CONTAINER_MARKERS = (Path("/.dockerenv"), Path("/run/.containerenv"))


def virtualization() -> tuple[str, str]:
    detector = shutil.which("systemd-detect-virt")
    if not detector:
        raise HarnessError("systemd-detect-virt is required to prove the execution boundary")
    container = subprocess.run(
        [detector, "--container"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    vm = subprocess.run(
        [detector, "--vm"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    container_kind = container.stdout.strip() if container.returncode == 0 else "none"
    vm_kind = vm.stdout.strip() if vm.returncode == 0 else "none"
    return container_kind or "unknown", vm_kind or "unknown"


def require_normal_host(expected_user: str) -> dict[str, object]:
    active_ci = sorted(key for key in CI_KEYS if os.environ.get(key))
    if active_ci:
        raise HarnessError("official execution is forbidden from CI runner environments")
    if any(marker.exists() for marker in CONTAINER_MARKERS):
        raise HarnessError("official execution is forbidden inside containers")
    proc_version = Path("/proc/version").read_text(encoding="utf-8", errors="replace")
    if "microsoft" in proc_version.lower() or "wsl" in proc_version.lower():
        raise HarnessError("official execution is forbidden inside WSL")
    if not Path("/run/systemd/system").is_dir():
        raise HarnessError("a normal systemd Linux host or VM is required")
    current_user = getpass.getuser()
    if current_user != expected_user:
        raise HarnessError("official execution must use the dedicated task account")

    container_kind, vm_kind = virtualization()
    if container_kind not in {"none", "unknown"}:
        raise HarnessError("systemd reports a container execution boundary")
    gpu_present = Path("/dev/dri").exists() or any(Path("/dev").glob("nvidia[0-9]*"))
    if not gpu_present:
        raise HarnessError("official execution requires a normal Linux graphics device")

    return {
        "dedicated_user": current_user,
        "ci_runner": False,
        "container": False,
        "wsl": False,
        "systemd_host": True,
        "virtual_machine_kind": vm_kind,
        "gpu_device_present": True,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--evidence-dir", type=Path, required=True)
    parser.add_argument("--expected-user", default="oteryn-tibia-ref")
    args = parser.parse_args()

    host = require_normal_host(args.expected_user)
    preflight = run_preflight(
        repo_root=args.repo_root.resolve(strict=True),
        evidence_root=args.evidence_dir,
        mode="official",
        arguments=read_proc_bytes(os.getpid(), "cmdline"),
    )
    preflight.pop("network_prefix", None)
    if preflight.get("boundary") != "linux-host-or-vm":
        raise HarnessError("official execution boundary is not a normal Linux host or VM")
    display = preflight.get("display")
    if not isinstance(display, dict) or not (display.get("x11_present") or display.get("wayland_present")):
        raise HarnessError("official execution requires an interactive graphical session")

    print(
        json.dumps(
            {
                "result": "PASS",
                "host": host,
                "preflight": preflight,
                "official_service_contacted": False,
                "binary_execution_performed": False,
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except HarnessError as error:
        print(json.dumps({"result": "FAIL", "error_class": type(error).__name__}, sort_keys=True))
        raise SystemExit(2) from error
