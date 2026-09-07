#!/usr/bin/env python3
"""Validate Platform's immutable META binding and local provider invariants.

The META validator is loaded from the exact commit selected by the provider binding.
This module deliberately does not reimplement META's provider-overlay grammar.
"""

from __future__ import annotations

import base64
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
from types import ModuleType
from typing import Any, Callable
import urllib.error
import urllib.parse
import urllib.request

REPO_ROOT = Path(__file__).resolve().parents[2]
BINDING_PATH = Path("docs/agents/META_AGENT_POLICY_BINDING.json")
ROOT_AGENTS_PATH = Path("AGENTS.md")
BOOTSTRAP_PATH = Path("docs/agents/PLATFORM_AGENT_BOOTSTRAP.md")
GOVERNANCE_CONTRACT_PATH = Path("docs/agents/GOVERNANCE_CONTRACT.json")
META_VALIDATOR_PATH = Path("tools/governance/central_agent_policy.py")
PLATFORM_REPOSITORY = "Oteryn/Oteryn-Platform"
META_REPOSITORY = "Oteryn/Oteryn"
META_POLICY_PATH = "ecosystem/organization-agent-policy.json"
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
EXPECTED_BINDING = {
    "schema_version": 1,
    "policy_id": "OTERYN_ORGANIZATION_AGENT_POLICY",
    "policy_version": "3.0.0",
    "authority_repository": META_REPOSITORY,
    "organization_policy_path": "docs/agents/policy/ORGANIZATION_AGENT_POLICY.md",
    "prompting_standard_path": "docs/agents/policy/PROMPTING_STANDARD.md",
    "prompt_eval_standard_path": "docs/agents/policy/PROMPT_EVAL_STANDARD.md",
}

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
    "github_ci_default": "Use GitHub APIs and repository CI as the default control plane",
    "remote_desktop_default_deny": "Remote Desktop access is denied by default and requires explicit owner authorization for the exact invocation.",
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


def _meta_checkout_clean(meta_root: Path) -> bool:
    process = subprocess.run(
        ["git", "status", "--porcelain", "--untracked-files=no"],
        cwd=meta_root,
        text=True,
        capture_output=True,
        check=False,
    )
    if process.returncode != 0:
        raise PolicyConsistencyError(
            f"unable to inspect bound META checkout: {process.stderr.strip()}"
        )
    return not process.stdout.strip()


