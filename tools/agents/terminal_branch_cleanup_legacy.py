#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Any

import branch_lifecycle

ISSUE_NUMBER = 1050
SCHEMA_VERSION = 1
MANIFEST_SCHEMA_VERSION = 1
CONFIRMATION = "DELETE_REVIEWED_TERMINAL_CLOSED_UNMERGED_BRANCHES_ISSUE_1050"
DEFAULT_MANIFEST_PATH = Path("docs/agents/TERMINAL_BRANCH_DELETION_APPROVAL.json")
DELETE_DISPOSITION_RE = re.compile(r"^\s*Branch-Disposition:\s*delete\s*$", re.I | re.M)
RETAIN_DISPOSITION_RE = re.compile(r"^\s*Branch-Disposition:\s*retain\s*$", re.I | re.M)
REASON_RE = re.compile(r"^\s*Branch-Disposition-Reason:\s*(\S.*)\s*$", re.I | re.M)


ValidationError = branch_lifecycle.ValidationError
ApiError = branch_lifecycle.ApiError
GitHubClient = branch_lifecycle.GitHubClient


def _pull_number(pull: dict[str, Any]) -> int | None:
    value = pull.get("number")
    return value if isinstance(value, int) and not isinstance(value, bool) else None


def _closed_at(pull: dict[str, Any]) -> str | None:
    value = pull.get("closed_at")
    return value if isinstance(value, str) and value else None


def _exact_closed_unmerged_prs(
    pulls: list[dict[str, Any]], repo: str, branch: str, sha: str
) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    for pull in pulls:
        if not isinstance(pull, dict):
            continue
        if not branch_lifecycle._same_repo_pull(pull, repo):
            continue
        if branch_lifecycle._pull_branch(pull) != branch:
            continue
        if branch_lifecycle._pull_sha(pull) != sha:
            continue
        if pull.get("state") != "closed" or pull.get("merged_at") is not None:
            continue
        if _pull_number(pull) is None or _closed_at(pull) is None:
            continue
        result.append(pull)
    result.sort(key=lambda item: (_closed_at(item) or "", _pull_number(item) or 0), reverse=True)
    return result


def classify_snapshot(
    policy: dict[str, Any], snapshot: dict[str, Any], *, root: Path
) -> dict[str, Any]:
    base = branch_lifecycle.classify_snapshot(policy, snapshot, root=root)
    pulls_raw = snapshot.get("pulls")
    if not isinstance(pulls_raw, list):
        raise ValidationError("snapshot.pulls must be an array")
    pulls = [item for item in pulls_raw if isinstance(item, dict)]
    repo = str(base["repository"])

    output: list[dict[str, Any]] = []
    for raw in base["branches"]:
        item = dict(raw)
        item["deletion_candidate"] = False
        item["closed_unmerged_pr"] = None
        if item["classification"] == "UNMERGED_ORPHAN":
            exact = _exact_closed_unmerged_prs(
                pulls, repo, item["branch"], item["head_sha"]
            )
            if len(exact) == 1:
                matched = exact[0]
                item["classification"] = "TERMINAL_CLOSED_UNMERGED"
                item["deletion_candidate"] = True
                item["closed_unmerged_pr"] = {
                    "closed_at": _closed_at(matched),
                    "number": _pull_number(matched),
                    "url": matched.get("html_url"),
                }
                item["evidence"] = [
                    f"closed unmerged pull request #{_pull_number(matched)} has the exact current branch head SHA",
                    "no open pull request, active task/open deterministic Issue, protection, retention exception or reserved recovery-sensitive name was accepted by the base classifier",
                ]
            elif len(exact) > 1:
                item["evidence"] = [
                    "multiple closed unmerged pull requests match the exact current branch head SHA; explicit PR identity is required before cleanup"
                ]
        output.append(item)

    output.sort(key=lambda entry: entry["branch"])
    counts = dict(sorted(Counter(entry["classification"] for entry in output).items()))
    return {
        "schema_version": SCHEMA_VERSION,
        "repository": base["repository"],
        "default_branch": base["default_branch"],
        "default_branch_sha": base["default_branch_sha"],
        "generated_at": base["generated_at"],
        "policy_sha256": base["policy_sha256"],
        "counts": counts,
        "deletion_candidate_count": sum(1 for entry in output if entry["deletion_candidate"]),
        "branches": output,
    }


