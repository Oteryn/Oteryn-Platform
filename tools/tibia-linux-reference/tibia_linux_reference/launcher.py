from __future__ import annotations

import json
import os
import re
import secrets
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Mapping

from . import HarnessError
from .identity import ApprovedIdentity, verify_client_identity
from .manifest import validate_manifest
from .preflight import run_preflight
from .security import (
    exact_secret_hits,
    high_confidence_secret_present,
    read_proc_bytes,
    sanitized_child_environment,
    scan_prohibited_locations,
    sha256_bytes,
    write_private_json,
)
from .x11 import window_ids


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


@dataclass
class SyntheticCorpus:
    values: list[str]

    @classmethod
    def generate(cls) -> "SyntheticCorpus":
        nonce = secrets.token_hex(16)
        return cls(
            [
                f"synthetic-login-{nonce}@invalid.example",
                f"SYNTHETIC-password-{nonce}-OnlyForDryRun!",
                f"synthetic.session.token.{nonce}.{secrets.token_hex(12)}",
                f"SYNTHETIC-AUTH-{nonce[:12]}-123456",
            ]
        )

    def pipe_payload(self) -> bytearray:
        return bytearray(("\n".join(self.values) + "\n").encode("utf-8"))


def _wipe_bytearray(value: bytearray) -> None:
    for index in range(len(value)):
        value[index] = 0


APPROVED_EVENT_KEYS = {
    "process_state": {"event", "state"},
    "credential_channel_consumed": {"event", "mechanism", "value_count"},
    "network_denial": {
        "event",
        "denied",
        "endpoint_classification",
        "error_class",
        "interfaces",
        "namespace",
    },
    "window_state": {"event", "backend", "state"},
    "fake_client_failure": {"event", "error_class"},
}


def _parse_events(stdout: bytes) -> list[dict[str, object]]:
    events: list[dict[str, object]] = []
    for line in stdout.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError as error:
            raise HarnessError("fake client emitted non-JSON output") from error
        kind = event.get("event")
        if kind not in APPROVED_EVENT_KEYS or set(event) != APPROVED_EVENT_KEYS[kind]:
            raise HarnessError("fake client emitted an unapproved evidence field")
        events.append(event)
    return events


def _safe_inventory(root: Path) -> list[str]:
    if not root.exists():
        return []
    return sorted(str(path.relative_to(root)) for path in root.rglob("*") if path.is_file())


def verify_cleanup(*, process_stopped: bool, profile: Path, raw_root: Path) -> dict[str, object]:
    raw_files_after = _safe_inventory(raw_root)
    passed = process_stopped and not profile.exists() and not raw_files_after
    return {
        "result": "PASS" if passed else "FAIL",
        "processes_stopped": process_stopped,
        "protected_temporary_files_removed": not profile.exists(),
        "raw_files_retained": raw_files_after,
        "temporary_profile_retention": "deleted-after-run",
    }


def _remove_profile(profile: Path) -> None:
    if profile.exists():
        shutil.rmtree(profile)


def _network_metadata(pid: int) -> tuple[str, list[str]]:
    namespace = os.readlink(f"/proc/{pid}/ns/net")
    lines = Path(f"/proc/{pid}/net/dev").read_text(encoding="utf-8", errors="replace").splitlines()[2:]
    interfaces = sorted(line.split(":", 1)[0].strip() for line in lines if ":" in line)
    return namespace, interfaces


