#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path
from typing import Any

from branch_lifecycle import ApiError, GitHubClient, ValidationError

ISSUE_NUMBER = 1068
SCHEMA_VERSION = 1
CONFIRMATION = "DELETE_REVIEWED_HISTORICAL_REDUNDANT_BRANCHES_ISSUE_1068"
DEFAULT_APPROVAL_PATH = Path("docs/agents/HISTORICAL_BRANCH_DELETION_APPROVAL.json")
ACTIVE_TASKS_PATH = Path("docs/agents/tasks/active")
FULL_SHA_RE = re.compile(r"^[0-9a-f]{40}$")
TASK_BRANCH_RE = re.compile(r"^\s*(?:branch|lock_branch):\s*([^\s#]+)\s*$", re.MULTILINE)
RESERVED_PURPOSE_RE = re.compile(r"(?:^|[-_/])(backup|rollback|recovery)(?:$|[-_/])", re.IGNORECASE)
PLACEHOLDER_BRANCHES = {"", "none", "null", "unknown", "n/a", "pending"}


def canonical_json(value: object) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(canonical_json(value), encoding="utf-8")


def load_canonical_json(path: Path) -> dict[str, Any]:
    raw = path.read_text(encoding="utf-8")
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise ValidationError(
            f"{path.as_posix()}: invalid JSON at {exc.lineno}:{exc.colno}: {exc.msg}"
        ) from exc
    if not isinstance(value, dict):
        raise ValidationError(f"{path.as_posix()}: root must be an object")
    if raw != canonical_json(value):
        raise ValidationError(
            f"{path.as_posix()}: JSON must use sorted keys, two-space indentation and one trailing newline"
        )
    return value