def make_manifest(report: dict[str, Any]) -> dict[str, Any]:
    entries: list[dict[str, Any]] = []
    for item in report["branches"]:
        if not item["deletion_candidate"]:
            continue
        pr = item["closed_unmerged_pr"]
        if not isinstance(pr, dict):
            raise ValidationError(f"candidate {item['branch']} lacks closed PR evidence")
        entries.append({
            "branch": item["branch"],
            "closed_at": pr["closed_at"],
            "closed_pr_number": pr["number"],
            "head_sha": item["head_sha"],
        })
    entries.sort(key=lambda entry: entry["branch"])
    return {
        "apply_on_main": False,
        "confirmation": CONFIRMATION,
        "entries": entries,
        "inventory_default_branch_sha": report["default_branch_sha"],
        "issue": ISSUE_NUMBER,
        "policy_sha256": report["policy_sha256"],
        "schema_version": MANIFEST_SCHEMA_VERSION,
    }


def validate_manifest(
    manifest: dict[str, Any], report: dict[str, Any], *, require_apply: bool
) -> list[dict[str, Any]]:
    branch_lifecycle._exact_fields(
        manifest,
        {
            "apply_on_main", "confirmation", "entries", "inventory_default_branch_sha",
            "issue", "policy_sha256", "schema_version",
        },
        "terminal manifest",
    )
    if manifest["schema_version"] != MANIFEST_SCHEMA_VERSION:
        raise ValidationError("terminal manifest.schema_version: unsupported")
    if manifest["issue"] != ISSUE_NUMBER:
        raise ValidationError(f"terminal manifest.issue: expected {ISSUE_NUMBER}")
    if manifest["confirmation"] != CONFIRMATION:
        raise ValidationError("terminal manifest.confirmation: exact phrase required")
    if manifest["policy_sha256"] != report["policy_sha256"]:
        raise ValidationError("terminal manifest.policy_sha256: policy drift detected")
    if require_apply and manifest["apply_on_main"] is not True:
        raise ValidationError("terminal manifest.apply_on_main must be true for apply mode")
    if not isinstance(manifest["entries"], list):
        raise ValidationError("terminal manifest.entries: expected array")

    live = {
        item["branch"]: item for item in report["branches"]
        if item["classification"] == "TERMINAL_CLOSED_UNMERGED"
    }
    validated: list[dict[str, Any]] = []
    seen: set[str] = set()
    for index, raw in enumerate(manifest["entries"]):
        entry = branch_lifecycle._exact_fields(
            raw,
            {"branch", "closed_at", "closed_pr_number", "head_sha"},
            f"terminal manifest.entries[{index}]",
        )
        branch = branch_lifecycle._text(entry["branch"], f"terminal manifest.entries[{index}].branch")
        if branch in seen:
            raise ValidationError(f"terminal manifest.entries: duplicate branch {branch}")
        seen.add(branch)
        current = live.get(branch)
        if current is None:
            raise ValidationError(
                f"terminal manifest.entries[{index}]: {branch} is no longer TERMINAL_CLOSED_UNMERGED"
            )
        pr = current["closed_unmerged_pr"]
        expected = {
            "branch": branch,
            "closed_at": pr["closed_at"],
            "closed_pr_number": pr["number"],
            "head_sha": current["head_sha"],
        }
        if entry != expected:
            raise ValidationError(
                f"terminal manifest.entries[{index}]: live branch/PR evidence drift for {branch}"
            )
        validated.append(expected)
    if [entry["branch"] for entry in validated] != sorted(entry["branch"] for entry in validated):
        raise ValidationError("terminal manifest.entries must be ordered by branch")
    return validated


