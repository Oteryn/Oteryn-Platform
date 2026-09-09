#!/usr/bin/env python3
"""Authenticate and consume the immutable META agent policy for Platform."""

from __future__ import annotations

import argparse
import base64
import hashlib
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
from types import ModuleType
from typing import Callable
import urllib.request

REPO_ROOT = Path(__file__).resolve().parents[2]
PROVIDER = "Oteryn/Oteryn-Platform"
AUTHORITY_REPOSITORY = "Oteryn/Oteryn"
POLICY_ID = "OTERYN_ORGANIZATION_AGENT_POLICY"
POLICY_VERSION = "3.1.0"
BINDING_PATH = Path("docs/agents/META_AGENT_POLICY_BINDING.json")
CATALOG_PATH = Path("docs/agents/DOCUMENTATION_IA_CATALOG.json")
CENTRAL_MODULE_PATH = Path("tools/governance/central_agent_policy.py")
POLICY_PATH = Path("ecosystem/organization-agent-policy.json")
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
EXPECTED_BINDING_KEYS = {
    "schema_version",
    "policy_id",
    "policy_version",
    "authority_repository",
    "authority_commit",
    "organization_policy_path",
    "prompting_standard_path",
    "prompt_eval_standard_path",
}
EXPECTED_SURFACES = {
    "organization_policy": "docs/agents/policy/ORGANIZATION_AGENT_POLICY.md",
    "prompting_standard": "docs/agents/policy/PROMPTING_STANDARD.md",
    "prompt_eval_standard": "docs/agents/policy/PROMPT_EVAL_STANDARD.md",
}

GitHubJson = Callable[[str], object]


class PolicyConsistencyError(RuntimeError):
    """Raised when a required policy source cannot be consumed safely."""