def run_git(
    root: Path,
    args: list[str],
    *,
    timeout: int = 60,
    allow: tuple[int, ...] = (0,),
    purpose: str,
) -> subprocess.CompletedProcess[str]:
    try:
        result = subprocess.run(
            ["git", *args],
            cwd=root,
            text=True,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
    except FileNotFoundError as exc:
        raise ValidationError(f"{purpose}: git executable is unavailable") from exc
    except subprocess.TimeoutExpired as exc:
        raise ValidationError(f"{purpose}: git command timed out") from exc
    if result.returncode not in allow:
        detail = (result.stderr or result.stdout).strip()
        raise ValidationError(
            f"{purpose}: git exited {result.returncode}: {detail[:500]}"
        )
    return result


def validate_repository_root(root: Path) -> Path:
    root = root.resolve()
    result = run_git(
        root,
        ["rev-parse", "--show-toplevel"],
        timeout=15,
        purpose="repository root validation",
    )
    reported = result.stdout.strip()
    if not reported or Path(reported).resolve() != root:
        raise ValidationError(
            f"configured root {root} does not match git working tree {reported!r}"
        )
    return root


def ensure_commit_object(root: Path, sha: str, *, context: str) -> None:
    if not FULL_SHA_RE.fullmatch(sha):
        raise ValidationError(f"{context}: invalid full commit SHA {sha!r}")
    run_git(
        root,
        ["cat-file", "-e", f"{sha}^{{commit}}"],
        timeout=30,
        purpose=f"{context} object validation",
    )


def git_lines(root: Path, args: list[str], *, purpose: str) -> list[str]:
    result = run_git(root, args, timeout=120, purpose=purpose)
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def branch_relation(root: Path, main_sha: str, branch_sha: str) -> dict[str, Any]:
    ensure_commit_object(root, main_sha, context="default branch")
    ensure_commit_object(root, branch_sha, context="branch")

    ancestor_result = run_git(
        root,
        ["merge-base", "--is-ancestor", branch_sha, main_sha],
        timeout=60,
        allow=(0, 1),
        purpose=f"ancestor test for {branch_sha}",
    )
    is_ancestor = ancestor_result.returncode == 0

    counts_raw = run_git(
        root,
        ["rev-list", "--left-right", "--count", f"{main_sha}...{branch_sha}"],
        timeout=120,
        purpose=f"divergence count for {branch_sha}",
    ).stdout.strip().split()
    if len(counts_raw) != 2 or not all(item.isdigit() for item in counts_raw):
        raise ValidationError(
            f"divergence count for {branch_sha}: unexpected output {counts_raw!r}"
        )
    behind, ahead = (int(counts_raw[0]), int(counts_raw[1]))

    main_tree = run_git(
        root,
        ["rev-parse", f"{main_sha}^{{tree}}"],
        timeout=30,
        purpose="default branch tree resolution",
    ).stdout.strip()
    branch_tree = run_git(
        root,
        ["rev-parse", f"{branch_sha}^{{tree}}"],
        timeout=30,
        purpose=f"branch tree resolution for {branch_sha}",
    ).stdout.strip()
    tree_equal = main_tree == branch_tree

    unique_merges = git_lines(
        root,
        ["rev-list", "--merges", f"{main_sha}..{branch_sha}"],
        purpose=f"unique merge scan for {branch_sha}",
    )
    patch_unique = git_lines(
        root,
        [
            "rev-list",
            "--cherry-pick",
            "--right-only",
            "--no-merges",
            f"{main_sha}...{branch_sha}",
        ],
        purpose=f"patch-equivalence scan for {branch_sha}",
    )

    if is_ancestor:
        incorporated = True
        proof = "ANCESTOR_OF_MAIN"
    elif tree_equal:
        incorporated = True
        proof = "TREE_EQUAL_TO_MAIN"
    elif not unique_merges and not patch_unique:
        incorporated = True
        proof = "PATCH_EQUIVALENT_TO_MAIN"
    else:
        incorporated = False
        proof = "UNIQUE_HISTORY_REMAINS"

    return {
        "ahead": ahead,
        "behind": behind,
        "branch_tree": branch_tree,
        "incorporated": incorporated,
        "is_ancestor_of_main": is_ancestor,
        "main_tree": main_tree,
        "patch_unique_commit_count": len(patch_unique),
        "patch_unique_commit_sample": patch_unique[:10],
        "proof": proof,
        "tree_equal_to_main": tree_equal,
        "unique_merge_commit_count": len(unique_merges),
        "unique_merge_commit_sample": unique_merges[:10],
    }


def same_repo_pull(pull: object, repo: str) -> bool:
    if not isinstance(pull, dict):
        return False
    head = pull.get("head")
    if not isinstance(head, dict):
        return False
    head_repo = head.get("repo")
    if not isinstance(head_repo, dict):
        return False
    full_name = head_repo.get("full_name")
    return isinstance(full_name, str) and full_name.casefold() == repo.casefold()


def pull_branch(pull: object) -> str | None:
    if not isinstance(pull, dict):
        return None
    head = pull.get("head")
    if not isinstance(head, dict):
        return None
    ref = head.get("ref")
    return ref if isinstance(ref, str) and ref else None


def pull_head_sha(pull: object) -> str | None:
    if not isinstance(pull, dict):
        return None
    head = pull.get("head")
    if not isinstance(head, dict):
        return None
    sha = head.get("sha")
    return sha if isinstance(sha, str) and FULL_SHA_RE.fullmatch(sha) else None


def pull_summary(pull: dict[str, Any]) -> dict[str, Any]:
    number = pull.get("number")
    state = pull.get("state")
    merged_at = pull.get("merged_at")
    title = pull.get("title")
    return {
        "head_sha": pull_head_sha(pull),
        "merged": bool(merged_at),
        "merged_at": merged_at if isinstance(merged_at, str) else None,
        "number": number if isinstance(number, int) else None,
        "state": state if state in {"open", "closed"} else "unknown",
        "title": title if isinstance(title, str) else "",
    }


def list_live_branches(client: GitHubClient) -> list[dict[str, Any]]:
    raw = client.paginate(f"/repos/{client.repo}/branches?per_page=100")
    branches: list[dict[str, Any]] = []
    for index, item in enumerate(raw):
        if not isinstance(item, dict):
            raise ValidationError(f"branches[{index}]: expected object")
        name = item.get("name")
        commit = item.get("commit")
        protected = item.get("protected")
        if not isinstance(name, str) or not name:
            raise ValidationError(f"branches[{index}].name: expected non-empty string")
        if not isinstance(commit, dict) or not isinstance(commit.get("sha"), str):
            raise ValidationError(f"branches[{index}].commit.sha: missing")
        sha = commit["sha"].lower()
        if not FULL_SHA_RE.fullmatch(sha):
            raise ValidationError(f"branches[{index}].commit.sha: invalid full SHA")
        if not isinstance(protected, bool):
            raise ValidationError(f"branches[{index}].protected: expected boolean")
        branches.append({"branch": name, "head_sha": sha, "protected": protected})
    names = [item["branch"] for item in branches]
    if len(names) != len(set(names)):
        raise ValidationError("live branch list contains duplicate branch names")
    return sorted(branches, key=lambda item: item["branch"])


def list_pull_history(client: GitHubClient) -> dict[str, list[dict[str, Any]]]:
    raw = client.paginate(
        f"/repos/{client.repo}/pulls?state=all&sort=updated&direction=desc&per_page=100"
    )
    history: dict[str, list[dict[str, Any]]] = {}
    for item in raw:
        if not isinstance(item, dict) or not same_repo_pull(item, client.repo):
            continue
        branch = pull_branch(item)
        if not branch:
            continue
        history.setdefault(branch, []).append(item)
    for branch in history:
        history[branch].sort(
            key=lambda pull: (
                int(pull.get("number")) if isinstance(pull.get("number"), int) else -1
            ),
            reverse=True,
        )
    return history


def active_task_branches(root: Path) -> dict[str, list[str]]:
    result: dict[str, list[str]] = {}
    task_root = root / ACTIVE_TASKS_PATH
    if not task_root.exists():
        return result
    for path in sorted(task_root.glob("*.md")):
        if path.name.casefold() == "readme.md":
            continue
        text = path.read_text(encoding="utf-8")
        for match in TASK_BRANCH_RE.finditer(text):
            branch = match.group(1).strip().strip("'\"")
            if branch.casefold() in PLACEHOLDER_BRANCHES:
                continue
            result.setdefault(branch, []).append(path.as_posix())
    return result


def decide_disposition(
    *,
    branch: str,
    protected: bool,
    open_pr_numbers: list[int],
    active_claims: list[str],
    relation: dict[str, Any],
) -> tuple[str, str]:
    if protected or branch == "main":
        return "PROTECTED", "protected/default branch"
    if open_pr_numbers:
        return "OPEN_PR", f"open same-repository PR(s): {open_pr_numbers}"
    if active_claims:
        return "RETAIN", f"active task branch claim(s): {active_claims}"
    if relation.get("incorporated") is True:
        return (
            "DELETE",
            f"branch content is fully represented by main ({relation.get('proof')})",
        )
    reserved = RESERVED_PURPOSE_RE.search(branch)
    if reserved:
        purpose = reserved.group(1).upper()
        return (
            "RECOVERY",
            f"unique unmerged history remains and branch name declares {purpose} purpose",
        )
    return (
        "RETAIN",
        "unique unmerged history remains and is not proven represented by main",
    )


def build_audit(
    client: GitHubClient,
    *,
    root: Path,
    default_branch: str,
    script_path: Path,
) -> tuple[dict[str, Any], dict[str, Any]]:
    root = validate_repository_root(root)
    script_path = script_path if script_path.is_absolute() else root / script_path
    implementation_sha256 = sha256_file(script_path)

    default = client.get_branch(default_branch)
    if not isinstance(default, dict):
        raise ValidationError(f"default branch {default_branch!r} is missing")
    default_commit = default.get("commit")
    default_sha = (
        default_commit.get("sha").lower()
        if isinstance(default_commit, dict) and isinstance(default_commit.get("sha"), str)
        else ""
    )
    if not FULL_SHA_RE.fullmatch(default_sha):
        raise ValidationError("default branch returned invalid commit SHA")
    if default.get("protected") is not True:
        raise ValidationError("default branch must be protected")

    branches = list_live_branches(client)
    if default_branch not in {item["branch"] for item in branches}:
        raise ValidationError("default branch is absent from live branch inventory")

    pull_history = list_pull_history(client)
    active_claims = active_task_branches(root)

    audited: list[dict[str, Any]] = []
    for item in branches:
        branch = item["branch"]
        head_sha = item["head_sha"]
        relation = branch_relation(root, default_sha, head_sha)
        pulls = pull_history.get(branch, [])
        open_pr_numbers = sorted(
            [
                int(pull["number"])
                for pull in pulls
                if pull.get("state") == "open" and isinstance(pull.get("number"), int)
            ]
        )
        claims = sorted(set(active_claims.get(branch, [])))
        disposition, reason = decide_disposition(
            branch=branch,
            protected=item["protected"],
            open_pr_numbers=open_pr_numbers,
            active_claims=claims,
            relation=relation,
        )
        audited.append(
            {
                "active_claims": claims,
                "branch": branch,
                "disposition": disposition,
                "head_sha": head_sha,
                "open_pr_numbers": open_pr_numbers,
                "protected": item["protected"],
                "pull_history": [pull_summary(pull) for pull in pulls[:20]],
                "reason": reason,
                "relation": relation,
                "reserved_purpose_name": bool(RESERVED_PURPOSE_RE.search(branch)),
            }
        )

    dispositions = Counter(item["disposition"] for item in audited)
    delete_entries = [
        {"branch": item["branch"], "head_sha": item["head_sha"]}
        for item in audited
        if item["disposition"] == "DELETE"
    ]
    manifest = {
        "confirmation": CONFIRMATION,
        "entries": delete_entries,
        "implementation_sha256": implementation_sha256,
        "issue": ISSUE_NUMBER,
        "schema_version": SCHEMA_VERSION,
    }
    report = {
        "branch_count": len(audited),
        "branches": audited,
        "default_branch": default_branch,
        "default_branch_sha": default_sha,
        "deletion_candidate_count": len(delete_entries),
        "disposition_counts": dict(sorted(dispositions.items())),
        "implementation_sha256": implementation_sha256,
        "issue": ISSUE_NUMBER,
        "schema_version": SCHEMA_VERSION,
    }
    return report, manifest


APPROVAL_FIELDS = {
    "apply_on_main",
    "candidate_count",
    "confirmation",
    "entries",
    "entries_sha256",
    "implementation_sha256",
    "issue",
    "review_summary",
    "reviewed_at",
    "reviewed_by",
    "schema_version",
    "source_artifact",
}


def entries_sha256(entries: list[dict[str, Any]]) -> str:
    return sha256_bytes(canonical_json(entries).encode("utf-8"))


def validate_approval_identity(approval: dict[str, Any]) -> None:
    if set(approval) != APPROVAL_FIELDS:
        missing = sorted(APPROVAL_FIELDS - set(approval))
        extra = sorted(set(approval) - APPROVAL_FIELDS)
        raise ValidationError(f"approval schema drift; missing={missing}, extra={extra}")
    if approval.get("schema_version") != SCHEMA_VERSION:
        raise ValidationError("approval schema_version mismatch")
    if approval.get("issue") != ISSUE_NUMBER:
        raise ValidationError("approval issue mismatch")
    if approval.get("confirmation") != CONFIRMATION:
        raise ValidationError("approval confirmation mismatch")
    if approval.get("apply_on_main") is not True:
        raise ValidationError("approval apply_on_main must be true")
    for field in ("entries_sha256", "implementation_sha256", "review_summary", "reviewed_at", "reviewed_by", "source_artifact"):
        value = approval.get(field)
        if not isinstance(value, str) or not value.strip():
            raise ValidationError(f"approval {field} must be non-empty")
    if not isinstance(approval.get("candidate_count"), int) or approval["candidate_count"] < 0:
        raise ValidationError("approval candidate_count must be a non-negative integer")
    entries = approval.get("entries")
    if not isinstance(entries, list):
        raise ValidationError("approval entries must be an array")
    seen: set[str] = set()
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict) or set(entry) != {"branch", "head_sha"}:
            raise ValidationError(f"approval entries[{index}] schema drift")
        branch = entry.get("branch")
        sha = entry.get("head_sha")
        if not isinstance(branch, str) or not branch:
            raise ValidationError(f"approval entries[{index}].branch is invalid")
        if not isinstance(sha, str) or not FULL_SHA_RE.fullmatch(sha):
            raise ValidationError(f"approval entries[{index}].head_sha is invalid")
        if branch in seen:
            raise ValidationError(f"duplicate approved branch {branch}")
        seen.add(branch)
    if approval["candidate_count"] != len(entries):
        raise ValidationError("approval candidate_count does not match approval entries")
    if approval["entries_sha256"] != entries_sha256(entries):
        raise ValidationError("approval entries_sha256 does not match approval entries")