def _revalidate_delete_entry(
    client: GitHubClient,
    policy: dict[str, Any],
    report: dict[str, Any],
    entry: dict[str, Any],
    *,
    root: Path,
) -> None:
    exceptions = branch_lifecycle.validate_policy(policy, root)
    branch = branch_lifecycle._text(entry.get("branch"), "terminal delete entry.branch")
    expected_sha = branch_lifecycle._text(entry.get("head_sha"), f"terminal delete {branch}.head_sha")
    default_branch = branch_lifecycle._text(report.get("default_branch"), "terminal report.default_branch")
    default_sha = branch_lifecycle._text(report.get("default_branch_sha"), "terminal report.default_branch_sha")

    if branch == default_branch:
        raise ValidationError("terminal delete default branch refusal")
    if branch in exceptions:
        raise ValidationError(f"terminal delete retention exception exists for {branch}")
    marker = branch_lifecycle._reserved_name(branch)
    if marker:
        raise ValidationError(f"terminal delete reserved branch refused: {branch} ({marker})")

    live_branch = client.get_branch(branch)
    if live_branch is None:
        raise ValidationError(f"terminal delete branch disappeared: {branch}")
    if branch_lifecycle._branch_sha(live_branch) != expected_sha:
        raise ValidationError(f"terminal delete SHA drift for {branch}")
    if live_branch.get("protected") is not False:
        raise ValidationError(f"terminal delete protected/ambiguous branch refused: {branch}")
    if client.open_pulls_for_branch(branch):
        raise ValidationError(f"terminal delete open pull request appeared for {branch}")
    if branch in branch_lifecycle.active_task_branches(root):
        raise ValidationError(f"terminal delete active task claim appeared for {branch}")

    issue_match = branch_lifecycle.REPAIR_ISSUE_RE.fullmatch(branch)
    if issue_match:
        issue_state = client.get_issue_state(int(issue_match.group(1)))
        if issue_state != "closed":
            raise ValidationError(
                f"terminal delete remediation Issue #{issue_match.group(1)} is {issue_state}"
            )

    if branch_lifecycle._ref_sha(client.get_ref(default_branch)) != default_sha:
        raise ValidationError("terminal delete default branch drift detected")
    if branch_lifecycle._ref_sha(client.get_ref(branch)) != expected_sha:
        raise ValidationError(f"terminal delete final SHA drift for {branch}")


def apply_manifest(
    client: GitHubClient,
    policy: dict[str, Any],
    report: dict[str, Any],
    manifest: dict[str, Any],
    *,
    root: Path,
    evidence_path: Path,
    event_name: str,
    ref_name: str,
) -> None:
    if event_name != "push" or ref_name != "main":
        raise ValidationError("terminal apply is allowed only on a push to main")
    entries = validate_manifest(manifest, report, require_apply=True)
    if not entries:
        raise ValidationError("terminal apply manifest contains no reviewed entries")
    deleted: list[dict[str, Any]] = []
    for entry in entries:
        _revalidate_delete_entry(client, policy, report, entry, root=root)
        client.delete_branch(entry["branch"], expected_sha=entry["head_sha"])
        if client.get_ref(entry["branch"]) is not None:
            raise ValidationError(f"terminal branch deletion could not be verified: {entry['branch']}")
        deleted.append(entry)
    evidence = {
        "applied_at": dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat(),
        "default_branch_sha": report["default_branch_sha"],
        "deleted": deleted,
        "issue": ISSUE_NUMBER,
        "policy_sha256": report["policy_sha256"],
        "repository": report["repository"],
        "schema_version": 1,
    }
    evidence_path.parent.mkdir(parents=True, exist_ok=True)
    evidence_path.write_text(branch_lifecycle.canonical_json(evidence), encoding="utf-8")


