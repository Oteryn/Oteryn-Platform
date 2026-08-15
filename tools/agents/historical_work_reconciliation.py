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

import historical_branch_policy as policy
from branch_lifecycle import ApiError, GitHubClient, ValidationError

ISSUE_NUMBER = 1072
SCHEMA_VERSION = 1
SENSITIVE_WORKFLOW_RE = re.compile(
    r"(?:\btags(?:-ignore)?\s*:|refs/tags/|\brelease\b|\bdeploy(?:ment)?\b|"
    r"\bpublication\b|github\.ref(?:_name)?\b|github\.ref_type\b)",
    re.IGNORECASE,
)


def canonical_json(value: object) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(canonical_json(value), encoding="utf-8")


def run_git(
    root: Path,
    args: list[str],
    *,
    purpose: str,
    timeout: int = 180,
    allow: tuple[int, ...] = (0,),
) -> subprocess.CompletedProcess[str]:
    return policy.audit.run_git(
        root, args, purpose=purpose, timeout=timeout, allow=allow
    )


def git_lines(root: Path, args: list[str], *, purpose: str) -> list[str]:
    result = run_git(root, args, purpose=purpose)
    return [line for line in result.stdout.splitlines() if line.strip()]


def merge_base(root: Path, main_sha: str, head_sha: str) -> str:
    value = run_git(
        root,
        ["merge-base", main_sha, head_sha],
        purpose=f"merge-base {head_sha}",
    ).stdout.strip()
    if not policy.audit.FULL_SHA_RE.fullmatch(value):
        raise ValidationError(f"merge-base for {head_sha} is invalid: {value!r}")
    return value


def unique_commits(root: Path, main_sha: str, head_sha: str) -> list[dict[str, Any]]:
    lines = git_lines(
        root,
        [
            "log",
            "--reverse",
            "--no-decorate",
            "--format=%H%x09%P%x09%ct%x09%s",
            f"{main_sha}..{head_sha}",
        ],
        purpose=f"unique commit log for {head_sha}",
    )
    result: list[dict[str, Any]] = []
    for raw in lines:
        parts = raw.split("\t", 3)
        if len(parts) != 4:
            raise ValidationError(f"unexpected unique commit log row for {head_sha}: {raw!r}")
        sha, parents, timestamp, subject = parts
        result.append(
            {
                "parents": [item for item in parents.split() if item],
                "sha": sha,
                "subject": subject,
                "unix_time": int(timestamp),
            }
        )
    return result


def cherry_status(root: Path, main_sha: str, head_sha: str) -> dict[str, Any]:
    lines = git_lines(
        root,
        ["cherry", main_sha, head_sha],
        purpose=f"patch equivalence for {head_sha}",
    )
    entries: list[dict[str, str]] = []
    for raw in lines:
        marker, sep, sha = raw.partition(" ")
        if sep != " " or marker not in {"+", "-"} or not policy.audit.FULL_SHA_RE.fullmatch(sha):
            raise ValidationError(f"unexpected git cherry row for {head_sha}: {raw!r}")
        entries.append({"patch_status": marker, "sha": sha})
    return {
        "entries": entries,
        "equivalent_patch_count": sum(item["patch_status"] == "-" for item in entries),
        "unique_patch_count": sum(item["patch_status"] == "+" for item in entries),
    }


def diff_paths(root: Path, base_sha: str, head_sha: str) -> list[dict[str, Any]]:
    status_lines = git_lines(
        root,
        ["diff", "--name-status", "--find-renames", f"{base_sha}..{head_sha}"],
        purpose=f"path delta for {head_sha}",
    )
    numstat_lines = git_lines(
        root,
        ["diff", "--numstat", "--find-renames", f"{base_sha}..{head_sha}"],
        purpose=f"numstat delta for {head_sha}",
    )
    numstats: dict[str, tuple[str, str]] = {}
    for raw in numstat_lines:
        parts = raw.split("\t")
        if len(parts) >= 3:
            path = parts[-1]
            numstats[path] = (parts[0], parts[1])

    result: list[dict[str, Any]] = []
    for raw in status_lines:
        parts = raw.split("\t")
        if len(parts) < 2:
            raise ValidationError(f"unexpected name-status row for {head_sha}: {raw!r}")
        status = parts[0]
        if status.startswith(("R", "C")) and len(parts) >= 3:
            old_path, path = parts[1], parts[2]
        else:
            old_path, path = None, parts[1]
        additions, deletions = numstats.get(path, ("?", "?"))
        result.append(
            {
                "additions": additions,
                "deletions": deletions,
                "old_path": old_path,
                "path": path,
                "status": status,
            }
        )
    return result


def object_sha_for_path(root: Path, revision: str, path: str) -> str | None:
    result = run_git(
        root,
        ["rev-parse", "--verify", f"{revision}:{path}"],
        purpose=f"path object {revision}:{path}",
        allow=(0, 128),
        timeout=30,
    )
    if result.returncode != 0:
        return None
    value = result.stdout.strip()
    return value if value else None