def validate_inventory_approval(
    approval: dict[str, Any], manifest: dict[str, Any]
) -> dict[str, Any]:
    validate_approval_identity(approval)
    entries = manifest.get("entries")
    if not isinstance(entries, list):
        raise ValidationError("manifest entries must be an array")
    digest = entries_sha256(entries)
    if approval["candidate_count"] != len(entries):
        raise ValidationError("approval candidate count drift")
    if approval["entries"] != entries:
        raise ValidationError("approval reviewed entries differ from current inventory")
    if approval["entries_sha256"] != digest:
        raise ValidationError("approval candidate entries drift")
    if approval["implementation_sha256"] != manifest.get("implementation_sha256"):
        raise ValidationError("approval implementation hash drift")
    return {
        "candidate_count": len(entries),
        "entries_sha256": digest,
        "implementation_sha256": manifest.get("implementation_sha256"),
        "issue": ISSUE_NUMBER,
        "result": "PASS",
    }


def validate_apply_candidate_set(
    *,
    approval: dict[str, Any],
    current_manifest: dict[str, Any],
    current_branches: dict[str, str],
) -> tuple[list[dict[str, str]], list[dict[str, str]]]:
    validate_approval_identity(approval)
    if approval["implementation_sha256"] != current_manifest.get("implementation_sha256"):
        raise ValidationError("approval implementation hash drift before apply")

    current_entries_raw = current_manifest.get("entries")
    if not isinstance(current_entries_raw, list):
        raise ValidationError("current manifest entries must be an array")
    current_entries: dict[str, str] = {}
    for entry in current_entries_raw:
        if not isinstance(entry, dict) or set(entry) != {"branch", "head_sha"}:
            raise ValidationError("current manifest entry schema drift")
        branch = entry["branch"]
        sha = entry["head_sha"]
        if not isinstance(branch, str) or not isinstance(sha, str):
            raise ValidationError("current manifest entry values are invalid")
        current_entries[branch] = sha

    approved_entries_raw = approval.get("entries")
    if not isinstance(approved_entries_raw, list):
        raise ValidationError("approved entries are missing")
    approved_entries: dict[str, str] = {}
    for entry in approved_entries_raw:
        if not isinstance(entry, dict) or set(entry) != {"branch", "head_sha"}:
            raise ValidationError("approved entry schema drift")
        branch = entry["branch"]
        sha = entry["head_sha"]
        if not isinstance(branch, str) or not isinstance(sha, str):
            raise ValidationError("approved entry values are invalid")
        if branch in approved_entries:
            raise ValidationError(f"duplicate approved branch {branch}")
        approved_entries[branch] = sha

    if approval["candidate_count"] != len(approved_entries):
        raise ValidationError("approval candidate count does not match bound entries")
    if approval["entries_sha256"] != entries_sha256(approved_entries_raw):
        raise ValidationError("approval bound entry digest mismatch")

    extras = sorted(set(current_entries) - set(approved_entries))
    if extras:
        raise ValidationError(
            f"new unreviewed deletion candidate(s) appeared before apply: {extras}"
        )

    present: list[dict[str, str]] = []
    already_absent: list[dict[str, str]] = []
    for branch, approved_sha in approved_entries.items():
        live_sha = current_branches.get(branch)
        if live_sha is None:
            already_absent.append({"branch": branch, "head_sha": approved_sha})
            continue
        if live_sha != approved_sha:
            raise ValidationError(
                f"approved branch SHA drift for {branch}: expected {approved_sha}, got {live_sha}"
            )
        if current_entries.get(branch) != approved_sha:
            raise ValidationError(
                f"approved branch {branch} is still present but no longer classifies DELETE"
            )
        present.append({"branch": branch, "head_sha": approved_sha})

    return sorted(present, key=lambda item: item["branch"]), sorted(
        already_absent, key=lambda item: item["branch"]
    )


