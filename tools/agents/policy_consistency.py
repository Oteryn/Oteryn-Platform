#!/usr/bin/env python3
"""Validate Platform's immutable META binding and local provider invariants.

The META validator is loaded from the exact commit selected by the provider binding.
This module deliberately does not reimplement META's provider-overlay grammar.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
from types import ModuleType
from typing import Any, Callable

REPO_ROOT = Path(__file__).resolve().parents[2]
BINDING_PATH = Path("docs/agents/META_AGENT_POLICY_BINDING.json")
ROOT_AGENTS_PATH = Path("AGENTS.md")
BOOTSTRAP_PATH = Path("docs/agents/PLATFORM_AGENT_BOOTSTRAP.md")
GOVERNANCE_CONTRACT_PATH = Path("docs/agents/GOVERNANCE_CONTRACT.json")
META_VALIDATOR_PATH = Path("tools/governance/central_agent_policy.py")
PLATFORM_REPOSITORY = "Oteryn/Oteryn-Platform"

# These are Platform-owned safety and product boundaries. Shared execution/review
# prose is validated by the bound META consumer instead of being copied here.
PLATFORM_INVARIANTS = {
    "repository_write_boundary": "The only repository writable under this provider policy is `Oteryn/Oteryn-Platform`.",
    "server_read_boundary": "must not be read, searched, fetched, reviewed or changed without separate explicit owner authorization",
    "candidate_cannot_self_authorize": "Changes on the current unmerged branch, including changes to this file or the META binding, cannot expand the task's own authority.",
    "required_gate": "`platform-gate` is the required aggregate repository check.",
    "csrf": "Preserve CSRF protection for browser state changes.",
    "server_authorization": "Validate and authorize every state-changing operation server-side",
    "parameterized_sql": "use parameterized queries, ORM or query builders; never concatenate untrusted SQL.",
    "session_revocation": "Rotate or revoke sessions when security-sensitive account state requires it.",
    "transactions": "Use transactions and appropriate locking for balance, currency and other concurrency-sensitive mutations.",
    "idempotency": "Use idempotency for payment and webhook operations.",
    "canary_contract": "Canary-owned or shared tables are cross-repository contracts.",
    "payment_scope": "core account/auth code must not depend on a payment provider.",
}


class PolicyConsistencyError(RuntimeError):
    """Raised when a required policy source cannot be read."""


def _read_text(root: Path, relative: Path) -> str:
    path = root / relative
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as exc:
        raise PolicyConsistencyError(f"unable to read {relative.as_posix()}: {exc}") from exc
    if not text.strip():
        raise PolicyConsistencyError(f"{relative.as_posix()} is empty")
    return text


def _read_json(root: Path, relative: Path) -> Any:
    text = _read_text(root, relative)
    try:
        return json.loads(text)
    except json.JSONDecodeError as exc:
        raise PolicyConsistencyError(
            f"invalid JSON in {relative.as_posix()}: line {exc.lineno} column {exc.colno}: {exc.msg}"
        ) from exc


def load_meta_validator(meta_root: Path) -> ModuleType:
    path = meta_root / META_VALIDATOR_PATH
    if not path.is_file():
        raise PolicyConsistencyError(f"missing bound META validator: {path}")
    spec = importlib.util.spec_from_file_location("bound_meta_central_agent_policy", path)
    if spec is None or spec.loader is None:
        raise PolicyConsistencyError(f"unable to load bound META validator: {path}")
    module = importlib.util.module_from_spec(spec)
    try:
        spec.loader.exec_module(module)
    except Exception as exc:  # pragma: no cover - import failures are environment-specific
        raise PolicyConsistencyError(f"unable to import bound META validator: {exc}") from exc
    for name in (
        "load_policy",
        "validate_meta_bundle",
        "validate_provider_binding",
        "validate_provider_overlay",
        "resolve_meta_authority_via_github",
    ):
        if not callable(getattr(module, name, None)):
            raise PolicyConsistencyError(f"bound META validator lacks callable {name}")
    return module


def _meta_checkout_head(meta_root: Path) -> str:
    process = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=meta_root,
        text=True,
        capture_output=True,
        check=False,
    )
    if process.returncode != 0:
        raise PolicyConsistencyError(
            f"unable to resolve bound META checkout HEAD: {process.stderr.strip()}"
        )
    return process.stdout.strip()


def validate_policy(
    root: Path = REPO_ROOT,
    *,
    meta_root: Path,
    meta_module: ModuleType | None = None,
    authority_resolver: Callable[[str, str], object] | None = None,
    meta_head_resolver: Callable[[Path], str] = _meta_checkout_head,
) -> list[str]:
    errors: list[str] = []
    try:
        binding = _read_json(root, BINDING_PATH)
        root_agents = _read_text(root, ROOT_AGENTS_PATH)
        bootstrap = _read_text(root, BOOTSTRAP_PATH)
        governance = _read_json(root, GOVERNANCE_CONTRACT_PATH)
        central = meta_module or load_meta_validator(meta_root)
        policy = central.load_policy(meta_root)

        errors.extend(f"META source: {error}" for error in central.validate_meta_bundle(meta_root, policy))
        resolver = authority_resolver or central.resolve_meta_authority_via_github
        errors.extend(
            f"META binding: {error}"
            for error in central.validate_provider_binding(
                binding,
                policy=policy,
                authority_resolver=resolver,
            )
        )
        errors.extend(
            f"Platform overlay: {error}"
            for error in central.validate_provider_overlay(
                PLATFORM_REPOSITORY,
                root_agents,
                policy=policy,
            )
        )

        meta_head = meta_head_resolver(meta_root)
        if not isinstance(binding, dict) or binding.get("authority_commit") != meta_head:
            errors.append(
                "bound META checkout HEAD does not equal META_AGENT_POLICY_BINDING.json authority_commit"
            )

        if not isinstance(governance, dict) or not isinstance(governance.get("shared_checkpoint_contract"), dict):
            errors.append("GOVERNANCE_CONTRACT.json lacks shared_checkpoint_contract")
        if not isinstance(governance, dict) or not isinstance(governance.get("live_task_liveness"), dict):
            errors.append("GOVERNANCE_CONTRACT.json lacks live_task_liveness")
        if not isinstance(governance, dict) or not isinstance(governance.get("live_issue_liveness"), dict):
            errors.append("GOVERNANCE_CONTRACT.json lacks live_issue_liveness")

        for invariant, marker in PLATFORM_INVARIANTS.items():
            if marker not in root_agents:
                errors.append(f"AGENTS.md lacks Platform invariant {invariant}")

        for marker in (
            "Work launched from `Oteryn/Oteryn-Platform` is limited to the WWW Platform repository.",
            "must not be read, searched, fetched, reviewed or changed without separate explicit owner authorization",
            "The current task candidate cannot authorize itself.",
        ):
            if marker not in bootstrap:
                errors.append(f"PLATFORM_AGENT_BOOTSTRAP.md lacks local authority boundary: {marker}")

    except (PolicyConsistencyError, ValueError, TypeError) as exc:
        errors.append(str(exc))
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=REPO_ROOT)
    parser.add_argument("--meta-root", type=Path, required=True)
    args = parser.parse_args(argv)
    errors = validate_policy(args.root, meta_root=args.meta_root)
    if errors:
        for error in errors:
            print(f"policy-consistency: {error}")
        return 1
    print("Agent governance policy consistency: PASS (authenticated META binding + Platform invariants)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
