from __future__ import annotations

import json
import os
import platform
import shutil
import subprocess
from pathlib import Path
from typing import Mapping

from . import HarnessError
from .security import (
    ensure_no_injection_environment,
    high_confidence_secret_present,
    require_private_directory,
    validate_no_secret_like_environment,
    validate_no_shell_trace,
)


REQUIRED_COMMANDS = ("git", "readelf", "sha256sum", "findmnt", "lsblk", "unshare")
ENCRYPTED_FILESYSTEM_TYPES = {"ecryptfs", "encfs", "gocryptfs", "cryfs"}


def _parent_cmdline() -> bytes:
    try:
        return (Path("/proc") / str(os.getppid()) / "cmdline").read_bytes()
    except (FileNotFoundError, PermissionError):
        return b""


def _block_device_type(source: str) -> str:
    if not source.startswith("/dev/"):
        return "unknown"
    completed = subprocess.run(
        ["lsblk", "-ndo", "TYPE", source],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if completed.returncode != 0:
        return "unknown"
    kinds = [line.strip() for line in completed.stdout.splitlines() if line.strip()]
    return kinds[0] if len(kinds) == 1 else "unknown"


def _storage_details(path: Path) -> dict[str, object]:
    completed = subprocess.run(
        ["findmnt", "-J", "-T", str(path)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    fstype = "unknown"
    source = "unknown"
    if completed.returncode == 0:
        try:
            filesystems = json.loads(completed.stdout).get("filesystems", [])
            if filesystems:
                fstype = str(filesystems[0].get("fstype", "unknown"))
                source = str(filesystems[0].get("source", "unknown"))
        except json.JSONDecodeError:
            pass

    device_type = _block_device_type(source)
    encrypted = fstype in ENCRYPTED_FILESYSTEM_TYPES or device_type == "crypt"
    if source.startswith("/dev/mapper/"):
        source_class = "device-mapper"
    elif source.startswith("/dev/"):
        source_class = "block-device"
    else:
        source_class = "virtual-or-unknown"
    return {
        "filesystem_type": fstype,
        "source_class": source_class,
        "block_device_type": device_type,
        "encryption_proven": encrypted,
    }


def _network_isolator() -> dict[str, object]:
    direct = subprocess.run(
        ["unshare", "-Urn", "true"],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    if direct.returncode == 0:
        return {"kind": "unprivileged-user-network-namespace", "prefix": ["unshare", "-Urn", "--"]}

    sudo = shutil.which("sudo")
    if sudo:
        uid = os.getuid()
        gid = os.getgid()
        # The process environment reaching this point has already passed fail-closed validation;
        # the launcher then supplies a smaller explicit child allowlist. Preserve that allowlist
        # across namespace creation so local X11/HOME/TMPDIR survive the uid/gid drop.
        prefix = [sudo, "-n", "-E", "unshare", "-n", "--setuid", str(uid), "--setgid", str(gid), "--"]
        probe = subprocess.run(
            [*prefix, "true"],
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        if probe.returncode == 0:
            return {"kind": "privileged-namespace-setup-with-uid-drop", "prefix": prefix}
    raise HarnessError("no fail-closed network namespace control is available")


def run_preflight(
    *,
    repo_root: Path,
    evidence_root: Path,
    mode: str,
    environment: Mapping[str, str] | None = None,
    arguments: bytes | None = None,
) -> dict[str, object]:
    environment = dict(os.environ if environment is None else environment)
    if platform.system() != "Linux" or platform.machine() not in {"x86_64", "amd64"}:
        raise HarnessError("the harness requires Linux x86-64")
    if os.geteuid() == 0:
        raise HarnessError("the harness must run as a dedicated non-privileged user")
    missing = [command for command in REQUIRED_COMMANDS if shutil.which(command) is None]
    if missing:
        raise HarnessError("required cleanup or identity controls are unavailable: " + ", ".join(missing))
    if not Path("/proc/self/fd").is_dir() or not hasattr(os, "killpg"):
        raise HarnessError("required process and descriptor cleanup controls are unavailable")
    if not environment.get("DISPLAY") and not environment.get("WAYLAND_DISPLAY"):
        raise HarnessError("a normal graphical display session is required")
    validate_no_shell_trace(environment, _parent_cmdline())
    validate_no_secret_like_environment(environment)
    ensure_no_injection_environment(environment)
    if arguments and high_confidence_secret_present(arguments):
        raise HarnessError("process arguments contain secret-like material")

    require_private_directory(evidence_root, repo_root)
    storage = _storage_details(evidence_root)
    if mode == "official" and storage["encryption_proven"] is not True:
        raise HarnessError("official mode requires a provably encrypted private evidence volume")

    isolator = _network_isolator()
    os_release = "unknown"
    release_path = Path("/etc/os-release")
    if release_path.is_file():
        for line in release_path.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.startswith("PRETTY_NAME="):
                os_release = line.split("=", 1)[1].strip('"')
                break
    proc_version = Path("/proc/version").read_text(encoding="utf-8", errors="replace")
    return {
        "mode": mode,
        "os": os_release,
        "architecture": platform.machine(),
        "kernel": platform.release(),
        "boundary": "wsl2-vm" if "microsoft" in proc_version.lower() else "linux-host-or-vm",
        "display": {
            "x11_present": bool(environment.get("DISPLAY")),
            "wayland_present": bool(environment.get("WAYLAND_DISPLAY")),
            "gpu_device_present": Path("/dev/dxg").exists() or Path("/dev/dri").exists(),
        },
        "user": {"non_privileged": True, "dedicated_user_required_for_official": True},
        "storage": storage,
        "evidence_path_classification": "private-outside-git",
        "shell_trace_disabled": True,
        "ordinary_environment_secret_scan": "PASS",
        "injection_controls": {"ld_preload_absent": True, "ld_audit_absent": True},
        "network_isolator": isolator["kind"],
        "network_prefix": isolator["prefix"],
    }