def assert_main_unchanged(
    client: GitHubClient, default_branch: str, expected_sha: str
) -> None:
    current = client.get_branch(default_branch)
    commit = current.get("commit") if isinstance(current, dict) else None
    sha = commit.get("sha") if isinstance(commit, dict) else None
    if sha != expected_sha:
        raise ValidationError(
            f"protected {default_branch} moved during apply: expected {expected_sha}, got {sha}"
        )
    if current.get("protected") is not True:
        raise ValidationError(f"protected {default_branch} lost protection during apply")


def recovery_probe(
    client: GitHubClient, *,
    default_branch_sha: str,
    run_id: str,
) -> dict[str, Any]:
    safe_run_id = re.sub(r"[^0-9A-Za-z_-]", "-", run_id or "local")
    branch = f"historical-audit-recovery-probe-{safe_run_id}"
    existing = client.get_ref(branch)
    if existing is not None:
        raise ValidationError(f"recovery probe ref already exists: {branch}")

    created = False
    cleanup_error: Exception | None = None
    try:
        client.create_branch(branch, default_branch_sha)
        created = True
        created_ref = client.get_ref(branch)
        created_object = created_ref.get("object") if isinstance(created_ref, dict) else None
        created_sha = created_object.get("sha") if isinstance(created_object, dict) else None
        if created_sha != default_branch_sha:
            raise ValidationError(
                f"recovery probe create verification mismatch: expected {default_branch_sha}, got {created_sha}"
            )
        client.delete_branch(branch, expected_sha=default_branch_sha)
        created = False
        if client.get_ref(branch) is not None:
            raise ValidationError("recovery probe delete verification failed")
        return {
            "branch": branch,
            "created_sha": default_branch_sha,
            "delete_verified": True,
            "result": "PASS",
        }
    finally:
        if created:
            try:
                client.delete_branch(branch, expected_sha=default_branch_sha)
                if client.get_ref(branch) is not None:
                    raise ValidationError("recovery probe cleanup verification failed")
            except Exception as exc:
                cleanup_error = exc
        if cleanup_error is not None:
            raise ValidationError(
                f"recovery probe cleanup failed after primary probe error: {cleanup_error}"
            ) from cleanup_error