def _read_json(path: Path) -> dict[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise PolicyConsistencyError(f"cannot read valid JSON from {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise PolicyConsistencyError(f"{path} must contain a JSON object")
    return value


def _git(meta_root: Path, *args: str) -> str:
    try:
        return subprocess.run(
            ["git", *args],
            cwd=meta_root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        raise PolicyConsistencyError(f"cannot verify META checkout with git {' '.join(args)}") from exc


def _github_json_reader(token: str | None) -> GitHubJson:
    def read(url: str) -> object:
        headers = {
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
        }
        if token:
            headers["Authorization"] = f"Bearer {token}"
        request = urllib.request.Request(url, headers=headers)
        try:
            with urllib.request.urlopen(request, timeout=20.0) as response:
                return json.load(response)
        except Exception as exc:
            raise PolicyConsistencyError(f"GitHub authority read failed for {url}: {exc}") from exc

    return read


def _safe_policy_path(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise PolicyConsistencyError(f"{label} must be a non-empty repository path")
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts:
        raise PolicyConsistencyError(f"{label} must stay inside the META repository")
    return value


def _blob_sha(data: bytes) -> str:
    return hashlib.sha1(b"blob " + str(len(data)).encode("ascii") + b"\0" + data).hexdigest()


def _prevalidate_binding(binding: dict[str, object]) -> str:
    if set(binding) != EXPECTED_BINDING_KEYS:
        raise PolicyConsistencyError("provider binding keys must match the closed schema before META code load")
    if binding.get("schema_version") != 1 or isinstance(binding.get("schema_version"), bool):
        raise PolicyConsistencyError("provider binding schema_version must be 1 before META code load")
    if binding.get("policy_id") != POLICY_ID or binding.get("policy_version") != POLICY_VERSION:
        raise PolicyConsistencyError("provider binding policy identity/version is invalid before META code load")
    if binding.get("authority_repository") != AUTHORITY_REPOSITORY:
        raise PolicyConsistencyError("provider binding authority repository is invalid before META code load")
    commit = binding.get("authority_commit")
    if not isinstance(commit, str) or SHA_RE.fullmatch(commit) is None:
        raise PolicyConsistencyError("provider binding authority_commit is invalid before META code load")
    actual_surfaces = {
        "organization_policy": binding.get("organization_policy_path"),
        "prompting_standard": binding.get("prompting_standard_path"),
        "prompt_eval_standard": binding.get("prompt_eval_standard_path"),
    }
    if actual_surfaces != EXPECTED_SURFACES:
        raise PolicyConsistencyError("provider binding canonical paths are invalid before META code load")
    return commit


def authenticate_meta_checkout(
    meta_root: Path,
    binding: dict[str, object],
    *,
    github_json: GitHubJson,
) -> dict[str, object]:
    """Authenticate exact clean bytes and protected-main ancestry before import."""

    commit = _prevalidate_binding(binding)
    if _git(meta_root, "rev-parse", "--verify", "HEAD") != commit:
        raise PolicyConsistencyError("META checkout HEAD does not match authority_commit")
    if _git(meta_root, "status", "--porcelain", "--untracked-files=all"):
        raise PolicyConsistencyError("META policy checkout must be clean before loading executable policy code")
    remote = _git(meta_root, "remote", "get-url", "origin").removesuffix(".git").rstrip("/")
    if remote not in {"https://github.com/Oteryn/Oteryn", "git@github.com:Oteryn/Oteryn"}:
        raise PolicyConsistencyError("META checkout origin does not match Oteryn/Oteryn")

    api = "https://api.github.com/repos/Oteryn/Oteryn"
    commit_payload = github_json(f"{api}/commits/{commit}")
    branch_payload = github_json(f"{api}/branches/main")
    if not isinstance(commit_payload, dict) or commit_payload.get("sha") != commit:
        raise PolicyConsistencyError("authority_commit is not available from GitHub")
    if not isinstance(branch_payload, dict) or branch_payload.get("protected") is not True:
        raise PolicyConsistencyError("META main is not authenticated as protected")
    branch_commit = branch_payload.get("commit")
    main_sha = branch_commit.get("sha") if isinstance(branch_commit, dict) else None
    if not isinstance(main_sha, str) or SHA_RE.fullmatch(main_sha) is None:
        raise PolicyConsistencyError("META protected-main head is invalid")
    compare = github_json(f"{api}/compare/{commit}...{main_sha}")
    if not isinstance(compare, dict) or compare.get("status") not in {"ahead", "identical"}:
        raise PolicyConsistencyError("authority_commit is not an ancestor of protected META main")
    for key in ("base_commit", "merge_base_commit"):
        coordinate = compare.get(key)
        if not isinstance(coordinate, dict) or coordinate.get("sha") != commit:
            raise PolicyConsistencyError("GitHub ancestry response does not bind the authority commit")

    tree_payload = github_json(f"{api}/git/trees/{commit}?recursive=1")
    if not isinstance(tree_payload, dict) or tree_payload.get("truncated") is True:
        raise PolicyConsistencyError("GitHub authority tree is unavailable or truncated")
    tree = tree_payload.get("tree")
    if not isinstance(tree, list):
        raise PolicyConsistencyError("GitHub authority tree is invalid")
    remote_blobs = {
        entry.get("path"): entry.get("sha")
        for entry in tree
        if isinstance(entry, dict) and entry.get("type") == "blob"
    }

    fixed_paths = {str(CENTRAL_MODULE_PATH), str(POLICY_PATH), *EXPECTED_SURFACES.values()}
    for relative in fixed_paths:
        data = (meta_root / relative).read_bytes()
        if remote_blobs.get(relative) != _blob_sha(data):
            raise PolicyConsistencyError(f"META policy source does not match authenticated GitHub blob: {relative}")

    policy = _read_json(meta_root / POLICY_PATH)
    if (
        policy.get("policy_id") != POLICY_ID
        or policy.get("policy_version") != POLICY_VERSION
        or policy.get("authority_repository") != AUTHORITY_REPOSITORY
        or policy.get("canonical_human_surfaces") != EXPECTED_SURFACES
    ):
        raise PolicyConsistencyError("authenticated META policy identity or canonical surfaces are invalid")
    machine = policy.get("machine_authorities")
    if not isinstance(machine, list) or not machine:
        raise PolicyConsistencyError("authenticated META policy machine_authorities are invalid")
    consumed_paths = set(fixed_paths)
    for index, value in enumerate(machine):
        consumed_paths.add(_safe_policy_path(value, f"machine_authorities[{index}]"))
    for relative in consumed_paths:
        try:
            data = (meta_root / relative).read_bytes()
        except OSError as exc:
            raise PolicyConsistencyError(f"cannot read consumed META authority {relative}: {exc}") from exc
        if remote_blobs.get(relative) != _blob_sha(data):
            raise PolicyConsistencyError(f"consumed META authority does not match authenticated GitHub blob: {relative}")

    human_surfaces = {
        relative: (meta_root / relative).read_text(encoding="utf-8")
        for relative in EXPECTED_SURFACES.values()
    }
    return {
        "repository": AUTHORITY_REPOSITORY,
        "commit": commit,
        "merged_to_protected_main": True,
        "protected_main_sha": main_sha,
        "branch_protected": True,
        "policy": policy,
        "human_surfaces": human_surfaces,
        "blob_sha256": {
            relative: hashlib.sha256((meta_root / relative).read_bytes()).hexdigest()
            for relative in consumed_paths
        },
    }


def load_central_module(meta_root: Path, authority: dict[str, object]) -> ModuleType:
    commit = authority.get("commit")
    if not isinstance(commit, str) or _git(meta_root, "rev-parse", "--verify", "HEAD") != commit:
        raise PolicyConsistencyError("authenticated META checkout identity changed before code load")
    if _git(meta_root, "status", "--porcelain", "--untracked-files=all"):
        raise PolicyConsistencyError("authenticated META checkout changed before code load")
    digests = authority.get("blob_sha256")
    module_relative = str(CENTRAL_MODULE_PATH)
    if not isinstance(digests, dict) or digests.get(module_relative) != hashlib.sha256(
        (meta_root / CENTRAL_MODULE_PATH).read_bytes()
    ).hexdigest():
        raise PolicyConsistencyError("central META validator bytes changed after authentication")

    module_path = meta_root / CENTRAL_MODULE_PATH
    spec = importlib.util.spec_from_file_location("oteryn_central_agent_policy", module_path)
    if spec is None or spec.loader is None:
        raise PolicyConsistencyError(f"cannot load central META validator: {module_path}")
    module = importlib.util.module_from_spec(spec)
    previous_bytecode_setting = sys.dont_write_bytecode
    sys.dont_write_bytecode = True
    try:
        spec.loader.exec_module(module)
    except Exception as exc:  # pragma: no cover - defensive loader boundary
        raise PolicyConsistencyError(f"central META validator failed to load: {exc}") from exc
    finally:
        sys.dont_write_bytecode = previous_bytecode_setting
    for name in (
        "load_policy",
        "validate_meta_bundle",
        "validate_provider_binding",
        "validate_provider_overlay",
        "validate_task_prompt_text",
    ):
        if not callable(getattr(module, name, None)):
            raise PolicyConsistencyError(f"central META validator lacks callable {name}")
    return module


def _validate_prompt_inventory(root: Path, module: ModuleType, policy: dict[str, object]) -> list[str]:
    errors: list[str] = []
    catalog = _read_json(root / CATALOG_PATH)
    prompts = catalog.get("prompts")
    if not isinstance(prompts, list):
        return [f"{CATALOG_PATH}: prompts must be a list"]
    seen: set[str] = set()
    for index, entry in enumerate(prompts):
        if not isinstance(entry, dict):
            errors.append(f"{CATALOG_PATH}: prompt entry {index} must be an object")
            continue
        relative = entry.get("path")
        if not isinstance(relative, str) or not relative.startswith("docs/agents/prompts/"):
            errors.append(f"{CATALOG_PATH}: prompt entry {index} has invalid path")
            continue
        if relative in seen:
            errors.append(f"{CATALOG_PATH}: duplicate prompt path {relative}")
            continue
        seen.add(relative)
        try:
            text = (root / relative).read_text(encoding="utf-8")
        except OSError as exc:
            errors.append(f"{relative}: cannot read catalogued prompt: {exc}")
            continue
        executable = entry.get("executable")
        classification = entry.get("classification")
        status = entry.get("status")
        if executable is True:
            if classification != "reusable" or status != "active_reusable":
                errors.append(f"{relative}: executable prompt must be reusable/active_reusable")
                continue
            errors.extend(f"{relative}: {error}" for error in module.validate_task_prompt_text(text, policy=policy))
        elif executable is False:
            if classification != "one_shot_historical" or status != "historical_do_not_run":
                errors.append(f"{relative}: inert prompt must be one_shot_historical/historical_do_not_run")
        else:
            errors.append(f"{relative}: executable must be a JSON boolean")
    return errors


def validate_policy(root: Path, meta_root: Path, authority: dict[str, object]) -> list[str]:
    errors: list[str] = []
    try:
        module = load_central_module(meta_root, authority)
        policy = module.load_policy(meta_root)
        errors.extend(f"META: {error}" for error in module.validate_meta_bundle(meta_root, policy))
        binding = _read_json(root / BINDING_PATH)
        errors.extend(
            f"{BINDING_PATH}: {error}"
            for error in module.validate_provider_binding(
                binding,
                policy=policy,
                authority_resolver=lambda repository, commit: authority
                if repository == authority.get("repository") and commit == authority.get("commit")
                else None,
            )
        )
        root_agents = (root / "AGENTS.md").read_text(encoding="utf-8")
        errors.extend(
            f"AGENTS.md: {error}"
            for error in module.validate_provider_overlay(PROVIDER, root_agents, policy=policy)
        )
        errors.extend(_validate_prompt_inventory(root, module, policy))
    except (PolicyConsistencyError, OSError, json.JSONDecodeError, ValueError) as exc:
        errors.append(str(exc))
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--meta-root",
        type=Path,
        default=Path(os.environ.get("OTERYN_META_POLICY_ROOT", ".meta-agent-policy")),
        help="Exact clean checkout of the META authority commit selected by the binding",
    )
    args = parser.parse_args()
    try:
        binding = _read_json(REPO_ROOT / BINDING_PATH)
        token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
        authority = authenticate_meta_checkout(
            args.meta_root,
            binding,
            github_json=_github_json_reader(token),
        )
        errors = validate_policy(REPO_ROOT, args.meta_root, authority)
    except PolicyConsistencyError as exc:
        errors = [str(exc)]
    if errors:
        for error in errors:
            print(f"policy-consistency: {error}", file=sys.stderr)
        return 1
    print(
        "Platform central agent policy adoption: PASS "
        f"(authenticated {authority['commit']} on protected META main {authority['protected_main_sha']})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