def main_path_equivalence(
    root: Path,
    *,
    main_sha: str,
    head_sha: str,
    paths: list[dict[str, Any]],
) -> dict[str, Any]:
    entries: list[dict[str, Any]] = []
    for item in paths:
        path = str(item["path"])
        head_object = object_sha_for_path(root, head_sha, path)
        main_object = object_sha_for_path(root, main_sha, path)
        if head_object is None and main_object is None:
            state = "ABSENT_BOTH"
        elif head_object is None:
            state = "HEAD_ABSENT_MAIN_PRESENT"
        elif main_object is None:
            state = "HEAD_PRESENT_MAIN_ABSENT"
        elif head_object == main_object:
            state = "EXACT_OBJECT_EQUAL"
        else:
            state = "DIFFERENT"
        entries.append(
            {
                "head_object": head_object,
                "main_object": main_object,
                "path": path,
                "state": state,
            }
        )
    counts = Counter(item["state"] for item in entries)
    return {"counts": dict(sorted(counts.items())), "entries": entries}


def refs_containing(root: Path, head_sha: str) -> list[str]:
    lines = git_lines(
        root,
        [
            "for-each-ref",
            "--contains",
            head_sha,
            "--format=%(refname)",
            "refs/remotes/origin",
            "refs/heads",
            "refs/tags",
        ],
        purpose=f"reachable ref scan for {head_sha}",
    )
    return sorted(set(lines))


def pull_provenance(item: dict[str, Any]) -> dict[str, Any]:
    pulls = item.get("pull_history")
    if not isinstance(pulls, list):
        pulls = []
    exact = [
        pull
        for pull in pulls
        if isinstance(pull, dict) and pull.get("head_sha") == item.get("head_sha")
    ]
    return {
        "exact_head_match": exact,
        "exact_head_match_count": len(exact),
        "pull_count": len(pulls),
    }


def workflow_safety_inventory(root: Path) -> dict[str, Any]:
    workflow_root = root / ".github" / "workflows"
    entries: list[dict[str, Any]] = []
    if not workflow_root.is_dir():
        raise ValidationError(".github/workflows is missing")
    for path in sorted(list(workflow_root.glob("*.yml")) + list(workflow_root.glob("*.yaml"))):
        raw = path.read_text(encoding="utf-8")
        lines = raw.splitlines()
        sensitive = [
            {"line": index, "text": line.strip()}
            for index, line in enumerate(lines, 1)
            if SENSITIVE_WORKFLOW_RE.search(line)
        ]
        entries.append(
            {
                "path": path.as_posix(),
                "ref_release_sensitive_lines": sensitive,
                "sha256": hashlib.sha256(raw.encode("utf-8")).hexdigest(),
            }
        )
    if not entries:
        raise ValidationError("workflow inventory is empty")
    return {
        "workflow_count": len(entries),
        "workflows": entries,
    }


def enrich_branch(
    root: Path,
    *,
    main_sha: str,
    item: dict[str, Any],
) -> dict[str, Any]:
    head_sha = str(item["head_sha"])
    base_sha = merge_base(root, main_sha, head_sha)
    paths = diff_paths(root, base_sha, head_sha)
    commits = unique_commits(root, main_sha, head_sha)
    cherry = cherry_status(root, main_sha, head_sha)
    result = dict(item)
    result["content_evidence"] = {
        "changed_path_count": len(paths),
        "changed_paths": paths,
        "main_path_equivalence": main_path_equivalence(
            root,
            main_sha=main_sha,
            head_sha=head_sha,
            paths=paths,
        ),
        "merge_base": base_sha,
        "refs_containing_head": refs_containing(root, head_sha),
        "unique_commit_count": len(commits),
        "unique_commits": commits,
        "patch_equivalence": cherry,
    }
    result["provenance_evidence"] = pull_provenance(item)
    return result


def build_inventory(
    client: GitHubClient,
    *,
    root: Path,
    default_branch: str,
) -> dict[str, Any]:
    root = policy.audit.validate_repository_root(root)
    base_report, _ = policy.build_policy_audit(
        client,
        root=root,
        default_branch=default_branch,
    )
    main_sha = str(base_report["default_branch_sha"])
    enriched = [
        enrich_branch(root, main_sha=main_sha, item=item)
        for item in base_report["branches"]
    ]
    return {
        "branch_count": len(enriched),
        "branches": enriched,
        "default_branch": default_branch,
        "default_branch_sha": main_sha,
        "issue": ISSUE_NUMBER,
        "legacy_disposition_counts": dict(
            sorted(Counter(str(item["disposition"]) for item in enriched).items())
        ),
        "schema_version": SCHEMA_VERSION,
        "workflow_safety": workflow_safety_inventory(root),
    }


def parser() -> argparse.ArgumentParser:
    value = argparse.ArgumentParser(
        description="Content/provenance inventory for Issue #1072 historical work reconciliation"
    )
    value.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", ""))
    value.add_argument("--token", default=os.environ.get("GITHUB_TOKEN", ""))
    value.add_argument("--root", type=Path, default=Path("."))
    value.add_argument("--default-branch", default="main")
    value.add_argument("--output", type=Path, required=True)
    return value


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    if not args.repo or "/" not in args.repo:
        raise ValidationError("--repo must use owner/name form")
    if not args.token:
        raise ValidationError("GitHub token is required")
    root = args.root.resolve()
    client = GitHubClient(args.repo, args.token, root=root)
    report = build_inventory(
        client,
        root=root,
        default_branch=args.default_branch,
    )
    write_json(args.output, report)
    print(
        f"historical work inventory PASS: {report['branch_count']} live refs, "
        f"{report['workflow_safety']['workflow_count']} workflows inspected"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValidationError, ApiError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