def apply_reviewed_deletions(
    client: GitHubClient,
    *,
    root: Path,
    default_branch: str,
    expected_main_sha: str,
    script_path: Path,
    approval_path: Path,
    run_id: str,
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    root = validate_repository_root(root)
    approval = load_canonical_json(approval_path)
    validate_approval_identity(approval)
    reviewed_entries = approval["entries"]

    assert_main_unchanged(client, default_branch, expected_main_sha)
    report, current_manifest = build_audit(
        client, root=root, default_branch=default_branch, script_path=script_path
    )
    if report["default_branch_sha"] != expected_main_sha:
        raise ValidationError(
            f"audit default branch SHA mismatch: expected {expected_main_sha}, got {report['default_branch_sha']}"
        )

    current_branches = {
        item["branch"]: item["head_sha"] for item in report["branches"]
    }
    present, already_absent = validate_apply_candidate_set(
        approval=approval,
        current_manifest=current_manifest,
        current_branches=current_branches,
    )

    non_candidate_names = sorted(
        set(current_branches) - {entry["branch"] for entry in reviewed_entries}
    )
    probe_evidence = recovery_probe(
        client,
        default_branch_sha=expected_main_sha,
        run_id=run_id,
    )

    deleted: list[dict[str, str]] = []
    for entry in present:
        branch = entry["branch"]
        sha = entry["head_sha"]
        assert_main_unchanged(client, default_branch, expected_main_sha)

        branch_state = client.get_branch(branch)
        if not isinstance(branch_state, dict):
            raise ValidationError(f"approved branch disappeared before exact delete: {branch}")
        live_commit = branch_state.get("commit")
        live_sha = live_commit.get("sha") if isinstance(live_commit, dict) else None
        if live_sha != sha:
            raise ValidationError(
                f"pre-delete SHA drift for {branch}: expected {sha}, got {live_sha}"
            )
        if branch_state.get("protected") is True:
            raise ValidationError(f"approved branch became protected: {branch}")
        if client.open_pulls_for_branch(branch):
            raise ValidationError(f"approved branch gained an open PR: {branch}")

        client.delete_branch(branch, expected_sha=sha)
        if client.get_ref(branch) is not None:
            raise ValidationError(f"historical branch deletion could not be verified: {branch}")
        deleted.append({"branch": branch, "head_sha": sha})

    assert_main_unchanged(client, default_branch, expected_main_sha)
    post_branches = {item["branch"]: item["head_sha"] for item in list_live_branches(client)}
    remaining_approved = sorted(
        entry["branch"] for entry in reviewed_entries if entry["branch"] in post_branches
    )
    if remaining_approved:
        raise ValidationError(
            f"approved branches remain after apply: {remaining_approved}"
        )
    missing_non_candidates = sorted(
        branch for branch in non_candidate_names if branch not in post_branches
    )
    if missing_non_candidates:
        raise ValidationError(
            "non-candidate branch(es) disappeared during apply: "
            + ", ".join(missing_non_candidates)
        )

    evidence = {
        "already_absent": already_absent,
        "approved_candidate_count": len(reviewed_entries),
        "deleted": deleted,
        "deleted_count": len(deleted),
        "issue": ISSUE_NUMBER,
        "main_sha": expected_main_sha,
        "non_candidate_branches_verified_present": len(non_candidate_names),
        "result": "PASS",
        "schema_version": SCHEMA_VERSION,
    }
    return report, evidence, probe_evidence


def parser() -> argparse.ArgumentParser:
    value = argparse.ArgumentParser(description="Oteryn fail-closed historical branch audit")
    value.add_argument("--mode", choices=("inventory", "apply"), required=True)
    value.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", ""))
    value.add_argument("--token", default=os.environ.get("GITHUB_TOKEN", ""))
    value.add_argument("--root", type=Path, default=Path("."))
    value.add_argument("--default-branch", default="main")
    value.add_argument(
        "--script-path",
        type=Path,
        default=Path("tools/agents/historical_branch_audit.py"),
    )
    value.add_argument("--output", type=Path, required=True)
    value.add_argument("--manifest", type=Path, required=True)
    value.add_argument("--approval", type=Path, default=DEFAULT_APPROVAL_PATH)
    value.add_argument("--approval-validation", type=Path)
    value.add_argument("--evidence", type=Path)
    value.add_argument("--recovery-evidence", type=Path)
    value.add_argument("--event-name", default=os.environ.get("GITHUB_EVENT_NAME", ""))
    value.add_argument("--ref-name", default=os.environ.get("GITHUB_REF_NAME", ""))
    value.add_argument("--expected-main-sha", default=os.environ.get("GITHUB_SHA", ""))
    return value


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    if not args.repo or "/" not in args.repo:
        raise ValidationError("--repo must use owner/name form")
    if not args.token:
        raise ValidationError("GitHub token is required")
    root = args.root.resolve()
    client = GitHubClient(args.repo, args.token, root=root)

    if args.mode == "inventory":
        report, manifest = build_audit(
            client,
            root=root,
            default_branch=args.default_branch,
            script_path=args.script_path,
        )
        write_json(args.output, report)
        write_json(args.manifest, manifest)
        if args.approval.exists():
            approval = load_canonical_json(args.approval)
            validation = validate_inventory_approval(approval, manifest)
            if args.approval_validation is None:
                raise ValidationError(
                    "--approval-validation is required when an approval file exists"
                )
            write_json(args.approval_validation, validation)
        print(
            f"Audited {report['branch_count']} branches; "
            f"{report['deletion_candidate_count']} exact historical deletion candidate(s)."
        )
        return 0

    if args.event_name != "push" or args.ref_name != args.default_branch:
        raise ValidationError("apply is allowed only on a push to the protected default branch")
    if not FULL_SHA_RE.fullmatch(args.expected_main_sha):
        raise ValidationError("--expected-main-sha must be a full 40-character SHA")
    if not args.approval.exists():
        raise ValidationError("apply requires the reviewed approval file")
    if args.evidence is None or args.recovery_evidence is None:
        raise ValidationError("apply requires --evidence and --recovery-evidence")

    report, evidence, probe = apply_reviewed_deletions(
        client,
        root=root,
        default_branch=args.default_branch,
        expected_main_sha=args.expected_main_sha,
        script_path=args.script_path,
        approval_path=args.approval,
        run_id=os.environ.get("GITHUB_RUN_ID", "local"),
    )
    write_json(args.output, report)
    _, current_manifest = build_audit(
        client,
        root=root,
        default_branch=args.default_branch,
        script_path=args.script_path,
    )
    write_json(args.manifest, current_manifest)
    write_json(args.evidence, evidence)
    write_json(args.recovery_evidence, probe)
    print(
        f"Deleted {evidence['deleted_count']} reviewed branch(es); "
        f"{len(evidence['already_absent'])} approved branch(es) were already absent."
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValidationError, ApiError, OSError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