def recovery_test(client: GitHubClient, default_sha: str, run_id: str, evidence_path: Path) -> None:
    safe = re.sub(r"[^A-Za-z0-9_.-]+", "-", run_id).strip("-") or "manual"
    branch = f"recovery-test/issue-{ISSUE_NUMBER}-{safe}"
    if client.get_ref(branch) is not None:
        raise ValidationError(f"terminal recovery test branch already exists: {branch}")
    client.create_branch(branch, default_sha)
    client.delete_branch(branch, expected_sha=default_sha)
    client.create_branch(branch, default_sha)
    if branch_lifecycle._ref_sha(client.get_ref(branch)) != default_sha:
        raise ValidationError("terminal recovery test restore mismatch")
    client.delete_branch(branch, expected_sha=default_sha)
    if client.get_ref(branch) is not None:
        raise ValidationError("terminal recovery test cleanup failed")
    evidence_path.write_text(branch_lifecycle.canonical_json({
        "branch": branch,
        "issue": ISSUE_NUMBER,
        "result": "PASS",
        "schema_version": 1,
        "sha": default_sha,
        "tested_at": dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat(),
    }), encoding="utf-8")


def _event_disposition(body: str) -> tuple[str | None, str | None]:
    delete = bool(DELETE_DISPOSITION_RE.search(body))
    retain = bool(RETAIN_DISPOSITION_RE.search(body))
    if delete and retain:
        raise ValidationError("pull request has conflicting Branch-Disposition markers")
    reason_match = REASON_RE.search(body)
    reason = reason_match.group(1).strip() if reason_match else None
    if delete:
        if not reason:
            raise ValidationError("Branch-Disposition: delete requires Branch-Disposition-Reason")
        return "delete", reason
    if retain:
        if not reason:
            raise ValidationError("Branch-Disposition: retain requires Branch-Disposition-Reason")
        return "retain", reason
    return None, None


def apply_closed_pr_event(
    client: GitHubClient,
    policy: dict[str, Any],
    report: dict[str, Any],
    event: dict[str, Any],
    *,
    root: Path,
    evidence_path: Path,
) -> None:
    pull = event.get("pull_request")
    if not isinstance(pull, dict):
        raise ValidationError("closed-PR event lacks pull_request object")
    if pull.get("state") != "closed" or pull.get("merged") is True or pull.get("merged_at") is not None:
        raise ValidationError("closed-PR cleanup accepts only unmerged closed pull requests")
    body = pull.get("body") if isinstance(pull.get("body"), str) else ""
    disposition, reason = _event_disposition(body)
    number = _pull_number(pull)
    branch = branch_lifecycle._pull_branch(pull)
    sha = branch_lifecycle._pull_sha(pull)
    repo = report["repository"]
    if not branch_lifecycle._same_repo_pull(pull, repo):
        raise ValidationError("closed-PR cleanup refuses foreign/fork source branches")
    if not isinstance(number, int) or not branch or not sha:
        raise ValidationError("closed-PR cleanup lacks exact PR number/branch/SHA")

    evidence = {
        "branch": branch,
        "disposition": disposition or "unspecified",
        "issue": ISSUE_NUMBER,
        "pr": number,
        "reason": reason,
        "schema_version": 1,
        "sha": sha,
    }
    if disposition in {None, "retain"}:
        evidence["result"] = "NO_DELETE"
        evidence_path.write_text(branch_lifecycle.canonical_json(evidence), encoding="utf-8")
        return

    current = next((item for item in report["branches"] if item["branch"] == branch), None)
    if not isinstance(current, dict) or current["classification"] != "TERMINAL_CLOSED_UNMERGED":
        raise ValidationError(f"closed-PR cleanup branch is not safely terminal: {branch}")
    pr = current["closed_unmerged_pr"]
    if current["head_sha"] != sha or not isinstance(pr, dict) or pr["number"] != number:
        raise ValidationError("closed-PR cleanup exact event/live evidence mismatch")
    entry = {
        "branch": branch,
        "closed_at": pr["closed_at"],
        "closed_pr_number": number,
        "head_sha": sha,
    }
    _revalidate_delete_entry(client, policy, report, entry, root=root)
    client.delete_branch(branch, expected_sha=sha)
    if client.get_ref(branch) is not None:
        raise ValidationError("closed-PR cleanup deletion verification failed")
    evidence["result"] = "DELETED"
    evidence_path.write_text(branch_lifecycle.canonical_json(evidence), encoding="utf-8")


