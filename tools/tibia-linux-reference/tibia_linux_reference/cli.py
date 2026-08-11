from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from . import HarnessError
from .identity import ApprovedIdentity, verify_client_identity
from .launcher import run_official_component, run_synthetic_dry_run
from .manifest import validate_manifest
from .preflight import run_preflight


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description="Bounded Tibia Linux reference harness")
    subcommands = root.add_subparsers(dest="command", required=True)

    dry = subcommands.add_parser("dry-run", help="run the deterministic fake client with outbound networking denied")
    dry.add_argument("--repo-root", type=Path, required=True)
    dry.add_argument("--evidence-dir", type=Path, required=True)

    preflight = subcommands.add_parser("preflight", help="validate a synthetic or official execution environment")
    preflight.add_argument("--repo-root", type=Path, required=True)
    preflight.add_argument("--evidence-dir", type=Path, required=True)
    preflight.add_argument("--mode", choices=("synthetic", "official"), required=True)

    identity = subcommands.add_parser("verify-identity", help="verify package, executable, and ELF identity without execution")
    identity.add_argument("--repo-root", type=Path, required=True)
    identity.add_argument("--identity", type=Path, required=True)
    identity.add_argument("--package", type=Path, required=True)
    identity.add_argument("--executable", type=Path, required=True)

    official = subcommands.add_parser("official-component", help="launch the verified client without authentication under network denial")
    official.add_argument("--repo-root", type=Path, required=True)
    official.add_argument("--evidence-dir", type=Path, required=True)
    official.add_argument("--identity", type=Path, required=True)
    official.add_argument("--package", type=Path, required=True)
    official.add_argument("--executable", type=Path, required=True)
    official.add_argument("--observation-seconds", type=float, default=5.0)

    manifest = subcommands.add_parser("validate-manifest", help="validate a redacted manifest against the local schema")
    manifest.add_argument("path", type=Path)
    return root


def _safe_error_code(error: BaseException) -> str:
    if not isinstance(error, HarnessError):
        return error.__class__.__name__
    message = str(error)
    static_prefixes = (
        ("no fail-closed network namespace control", "network-isolator-unavailable"),
        ("synthetic credential channel was not created", "credential-channel-not-created"),
        ("synthetic credential channel closed before consumption", "credential-channel-closed"),
        ("fake client did not prove network denial", "network-denial-unproven"),
        ("fake client did not enter a distinct network namespace", "network-namespace-not-distinct"),
        ("fake client graphical lifecycle was incomplete", "graphical-lifecycle-incomplete"),
        ("fake client process lifecycle was incomplete", "process-lifecycle-incomplete"),
        ("synthetic secret leak detected", "synthetic-leak-detected"),
        ("deterministic cleanup verification failed", "cleanup-failed"),
        ("post-manifest leak scan failed", "post-manifest-leak-detected"),
        ("ordinary environment contains secret-like material", "environment-secret-like"),
        ("shell tracing or verbose command echo is enabled", "shell-tracing-enabled"),
        ("evidence directory must", "evidence-boundary-invalid"),
    )
    for prefix, code in static_prefixes:
        if message.startswith(prefix):
            return code
    fake_prefix = "fake client failed with "
    if message.startswith(fake_prefix):
        failure_class = message[len(fake_prefix):]
        if failure_class.isidentifier() and len(failure_class) <= 64:
            return "fake-client-" + failure_class.lower()
        return "fake-client-failed"
    return "harness-validation-failed"


def main() -> int:
    arguments = parser().parse_args()
    try:
        if arguments.command == "dry-run":
            result = run_synthetic_dry_run(
                repo_root=arguments.repo_root.resolve(strict=True),
                evidence_root=arguments.evidence_dir,
            )
            print(json.dumps({"result": "PASS", "session_id": result["session_id"], "manifest": "publishable/session-manifest.json"}, sort_keys=True))
        elif arguments.command == "preflight":
            result = run_preflight(
                repo_root=arguments.repo_root.resolve(strict=True),
                evidence_root=arguments.evidence_dir,
                mode=arguments.mode,
            )
            result.pop("network_prefix", None)
            print(json.dumps(result, indent=2, sort_keys=True))
        elif arguments.command == "verify-identity":
            approved = ApprovedIdentity.load(arguments.identity)
            result = verify_client_identity(
                approved,
                package_path=arguments.package,
                executable_path=arguments.executable,
                repo_root=arguments.repo_root.resolve(strict=True),
            )
            print(json.dumps(result, indent=2, sort_keys=True))
        elif arguments.command == "official-component":
            result = run_official_component(
                repo_root=arguments.repo_root.resolve(strict=True),
                evidence_root=arguments.evidence_dir,
                identity_file=arguments.identity,
                package_path=arguments.package,
                executable_path=arguments.executable,
                observation_seconds=arguments.observation_seconds,
            )
            print(json.dumps({"result": "PASS", "session_id": result["session_id"]}, sort_keys=True))
        else:
            document = json.loads(arguments.path.read_text(encoding="utf-8"))
            validate_manifest(document)
            print(json.dumps({"result": "PASS", "schema_version": document["schema_version"]}, sort_keys=True))
        return 0
    except (HarnessError, FileNotFoundError, PermissionError, json.JSONDecodeError) as error:
        print(
            json.dumps(
                {"result": "FAIL", "error": error.__class__.__name__, "reason": _safe_error_code(error)},
                sort_keys=True,
            ),
            file=sys.stderr,
        )
        return 2


if __name__ == "__main__":
    sys.exit(main())