def _github_json(url: str, *, timeout: float = 15.0) -> object:
    headers = {
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return json.load(response)


def _github_text_at_commit(repository: str, commit: str, relative: str) -> str:
    quoted_path = urllib.parse.quote(relative, safe="/")
    payload = _github_json(
        f"https://api.github.com/repos/{repository}/contents/{quoted_path}?ref={commit}"
    )
    if not isinstance(payload, dict) or payload.get("encoding") != "base64":
        raise ValueError(f"invalid GitHub contents response for {relative}")
    encoded = payload.get("content")
    if not isinstance(encoded, str) or not encoded:
        raise ValueError(f"empty GitHub contents response for {relative}")
    return base64.b64decode(encoded).decode("utf-8")


def resolve_trusted_meta_authority(repository: str, commit: str) -> dict[str, object] | None:
    """Authenticate one META commit without executing code from its checkout."""
    if repository != META_REPOSITORY or not isinstance(commit, str) or SHA_RE.fullmatch(commit) is None:
        return None
    try:
        commit_payload = _github_json(f"https://api.github.com/repos/{repository}/commits/{commit}")
        if not isinstance(commit_payload, dict) or commit_payload.get("sha") != commit:
            return None
        branch = _github_json(f"https://api.github.com/repos/{repository}/branches/main")
        branch_commit = branch.get("commit") if isinstance(branch, dict) else None
        main_sha = branch_commit.get("sha") if isinstance(branch_commit, dict) else None
        if (
            not isinstance(branch, dict)
            or branch.get("name") != "main"
            or branch.get("protected") is not True
            or not isinstance(main_sha, str)
            or SHA_RE.fullmatch(main_sha) is None
        ):
            return None
        comparison = _github_json(
            f"https://api.github.com/repos/{repository}/compare/{commit}...{main_sha}"
        )
        if not isinstance(comparison, dict) or comparison.get("status") not in ("ahead", "identical"):
            return None
        for key in ("base_commit", "merge_base_commit"):
            coordinate = comparison.get(key)
            if not isinstance(coordinate, dict) or coordinate.get("sha") != commit:
                return None

        policy = json.loads(_github_text_at_commit(repository, commit, META_POLICY_PATH))
        if not isinstance(policy, dict):
            return None
        paths = {
            EXPECTED_BINDING["organization_policy_path"],
            EXPECTED_BINDING["prompting_standard_path"],
            EXPECTED_BINDING["prompt_eval_standard_path"],
        }
        human_surfaces = {
            path: _github_text_at_commit(repository, commit, str(path)) for path in paths
        }
        if not all(text.strip() for text in human_surfaces.values()):
            return None
        return {
            "repository": repository,
            "commit": commit,
            "merged_to_protected_main": True,
            "protected_main_sha": main_sha,
            "branch_protected": True,
            "policy": policy,
            "human_surfaces": human_surfaces,
        }
    except (
        urllib.error.HTTPError,
        urllib.error.URLError,
        TimeoutError,
        json.JSONDecodeError,
        UnicodeDecodeError,
        ValueError,
    ):
        return None


def authenticate_meta_source(
    binding: object,
    meta_root: Path,
    *,
    authority_resolver: Callable[[str, str], object] = resolve_trusted_meta_authority,
    meta_head_resolver: Callable[[Path], str] = _meta_checkout_head,
    meta_clean_resolver: Callable[[Path], bool] = _meta_checkout_clean,
) -> tuple[list[str], object | None]:
    """Authenticate binding and checkout before importing checkout-owned Python."""
    errors: list[str] = []
    if not isinstance(binding, dict) or set(binding) != set(EXPECTED_BINDING) | {"authority_commit"}:
        return ["META trust bootstrap: provider binding keys must match the closed schema"], None
    for key, expected in EXPECTED_BINDING.items():
        value = binding.get(key)
        if key == "schema_version":
            valid = isinstance(value, int) and not isinstance(value, bool) and value == expected
        else:
            valid = value == expected
        if not valid:
            errors.append(f"META trust bootstrap: binding {key} is invalid")
    commit = binding.get("authority_commit")
    if not isinstance(commit, str) or SHA_RE.fullmatch(commit) is None:
        errors.append("META trust bootstrap: authority_commit must be a lowercase full 40-hex SHA")
    if errors:
        return errors, None

    try:
        head = meta_head_resolver(meta_root)
        clean = meta_clean_resolver(meta_root)
    except (PolicyConsistencyError, OSError, ValueError) as exc:
        return [f"META trust bootstrap: {exc}"], None
    if head != commit:
        errors.append("META trust bootstrap: checkout HEAD does not equal authority_commit")
    if clean is not True:
        errors.append("META trust bootstrap: checkout has modified tracked files")
    if errors:
        return errors, None

    try:
        resolved = authority_resolver(META_REPOSITORY, commit)
    except Exception:
        resolved = None
    if not isinstance(resolved, dict):
        return ["META trust bootstrap: authority commit could not be authenticated"], None
    if (
        resolved.get("repository") != META_REPOSITORY
        or resolved.get("commit") != commit
        or resolved.get("merged_to_protected_main") is not True
        or resolved.get("branch_protected") is not True
    ):
        return ["META trust bootstrap: authority is not an ancestor of protected META main"], None
    return [], resolved


def validate_policy(
    root: Path = REPO_ROOT,
    *,
    meta_root: Path,
    meta_module: ModuleType | None = None,
    authority_resolver: Callable[[str, str], object] | None = None,
    meta_head_resolver: Callable[[Path], str] = _meta_checkout_head,
    meta_clean_resolver: Callable[[Path], bool] = _meta_checkout_clean,
) -> list[str]:
    errors: list[str] = []
    try:
        binding = _read_json(root, BINDING_PATH)
        root_agents = _read_text(root, ROOT_AGENTS_PATH)
        bootstrap = _read_text(root, BOOTSTRAP_PATH)
        governance = _read_json(root, GOVERNANCE_CONTRACT_PATH)

        bootstrap_errors, resolved_authority = authenticate_meta_source(
            binding,
            meta_root,
            authority_resolver=authority_resolver or resolve_trusted_meta_authority,
            meta_head_resolver=meta_head_resolver,
            meta_clean_resolver=meta_clean_resolver,
        )
        errors.extend(bootstrap_errors)
        if bootstrap_errors:
            return errors

        central = meta_module or load_meta_validator(meta_root)
        policy = central.load_policy(meta_root)

        errors.extend(f"META source: {error}" for error in central.validate_meta_bundle(meta_root, policy))
        errors.extend(
            f"META binding: {error}"
            for error in central.validate_provider_binding(
                binding,
                policy=policy,
                authority_resolver=lambda _repository, _commit: resolved_authority,
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
            "Remote Desktop is denied unless the owner authorizes the exact invocation.",
        ):
            if marker not in bootstrap:
                errors.append(f"PLATFORM_AGENT_BOOTSTRAP.md lacks local authority boundary: {marker}")

    except (PolicyConsistencyError, ValueError, TypeError) as exc:
        errors.append(str(exc))
    return errors


def main() -> int:
    try:
        binding = _read_json(REPO_ROOT, BINDING_PATH)
    except PolicyConsistencyError as exc:
        print(f"policy-consistency: {exc}")
        return 1
    commit = binding.get("authority_commit") if isinstance(binding, dict) else None
    if not isinstance(commit, str) or SHA_RE.fullmatch(commit) is None:
        commit = "invalid-binding"
    errors = validate_policy(
        REPO_ROOT,
        meta_root=REPO_ROOT / "_meta-policy" / commit,
    )
    if errors:
        for error in errors:
            print(f"policy-consistency: {error}")
        return 1
    print("Agent governance policy consistency: PASS (authenticated META binding + Platform invariants)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