def parser() -> argparse.ArgumentParser:
    value = argparse.ArgumentParser(description="Oteryn terminal closed-unmerged branch cleanup")
    value.add_argument("--mode", choices=("inventory", "validate-manifest", "apply", "event"), required=True)
    value.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", ""))
    value.add_argument("--token", default=os.environ.get("GITHUB_TOKEN", ""))
    value.add_argument("--root", type=Path, default=Path("."))
    value.add_argument("--policy", type=Path, default=branch_lifecycle.POLICY_PATH)
    value.add_argument("--snapshot", type=Path)
    value.add_argument("--output", type=Path, default=Path("terminal-branch-report.json"))
    value.add_argument("--generate-manifest", type=Path)
    value.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST_PATH)
    value.add_argument("--evidence", type=Path, default=Path("terminal-branch-deletion-evidence.json"))
    value.add_argument("--recovery-evidence", type=Path)
    value.add_argument("--event", type=Path, default=Path(os.environ.get("GITHUB_EVENT_PATH", "")))
    value.add_argument("--event-name", default=os.environ.get("GITHUB_EVENT_NAME", ""))
    value.add_argument("--ref-name", default=os.environ.get("GITHUB_REF_NAME", ""))
    value.add_argument("--run-id", default=os.environ.get("GITHUB_RUN_ID", "manual"))
    return value


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    root = args.root.resolve()
    policy_path = args.policy if args.policy.is_absolute() else root / args.policy
    policy = branch_lifecycle.load_json(policy_path)
    branch_lifecycle.validate_policy(policy, root)

    if args.snapshot is not None:
        snapshot_path = args.snapshot if args.snapshot.is_absolute() else root / args.snapshot
        snapshot = branch_lifecycle.load_json(snapshot_path)
        client = None
    else:
        if not args.repo or not args.token:
            raise ValidationError("--repo and --token (or GitHub environment variables) are required")
        client = GitHubClient(args.repo, args.token, root=root)
        snapshot = branch_lifecycle.fetch_live_snapshot(client, root)
    report = classify_snapshot(policy, snapshot, root=root)

    output_path = args.output if args.output.is_absolute() else root / args.output
    output_path.write_text(branch_lifecycle.canonical_json(report), encoding="utf-8")
    if args.generate_manifest is not None:
        manifest_path = args.generate_manifest if args.generate_manifest.is_absolute() else root / args.generate_manifest
        manifest_path.write_text(branch_lifecycle.canonical_json(make_manifest(report)), encoding="utf-8")

    if args.mode == "inventory":
        print(f"Inventoried {len(report['branches'])} branches; {report['deletion_candidate_count']} terminal closed-unmerged candidates.")
        return 0

    if args.mode == "event":
        if client is None:
            raise ValidationError("event mode cannot use an offline snapshot")
        event_path = args.event if args.event.is_absolute() else root / args.event
        event = json.loads(event_path.read_text(encoding="utf-8"))
        evidence_path = args.evidence if args.evidence.is_absolute() else root / args.evidence
        apply_closed_pr_event(client, policy, report, event, root=root, evidence_path=evidence_path)
        print("Processed terminal closed-PR branch disposition.")
        return 0

    manifest_path = args.manifest if args.manifest.is_absolute() else root / args.manifest
    manifest = branch_lifecycle.load_json(manifest_path)
    if args.mode == "validate-manifest":
        entries = validate_manifest(manifest, report, require_apply=False)
        print(f"Validated {len(entries)} terminal branch manifest entries.")
        return 0

    if client is None:
        raise ValidationError("apply mode cannot use an offline snapshot")
    evidence_path = args.evidence if args.evidence.is_absolute() else root / args.evidence
    apply_manifest(
        client, policy, report, manifest, root=root, evidence_path=evidence_path,
        event_name=args.event_name, ref_name=args.ref_name,
    )
    if args.recovery_evidence is not None:
        recovery_path = args.recovery_evidence if args.recovery_evidence.is_absolute() else root / args.recovery_evidence
        recovery_test(client, report["default_branch_sha"], args.run_id, recovery_path)
    print("Applied reviewed terminal closed-unmerged branch manifest.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValidationError, ApiError, json.JSONDecodeError, OSError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