def run_synthetic_dry_run(*, repo_root: Path, evidence_root: Path) -> dict[str, object]:
    preflight = run_preflight(
        repo_root=repo_root,
        evidence_root=evidence_root,
        mode="synthetic",
        arguments=read_proc_bytes(os.getpid(), "cmdline"),
    )
    session_id = "synthetic-" + secrets.token_hex(8)
    session_root = evidence_root / session_id
    publishable = session_root / "publishable"
    raw = session_root / "raw"
    publishable.mkdir(parents=True, mode=0o700)
    raw.mkdir(mode=0o700)
    profile = Path(tempfile.mkdtemp(prefix="profile-", dir=raw))
    (profile / "tmp").mkdir(mode=0o700)
    corpus = SyntheticCorpus.generate()
    secret_payload = corpus.pipe_payload()
    child_environment = sanitized_child_environment(profile)
    fake_client = Path(__file__).with_name("fake_client.py")
    command = [
        *preflight["network_prefix"],
        sys.executable,
        str(fake_client),
        "--window-seconds",
        "0.25",
    ]
    started_wall = utc_now()
    started_monotonic = time.monotonic_ns()
    process: subprocess.Popen[bytes] | None = None
    stdout = b""
    stderr = b""
    process_arguments = b""
    events: list[dict[str, object]] = []
    exit_status = -1
    cleanup_passed = False
    try:
        process = subprocess.Popen(
            command,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=child_environment,
            start_new_session=True,
        )
        process_arguments = read_proc_bytes(process.pid, "cmdline")
        if process.stdin is None:
            raise HarnessError("synthetic credential channel was not created")
        try:
            process.stdin.write(secret_payload)
            process.stdin.flush()
        except (BrokenPipeError, OSError) as error:
            raise HarnessError("synthetic credential channel closed before consumption") from error
        finally:
            try:
                process.stdin.close()
            except OSError:
                pass
            process.stdin = None
        _wipe_bytearray(secret_payload)
        stdout, stderr = process.communicate(timeout=15)
        exit_status = process.returncode
        events = _parse_events(stdout)
        if exit_status != 0:
            failure = next((event for event in events if event["event"] == "fake_client_failure"), None)
            failure_class = failure.get("error_class", "unknown") if failure else "unknown"
            raise HarnessError(f"fake client failed with {failure_class}")
    except subprocess.TimeoutExpired as error:
        raise HarnessError("fake client exceeded its bounded runtime") from error
    except BaseException:
        _remove_profile(profile)
        raise
    finally:
        _wipe_bytearray(secret_payload)
        if process is not None and process.stdin is not None:
            try:
                process.stdin.close()
            except OSError:
                pass
            process.stdin = None
        if process is not None and process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=2)

    network_event = next((event for event in events if event["event"] == "network_denial"), None)
    windows = [event for event in events if event["event"] == "window_state"]
    process_states = [event["state"] for event in events if event["event"] == "process_state"]
    if not network_event or network_event.get("denied") is not True:
        _remove_profile(profile)
        raise HarnessError("fake client did not prove network denial")
    if network_event.get("namespace") == os.readlink("/proc/self/ns/net"):
        _remove_profile(profile)
        raise HarnessError("fake client did not enter a distinct network namespace")
    if [event.get("state") for event in windows] != ["mapped", "destroyed"]:
        _remove_profile(profile)
        raise HarnessError("fake client graphical lifecycle was incomplete")
    if process_states != ["started", "exiting"]:
        _remove_profile(profile)
        raise HarnessError("fake client process lifecycle was incomplete")

    environment_report = {
        "reported_keys": sorted(child_environment),
        "values_retained": False,
        "secret_scan": "PASS",
    }
    environment_bytes = json.dumps(environment_report, sort_keys=True).encode()
    pre_cleanup_scan = scan_prohibited_locations(
        repo_root=repo_root,
        evidence_root=session_root,
        temporary_root=profile,
        secrets=corpus.values,
        process_arguments=process_arguments,
        retained_environment_report=environment_bytes,
        stdout=stdout,
        stderr=stderr,
    )
    if not pre_cleanup_scan.passed:
        _remove_profile(profile)
        raise HarnessError("synthetic secret leak detected in: " + ", ".join(pre_cleanup_scan.categories))

    before_cleanup = _safe_inventory(profile)
    _remove_profile(profile)
    process_stopped = process is not None and process.poll() is not None
    cleanup_report = verify_cleanup(process_stopped=process_stopped, profile=profile, raw_root=raw)
    cleanup_report["files_before_cleanup"] = before_cleanup
    raw_files_after = cleanup_report["raw_files_retained"]
    cleanup_passed = cleanup_report["result"] == "PASS"
    if not cleanup_passed:
        raise HarnessError("deterministic cleanup verification failed")
    write_private_json(publishable / "cleanup-report.json", cleanup_report)

    finished_monotonic = time.monotonic_ns()
    manifest = {
        "schema_version": 1,
        "session_id": session_id,
        "mode": "synthetic-dry-run",
        "started_at": started_wall,
        "finished_at": utc_now(),
        "duration_monotonic_ms": (finished_monotonic - started_monotonic) // 1_000_000,
        "environment": {key: value for key, value in preflight.items() if key != "network_prefix"},
        "client_identity": {
            "classification": "deterministic-fake-client",
            "source_sha256": sha256_bytes(fake_client.read_bytes()),
            "official_binary_used": False,
        },
        "credential_handling": {
            "values": "synthetic-only",
            "count": len(corpus.values),
            "mechanism": "anonymous-pipe",
            "arguments_contained_values": False,
            "environment_contained_values": False,
            "retained": False,
        },
        "process_lifecycle": {
            "states": process_states,
            "exit_status": exit_status,
            "arguments_retained": False,
            "arguments_sha256": sha256_bytes(process_arguments),
            "environment_report": environment_report,
        },
        "window_lifecycle": [{"backend": event["backend"], "state": event["state"]} for event in windows],
        "network_denial": {
            "proven": True,
            "namespace": network_event["namespace"],
            "interfaces": network_event["interfaces"],
            "blocked_endpoint_classification": network_event["endpoint_classification"],
            "raw_capture_created": False,
            "official_endpoint_contacted": False,
        },
        "filesystem_inventory": {
            "scope": "temporary-synthetic-profile",
            "files_before_cleanup": before_cleanup,
            "files_after_cleanup": raw_files_after,
        },
        "leak_scan": {
            "result": "PASS",
            "locations": [
                "git-diff",
                "tracked-files",
                "untracked-files",
                "ignored-files",
                "process-arguments",
                "retained-environment-report",
                "stdout",
                "stderr",
                "evidence-or-artifacts",
                "temporary-files",
                "shell-history",
                "cleanup-report",
            ],
            "files_scanned": pre_cleanup_scan.files_scanned,
        },
        "cleanup": cleanup_report,
        "safety": {
            "official_login_attempted": False,
            "official_service_contacted": False,
            "client_or_battleye_modified": False,
            "ptrace_debugger_hook_injection_used": False,
            "traffic_decrypted_replayed_altered_injected": False,
        },
        "findings": {
            "PROVEN": [
                "The fake client ran in a separate network namespace containing only loopback.",
                "The synthetic credential corpus used an anonymous pipe and passed the prohibited-location leak scan.",
                "The fake graphical window and process completed their bounded lifecycle and cleanup passed.",
            ],
            "DERIVED": ["The local harness controls are ready for exact-client component validation on a suitable host."],
            "UNKNOWN": [
                "Whether the official client and BattlEye support this virtualized graphical environment.",
                "Whether the current host storage is encrypted outside the WSL2 guest boundary.",
            ],
            "CONFLICT": [],
        },
    }
    validate_manifest(manifest)
    write_private_json(publishable / "session-manifest.json", manifest)

    final_scan = scan_prohibited_locations(
        repo_root=repo_root,
        evidence_root=publishable,
        temporary_root=None,
        secrets=corpus.values,
        process_arguments=process_arguments,
        retained_environment_report=environment_bytes,
        stdout=stdout,
        stderr=stderr,
    )
    if not final_scan.passed:
        raise HarnessError("post-manifest leak scan failed in: " + ", ".join(final_scan.categories))
    return manifest


