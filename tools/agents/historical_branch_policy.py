#!/usr/bin/env python3
from __future__ import annotations

import argparse
import copy
import hashlib
import os
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Callable

import historical_branch_audit as audit
from branch_lifecycle import ApiError, GitHubClient, ValidationError

POLICY_FILES = (
    Path("tools/agents/historical_branch_audit.py"),
    Path("tools/agents/historical_branch_policy.py"),
)
HARD_ANCHOR_DISPOSITIONS = {"OPEN_PR", "PROTECTED", "RECOVERY"}
LIVE_ANCHOR_PREFIX = "REACHABLE_FROM_LIVE_ANCHOR:"
DUPLICATE_ANCHOR_PREFIX = "DUPLICATE_HEAD_RETAINED_AS:"
BASE_MAIN_PROOFS = {
    "ANCESTOR_OF_MAIN",
    "TREE_EQUAL_TO_MAIN",
    "PATCH_EQUIVALENT_TO_MAIN",
}


def combined_implementation_sha256(root: Path) -> str:
    digest = hashlib.sha256()
    for relative in POLICY_FILES:
        path = root / relative
        digest.update(relative.as_posix().encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def is_ancestor(root: Path, ancestor_sha: str, descendant_sha: str) -> bool:
    if ancestor_sha == descendant_sha:
        return True
    result = audit.run_git(
        root,
        ["merge-base", "--is-ancestor", ancestor_sha, descendant_sha],
        timeout=60,
        allow=(0, 1),
        purpose=f"historical ref coverage {ancestor_sha} -> {descendant_sha}",
    )
    return result.returncode == 0


def anchor_rank(branch: dict[str, Any]) -> tuple[int, int, str]:
    name = str(branch.get("branch", ""))
    disposition = str(branch.get("disposition", ""))
    hard_rank = 0 if disposition in HARD_ANCHOR_DISPOSITIONS else 1
    return (hard_rank, len(name), name)


def collapse_redundant_refs(
    branches: list[dict[str, Any]],
    *,
    ancestor_predicate: Callable[[str, str], bool],
) -> list[dict[str, Any]]:
    result = copy.deepcopy(branches)
    names = [str(item["branch"]) for item in result]
    if len(names) != len(set(names)):
        raise ValidationError("historical policy input contains duplicate branch identities")

    # Active task claims are semantic anchors even though the base classifier labels them RETAIN.
    hard_anchors = [
        item
        for item in result
        if item.get("disposition") in HARD_ANCHOR_DISPOSITIONS
        or bool(item.get("active_claims"))
    ]
    generic = [
        item
        for item in result
        if item.get("disposition") == "RETAIN" and not item.get("active_claims")
    ]

    # A generic historical ref is redundant only when every one of its commits remains
    # reachable from a semantically protected live anchor: an open PR, protected branch,
    # active task branch, or deliberately preserved recovery/rollback/backup ref.
    for item in generic:
        head_sha = str(item["head_sha"])
        covering = [
            anchor
            for anchor in hard_anchors
            if ancestor_predicate(head_sha, str(anchor["head_sha"]))
        ]
        if not covering:
            continue
        anchor = sorted(covering, key=anchor_rank)[0]
        item["disposition"] = "DELETE"
        item["reason"] = (
            f"all commits remain reachable from live {anchor['disposition']} anchor "
            f"{anchor['branch']}@{anchor['head_sha']}"
        )
        item["cleanup_proof"] = f"{LIVE_ANCHOR_PREFIX}{anchor['branch']}"

    # Collapse exact duplicate generic refs while preserving one deterministic retained
    # semantic label. This never removes the last live ref that reaches the unique commit.
    remaining = [item for item in generic if item.get("disposition") == "RETAIN"]
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for item in remaining:
        groups[str(item["head_sha"])].append(item)
    for head_sha, group in groups.items():
        if len(group) < 2:
            continue
        anchor = sorted(
            group,
            key=lambda item: (len(str(item["branch"])), str(item["branch"])),
        )[0]
        for item in group:
            if item is anchor:
                continue
            item["disposition"] = "DELETE"
            item["reason"] = (
                f"exact duplicate head {head_sha} remains reachable from retained ref "
                f"{anchor['branch']}"
            )
            item["cleanup_proof"] = f"{DUPLICATE_ANCHOR_PREFIX}{anchor['branch']}"

    # Base DELETE entries already carry one of the strict Git-to-main proofs.
    for item in result:
        if item.get("disposition") != "DELETE":
            continue
        if "cleanup_proof" not in item:
            relation = item.get("relation") if isinstance(item.get("relation"), dict) else {}
            proof = str(relation.get("proof", ""))
            if proof not in BASE_MAIN_PROOFS:
                raise ValidationError(
                    f"base DELETE {item.get('branch')} lacks an accepted main-incorporation proof"
                )
            item["cleanup_proof"] = proof

    if names != [str(item["branch"]) for item in sorted(result, key=lambda x: names.index(str(x["branch"])))]:
        raise ValidationError("historical policy transform changed branch identity accounting")
    return sorted(result, key=lambda item: str(item["branch"]))


def build_policy_audit(
    client: GitHubClient,
    *,
    root: Path,
    default_branch: str,
) -> tuple[dict[str, Any], dict[str, Any]]:
    root = audit.validate_repository_root(root)
    base_report, _ = audit.build_audit(
        client,
        root=root,
        default_branch=default_branch,
        script_path=Path("tools/agents/historical_branch_audit.py"),
    )
    implementation_sha256 = combined_implementation_sha256(root)
    branches = collapse_redundant_refs(
        base_report["branches"],
        ancestor_predicate=lambda ancestor, descendant: is_ancestor(root, ancestor, descendant),
    )
    counts = Counter(str(item["disposition"]) for item in branches)
    delete_entries = [
        {"branch": str(item["branch"]), "head_sha": str(item["head_sha"])}
        for item in branches
        if item.get("disposition") == "DELETE"
    ]
    report = dict(base_report)
    report["branches"] = branches
    report["branch_count"] = len(branches)
    report["deletion_candidate_count"] = len(delete_entries)
    report["disposition_counts"] = dict(sorted(counts.items()))
    report["implementation_sha256"] = implementation_sha256
    report["policy_layers"] = [path.as_posix() for path in POLICY_FILES]
    manifest = {
        "confirmation": audit.CONFIRMATION,
        "entries": delete_entries,
        "implementation_sha256": implementation_sha256,
        "issue": audit.ISSUE_NUMBER,
        "schema_version": audit.SCHEMA_VERSION,
    }
    return report, manifest


def fetch_exact_live_branch(
    client: GitHubClient,
    *,
    branch: str,
    expected_sha: str,
) -> dict[str, Any]:
    state = client.get_branch(branch)
    if not isinstance(state, dict):
        raise ValidationError(f"required live branch disappeared: {branch}")
    commit = state.get("commit")
    live_sha = commit.get("sha") if isinstance(commit, dict) else None
    if live_sha != expected_sha:
        raise ValidationError(
            f"required live branch SHA drift for {branch}: expected {expected_sha}, got {live_sha}"
        )
    return state


def verify_candidate_proof_live(
    client: GitHubClient,
    *,
    root: Path,
    candidate: dict[str, Any],
    branch_index: dict[str, dict[str, Any]],
    main_sha: str,
) -> None:
    branch = str(candidate["branch"])
    head_sha = str(candidate["head_sha"])
    proof = str(candidate.get("cleanup_proof", ""))

    if proof in BASE_MAIN_PROOFS:
        relation = audit.branch_relation(root, main_sha, head_sha)
        if relation.get("incorporated") is not True or relation.get("proof") != proof:
            raise ValidationError(
                f"main-incorporation proof drift for {branch}: expected {proof}, got {relation.get('proof')}"
            )
        return

    if proof.startswith(LIVE_ANCHOR_PREFIX):
        anchor_name = proof[len(LIVE_ANCHOR_PREFIX) :]
        anchor = branch_index.get(anchor_name)
        if not anchor:
            raise ValidationError(f"missing reviewed live anchor {anchor_name} for {branch}")
        anchor_sha = str(anchor["head_sha"])
        state = fetch_exact_live_branch(
            client,
            branch=anchor_name,
            expected_sha=anchor_sha,
        )
        expected_disposition = str(anchor.get("disposition", ""))
        if expected_disposition == "PROTECTED" and state.get("protected") is not True:
            raise ValidationError(f"reviewed protected anchor lost protection: {anchor_name}")
        if expected_disposition == "OPEN_PR" and not client.open_pulls_for_branch(anchor_name):
            raise ValidationError(f"reviewed open-PR anchor is no longer open: {anchor_name}")
        if not is_ancestor(root, head_sha, anchor_sha):
            raise ValidationError(
                f"reviewed live anchor no longer contains candidate history: {branch} -> {anchor_name}"
            )
        return

    if proof.startswith(DUPLICATE_ANCHOR_PREFIX):
        anchor_name = proof[len(DUPLICATE_ANCHOR_PREFIX) :]
        anchor = branch_index.get(anchor_name)
        if not anchor:
            raise ValidationError(f"missing reviewed duplicate anchor {anchor_name} for {branch}")
        anchor_sha = str(anchor["head_sha"])
        if anchor_sha != head_sha:
            raise ValidationError(
                f"reviewed duplicate proof no longer shares exact head: {branch} vs {anchor_name}"
            )
        fetch_exact_live_branch(
            client,
            branch=anchor_name,
            expected_sha=anchor_sha,
        )
        return

    raise ValidationError(f"unsupported cleanup proof for {branch}: {proof!r}")


def apply_reviewed_policy(
    client: GitHubClient,
    *,
    root: Path,
    default_branch: str,
    expected_main_sha: str,
    approval_path: Path,
    run_id: str,
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    root = audit.validate_repository_root(root)
    approval = audit.load_canonical_json(approval_path)
    audit.validate_approval_identity(approval)
    reviewed_entries = approval["entries"]

    audit.assert_main_unchanged(client, default_branch, expected_main_sha)
    report, current_manifest = build_policy_audit(
        client,
        root=root,
        default_branch=default_branch,
    )
    if report["default_branch_sha"] != expected_main_sha:
        raise ValidationError(
            f"audit default branch SHA mismatch: expected {expected_main_sha}, got {report['default_branch_sha']}"
        )

    branch_index = {str(item["branch"]): item for item in report["branches"]}
    current_branches = {
        branch: str(item["head_sha"]) for branch, item in branch_index.items()
    }
    present, already_absent = audit.validate_apply_candidate_set(
        approval=approval,
        current_manifest=current_manifest,
        current_branches=current_branches,
    )

    reviewed_names = {str(entry["branch"]) for entry in reviewed_entries}
    non_candidate_names = sorted(set(current_branches) - reviewed_names)
    probe = audit.recovery_probe(
        client,
        default_branch_sha=expected_main_sha,
        run_id=run_id,
    )

    deleted: list[dict[str, str]] = []
    for entry in present:
        branch = entry["branch"]
        sha = entry["head_sha"]
        audit.assert_main_unchanged(client, default_branch, expected_main_sha)

        candidate = branch_index.get(branch)
        if not candidate or candidate.get("disposition") != "DELETE":
            raise ValidationError(f"reviewed candidate is no longer DELETE: {branch}")
        verify_candidate_proof_live(
            client,
            root=root,
            candidate=candidate,
            branch_index=branch_index,
            main_sha=expected_main_sha,
        )

        branch_state = fetch_exact_live_branch(
            client,
            branch=branch,
            expected_sha=sha,
        )
        if branch_state.get("protected") is True:
            raise ValidationError(f"approved branch became protected: {branch}")
        if client.open_pulls_for_branch(branch):
            raise ValidationError(f"approved branch gained an open PR: {branch}")

        client.delete_branch(branch, expected_sha=sha)
        if client.get_ref(branch) is not None:
            raise ValidationError(f"historical branch deletion could not be verified: {branch}")
        deleted.append({"branch": branch, "head_sha": sha})

    audit.assert_main_unchanged(client, default_branch, expected_main_sha)
    post_branches = {
        item["branch"]: item["head_sha"] for item in audit.list_live_branches(client)
    }
    remaining_approved = sorted(
        str(entry["branch"]) for entry in reviewed_entries if entry["branch"] in post_branches
    )
    if remaining_approved:
        raise ValidationError(f"approved branches remain after apply: {remaining_approved}")
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
        "issue": audit.ISSUE_NUMBER,
        "main_sha": expected_main_sha,
        "non_candidate_branches_verified_present": len(non_candidate_names),
        "result": "PASS",
        "schema_version": audit.SCHEMA_VERSION,
    }
    return report, evidence, probe


def parser() -> argparse.ArgumentParser:
    value = argparse.ArgumentParser(description="Oteryn historical branch audit policy")
    value.add_argument("--mode", choices=("inventory", "apply"), required=True)
    value.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", ""))
    value.add_argument("--token", default=os.environ.get("GITHUB_TOKEN", ""))
    value.add_argument("--root", type=Path, default=Path("."))
    value.add_argument("--default-branch", default="main")
    value.add_argument("--output", type=Path, required=True)
    value.add_argument("--manifest", type=Path, required=True)
    value.add_argument("--approval", type=Path, default=audit.DEFAULT_APPROVAL_PATH)
    value.add_argument("--approval-validation", type=Path)
    value.add_argument("--evidence", type=Path)
    value.add_argument("--recovery-evidence", type=Path)
    value.add_argument("--event-name", default=os.environ.get("GITHUB_EVENT_NAME", ""))
    value.add_argument("--ref-name", default=os.environ.get("GITHUB_REF_NAME", ""))
    value.add_argument("--expected-main-sha", default="")
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
        report, manifest = build_policy_audit(
            client,
            root=root,
            default_branch=args.default_branch,
        )
        audit.write_json(args.output, report)
        audit.write_json(args.manifest, manifest)
        if args.approval.exists():
            approval = audit.load_canonical_json(args.approval)
            validation = audit.validate_inventory_approval(approval, manifest)
            if args.approval_validation is None:
                raise ValidationError(
                    "--approval-validation is required when an approval file exists"
                )
            audit.write_json(args.approval_validation, validation)
        print(
            f"Policy-audited {report['branch_count']} branches; "
            f"{report['deletion_candidate_count']} exact deletion candidate(s)."
        )
        return 0

    if args.event_name != "push" or args.ref_name != args.default_branch:
        raise ValidationError("apply is allowed only on a push to the protected default branch")
    if not audit.FULL_SHA_RE.fullmatch(args.expected_main_sha):
        raise ValidationError("--expected-main-sha must be a full 40-character SHA")
    if not args.approval.exists():
        raise ValidationError("apply requires the reviewed approval file")
    if args.evidence is None or args.recovery_evidence is None:
        raise ValidationError("apply requires --evidence and --recovery-evidence")

    report, evidence, probe = apply_reviewed_policy(
        client,
        root=root,
        default_branch=args.default_branch,
        expected_main_sha=args.expected_main_sha,
        approval_path=args.approval,
        run_id=os.environ.get("GITHUB_RUN_ID", "local"),
    )
    _, manifest = build_policy_audit(
        client,
        root=root,
        default_branch=args.default_branch,
    )
    audit.write_json(args.output, report)
    audit.write_json(args.manifest, manifest)
    audit.write_json(args.evidence, evidence)
    audit.write_json(args.recovery_evidence, probe)
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