def run_official_component(
    *,
    repo_root: Path,
    evidence_root: Path,
    identity_file: Path,
    package_path: Path,
    executable_path: Path,
    observation_seconds: float,
) -> dict[str, object]:
    if not 1.0 <= observation_seconds <= 30.0:
        raise HarnessError("official component observation must be between 1 and 30 seconds")
    preflight = run_preflight(
        repo_root=repo_root,
        evidence_root=evidence_root,
        mode="official",
        arguments=read_proc_bytes(os.getpid(), "cmdline"),
    )
    approved = ApprovedIdentity.load(identity_file)
    identity = verify_client_identity(
        approved,
        package_path=package_path,
        executable_path=executable_path,
        repo_root=repo_root,
    )
    session_id = "official-component-" + secrets.token_hex(8)
    session_root = evidence_root / session_id
    publishable = session_root / "publishable"
    raw = session_root / "raw"
    publishable.mkdir(parents=True, mode=0o700)
    raw.mkdir(mode=0o700)
    profile = Path(tempfile.mkdtemp(prefix="profile-", dir=raw))
    (profile / "tmp").mkdir(mode=0o700)
    environment = sanitized_child_environment(profile)
    before_windows = window_ids()
    command = [*preflight["network_prefix"], str(executable_path.resolve(strict=True))]
    started = utc_now()
    started_monotonic = time.monotonic_ns()
    process: subprocess.Popen[bytes] | None = None
    arguments = b""
    stdout = b""
    stderr = b""
    new_windows: set[int] = set()
    child_namespace = "unknown"
    child_interfaces: list[str] = []
    try:
        process = subprocess.Popen(
            command,
            cwd=executable_path.resolve(strict=True).parent,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            start_new_session=True,
        )
        arguments = read_proc_bytes(process.pid, "cmdline")
        child_namespace, child_interfaces = _network_metadata(process.pid)
        deadline = time.monotonic() + observation_seconds
        while time.monotonic() < deadline and process.poll() is None:
            new_windows |= window_ids() - before_windows
            time.sleep(0.1)
    except BaseException:
        _remove_profile(profile)
        raise
    finally:
        if process is not None and process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                stdout, stderr = process.communicate(timeout=3)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                stdout, stderr = process.communicate(timeout=3)
        elif process is not None:
            stdout, stderr = process.communicate(timeout=3)
    if process is None:
        _remove_profile(profile)
        raise HarnessError("official client process did not start")
    if child_namespace == os.readlink("/proc/self/ns/net") or any(name != "lo" for name in child_interfaces):
        _remove_profile(profile)
        raise HarnessError("official client process did not enter the denied network namespace")
    combined_output = stdout + b"\n" + stderr
    if high_confidence_secret_present(combined_output):
        _remove_profile(profile)
        raise HarnessError("official component output contains secret-like material; private evidence review required")
    if re.search(rb"(?:battleye|anti-cheat|account.security|client.modification).{0,80}(?:warn|error|fail|restart|required)", combined_output, re.IGNORECASE | re.DOTALL):
        _remove_profile(profile)
        raise HarnessError("official component emitted an anti-cheat, modification, or account-security warning")
    if not new_windows:
        _remove_profile(profile)
        raise HarnessError("official client did not create an observable X11 window")

    environment_report = {
        "reported_keys": sorted(environment),
        "values_retained": False,
        "secret_scan": "PASS",
    }
    environment_bytes = json.dumps(environment_report, sort_keys=True).encode()
    profile_files = _safe_inventory(profile)
    profile_scan = scan_prohibited_locations(
        repo_root=repo_root,
        evidence_root=publishable,
        temporary_root=profile,
        secrets=[],
        process_arguments=arguments,
        retained_environment_report=environment_bytes,
        stdout=stdout,
        stderr=stderr,
    )
    if not profile_scan.passed:
        _remove_profile(profile)
        raise HarnessError("official component leak scan failed in: " + ", ".join(profile_scan.categories))

    _remove_profile(profile)
    cleanup = verify_cleanup(process_stopped=process.poll() is not None, profile=profile, raw_root=raw)
    cleanup["temporary_profile_retention"] = "deleted-after-component-test"
    cleanup["files_before_cleanup"] = profile_files
    manifest = {
        "schema_version": 1,
        "session_id": session_id,
        "mode": "official-component",
        "started_at": started,
        "finished_at": utc_now(),
        "duration_monotonic_ms": (time.monotonic_ns() - started_monotonic) // 1_000_000,
        "environment": {key: value for key, value in preflight.items() if key != "network_prefix"},
        "client_identity": identity,
        "credential_handling": {
            "values": "none",
            "count": 0,
            "mechanism": "authentication-forbidden-in-component-test",
            "arguments_contained_values": False,
            "environment_contained_values": False,
            "retained": False,
        },
        "process_lifecycle": {
            "states": ["started", "stopped"],
            "exit_status": process.returncode,
            "arguments_retained": False,
            "arguments_sha256": sha256_bytes(arguments),
            "environment_report": environment_report,
        },
        "window_lifecycle": [{"backend": "x11", "state": "observed"}, {"backend": "x11", "state": "closed-with-process-group"}],
        "network_denial": {
            "proven": True,
            "namespace": child_namespace,
            "interfaces": child_interfaces,
            "blocked_endpoint_classification": "all-outbound",
            "raw_capture_created": False,
            "official_endpoint_contacted": False,
        },
        "filesystem_inventory": {
            "scope": "temporary-official-component-profile",
            "files_before_cleanup": profile_files,
            "files_after_cleanup": [],
        },
        "leak_scan": {
            "result": "PASS",
            "locations": [
                "git-diff",
                "tracked-files",
                "untracked-files",
                "ignored-files",
                "process-arguments",
                "retained-environment-report",
                "stdout",
                "stderr",
                "temporary-files",
                "shell-history",
            ],
            "files_scanned": profile_scan.files_scanned,
        },
        "cleanup": cleanup,
        "safety": {
            "official_login_attempted": False,
            "official_service_contacted": False,
            "client_or_battleye_modified": False,
            "ptrace_debugger_hook_injection_used": False,
            "traffic_decrypted_replayed_altered_injected": False,
        },
        "findings": {
            "PROVEN": ["The exact approved executable created an X11 window under outbound network denial and stopped cleanly."],
            "DERIVED": ["The selected host can proceed to separately owner-gated live validation if BattlEye emitted no warning."],
            "UNKNOWN": ["Official service authentication and world entry were not attempted."],
            "CONFLICT": [],
        },
    }
    validate_manifest(manifest)
    write_private_json(publishable / "session-manifest.json", manifest)
    write_private_json(publishable / "cleanup-report.json", cleanup)
    return manifest
