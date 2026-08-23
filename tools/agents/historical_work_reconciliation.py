#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Any, Callable

import historical_branch_audit as audit
from branch_lifecycle import ApiError, GitHubClient, ValidationError

ISSUE_NUMBER = 1072
ALLOWED_REPOSITORY = "Oteryn/Oteryn-Platform"
ALLOWED_DEFAULT_BRANCH = "main"
REGISTRY = Path("docs/agents/HISTORICAL_WORK_RECONCILIATION_REGISTRY.json")
REVIEWED_IMPLEMENTATION_PATHS = (
    Path(".github/workflows/historical-branch-audit.yml"),
    Path("tools/agents/historical_work_reconciliation.py"),
)
TERMINAL = {
    "ACTIVE",
    "CANONICALIZE_TO_MAIN",
    "DOCUMENT_ARCHIVE",
    "PR_PROVENANCE_DELETE",
    "MANAGED_RECOVERY",
    "DELETE",
}
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
RECOVERY_REF_RE = re.compile(r"^refs/oteryn-recovery/[A-Za-z0-9._/-]+$")
REF_SENSITIVE_RE = re.compile(
    r"(?:refs/tags/|refs/oteryn-recovery/|\btags(?:-ignore)?\s*:|"
    r"github\.ref(?:_name|_type)?\b|\brelease\b|\bdeploy(?:ment)?\b|\bpublication\b)",
    re.I,
)


def canonical(value: object) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(canonical(value), encoding="utf-8")


def load_json(path: Path) -> dict[str, Any]:
    raw = path.read_text(encoding="utf-8")
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise ValidationError(
            f"{path}: invalid JSON at {exc.lineno}:{exc.colno}: {exc.msg}"
        ) from exc
    if not isinstance(value, dict) or raw != canonical(value):
        raise ValidationError(f"{path}: expected canonical JSON object")
    return value


def validate_repository_scope(repo: str, default_branch: str) -> None:
    if repo.casefold() != ALLOWED_REPOSITORY.casefold():
        raise ValidationError(
            f"Issue #{ISSUE_NUMBER} may mutate only {ALLOWED_REPOSITORY}; got {repo!r}"
        )
    if default_branch != ALLOWED_DEFAULT_BRANCH:
        raise ValidationError(
            f"Issue #{ISSUE_NUMBER} may target only protected {ALLOWED_DEFAULT_BRANCH}; "
            f"got {default_branch!r}"
        )


def git_blob_sha(root: Path, relative: Path) -> str:
    value = audit.run_git(
        root,
        ["rev-parse", "--verify", f"HEAD:{relative.as_posix()}"],
        purpose=f"reviewed implementation blob {relative.as_posix()}",
        timeout=30,
    ).stdout.strip()
    if not SHA_RE.fullmatch(value):
        raise ValidationError(
            f"reviewed implementation blob invalid for {relative.as_posix()}: {value!r}"
        )
    return value


def validate_reviewed_implementation(
    root: Path,
    registry: dict[str, Any],
    *,
    blob_resolver: Callable[[Path], str] | None = None,
) -> None:
    binding = registry.get("reviewed_apply_implementation_blobs")
    if not isinstance(binding, dict):
        raise ValidationError("reviewed_apply_implementation_blobs missing")
    expected_paths = {path.as_posix() for path in REVIEWED_IMPLEMENTATION_PATHS}
    if set(binding) != expected_paths:
        raise ValidationError(
            "reviewed apply implementation binding must cover exactly "
            + ", ".join(sorted(expected_paths))
        )
    resolve = blob_resolver or (lambda relative: git_blob_sha(root, relative))
    for relative in REVIEWED_IMPLEMENTATION_PATHS:
        expected = binding.get(relative.as_posix())
        if not isinstance(expected, str) or not SHA_RE.fullmatch(expected):
            raise ValidationError(
                f"reviewed implementation blob binding invalid for {relative.as_posix()}"
            )
        actual = resolve(relative)
        if actual != expected:
            raise ValidationError(
                f"reviewed implementation drift for {relative.as_posix()}: "
                f"expected {expected}, got {actual}"
            )


def workflow_inventory(root: Path) -> dict[str, Any]:
    base = root / ".github/workflows"
    if not base.is_dir():
        raise ValidationError(".github/workflows is missing")
    rows = []
    for path in sorted([*base.glob("*.yml"), *base.glob("*.yaml")]):
        raw = path.read_text(encoding="utf-8")
        rows.append(
            {
                "path": path.relative_to(root).as_posix(),
                "sha256": hashlib.sha256(raw.encode()).hexdigest(),
                "ref_sensitive_lines": [
                    {"line": n, "text": line.strip()}
                    for n, line in enumerate(raw.splitlines(), 1)
                    if REF_SENSITIVE_RE.search(line)
                ],
            }
        )
    if not rows:
        raise ValidationError("workflow inventory is empty")
    return {"workflow_count": len(rows), "workflows": rows}


def parse_time(value: str) -> dt.datetime:
    try:
        parsed = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ValidationError(f"invalid ISO-8601 timestamp: {value}") from exc
    if parsed.tzinfo is None:
        raise ValidationError("managed recovery timestamp must include timezone")
    return parsed.astimezone(dt.timezone.utc)


def validate_managed(
    entry: dict[str, Any],
    *,
    workflow_safety: str,
    now: dt.datetime | None = None,
    reachable: Callable[[str, str], bool] | None = None,
    root: Path | None = None,
) -> None:
    contract = entry.get("managed_recovery")
    if not isinstance(contract, dict):
        raise ValidationError(f"{entry.get('branch')}: MANAGED_RECOVERY metadata missing")
    for field in (
        "owner",
        "purpose",
        "restore_procedure",
        "review_trigger",
        "retention_reason",
    ):
        if not isinstance(contract.get(field), str) or not contract[field].strip():
            raise ValidationError(f"{entry.get('branch')}: managed_recovery.{field} missing")
    source = contract.get("source_object_sha")
    ref = contract.get("retention_ref")
    due = contract.get("review_due_at")
    if (
        source != entry.get("source_head_sha")
        or not isinstance(source, str)
        or not SHA_RE.fullmatch(source)
    ):
        raise ValidationError(f"{entry.get('branch')}: managed recovery source SHA mismatch")
    if not isinstance(ref, str) or not RECOVERY_REF_RE.fullmatch(ref):
        raise ValidationError(
            f"{entry.get('branch')}: managed recovery ref must use refs/oteryn-recovery/*"
        )
    if not isinstance(due, str) or parse_time(due) <= (
        now or dt.datetime.now(dt.timezone.utc)
    ):
        raise ValidationError(f"{entry.get('branch')}: managed recovery review is due/expired")
    if workflow_safety != "SAFE":
        raise ValidationError(
            f"{entry.get('branch')}: managed recovery automation safety is not proven SAFE"
        )
    if reachable is None:
        if root is None:
            raise ValidationError("managed recovery reachability requires repository root")
        resolved = audit.run_git(
            root, ["rev-parse", "--verify", ref], purpose=f"resolve {ref}"
        ).stdout.strip()
        ok = bool(SHA_RE.fullmatch(resolved)) and audit.run_git(
            root,
            ["merge-base", "--is-ancestor", source, resolved],
            allow=(0, 1),
            purpose=f"prove recovery reachability {ref}",
        ).returncode == 0
    else:
        ok = reachable(source, ref)
    if not ok:
        raise ValidationError(f"{entry.get('branch')}: managed recovery object is not reachable")


def validate_registry(
    registry: dict[str, Any],
    *,
    root: Path,
    workflows: dict[str, Any],
    now: dt.datetime | None = None,
    reachable: Callable[[str, str], bool] | None = None,
    blob_resolver: Callable[[Path], str] | None = None,
) -> list[dict[str, Any]]:
    if registry.get("schema_version") != 1 or registry.get("issue") != ISSUE_NUMBER:
        raise ValidationError("registry identity mismatch")
    phase = registry.get("registry_phase")
    if phase not in {"reviewed_for_deletion", "applied"}:
        raise ValidationError("registry phase invalid")
    if (
        not isinstance(registry.get("reviewed_main_sha"), str)
        or not SHA_RE.fullmatch(registry["reviewed_main_sha"])
    ):
        raise ValidationError("reviewed_main_sha invalid")
    entries = registry.get("entries")
    if not isinstance(entries, list) or not entries:
        raise ValidationError("registry entries missing")
    validate_reviewed_implementation(root, registry, blob_resolver=blob_resolver)

    names: set[str] = set()
    counts: Counter[str] = Counter()
    managed_names = []
    for item in entries:
        if not isinstance(item, dict):
            raise ValidationError("registry entry must be object")
        branch = item.get("branch")
        sha = item.get("source_head_sha")
        disp = item.get("terminal_disposition")
        if not isinstance(branch, str) or not branch or branch in names:
            raise ValidationError(f"duplicate/invalid registry branch: {branch!r}")
        names.add(branch)
        if not isinstance(sha, str) or not SHA_RE.fullmatch(sha):
            raise ValidationError(f"{branch}: source_head_sha invalid")
        if disp not in TERMINAL:
            raise ValidationError(f"{branch}: non-terminal or unknown disposition {disp!r}")
        counts[str(disp)] += 1
        catalog = registry.get("reason_catalog")
        reason = item.get("reason")
        if (
            not isinstance(catalog, dict)
            or not isinstance(reason, str)
            or not isinstance(catalog.get(reason), str)
            or not catalog[reason].strip()
        ):
            raise ValidationError(f"{branch}: reason is missing from reason_catalog")
        evidence = item.get("evidence")
        if (
            not isinstance(evidence, list)
            or "registry:self" not in evidence
            or any(not isinstance(value, str) or not value for value in evidence)
        ):
            raise ValidationError(
                f"{branch}: evidence must include registry:self and non-empty evidence refs"
            )
        if disp == "PR_PROVENANCE_DELETE":
            pr = item.get("exact_pr")
            if (
                not isinstance(pr, dict)
                or pr.get("head_sha") != sha
                or pr.get("state") != "closed"
            ):
                raise ValidationError(f"{branch}: exact closed PR provenance mismatch")
        if disp == "MANAGED_RECOVERY":
            managed_names.append(branch)
            verdict = registry.get("managed_recovery_workflow_safety_verdict", "UNPROVEN")
            validate_managed(
                item,
                workflow_safety=verdict,
                now=now,
                reachable=reachable,
                root=root,
            )
        expected = "present_reviewed" if phase == "reviewed_for_deletion" else "deleted_verified"
        if item.get("source_ref_state") != expected:
            raise ValidationError(f"{branch}: source_ref_state must be {expected}")
    if registry.get("terminal_disposition_counts") != dict(sorted(counts.items())):
        raise ValidationError("terminal_disposition_counts drift")
    managed = registry.get("managed_recovery")
    if not isinstance(managed, list) or sorted(
        item.get("branch") for item in managed if isinstance(item, dict)
    ) != sorted(managed_names):
        raise ValidationError("managed_recovery summary mismatch")
    if registry.get("workflow_inventory_count") != workflows.get("workflow_count"):
        raise ValidationError(
            "workflow inventory count drift; ref-safety review must be refreshed"
        )
    return sorted(entries, key=lambda item: item["branch"])


def canonical_preservation(root: Path, item: dict[str, Any]) -> None:
    missing = []
    for value in item["evidence"]:
        if value == "registry:self":
            path = root / REGISTRY
            if not path.exists():
                missing.append(value)
            continue
        if not value.startswith("path:"):
            continue
        path = (root / value.removeprefix("path:")).resolve()
        try:
            path.relative_to(root.resolve())
        except ValueError as exc:
            raise ValidationError(f"{item['branch']}: evidence path escapes repository") from exc
        if not path.exists():
            missing.append(value)
    if missing:
        raise ValidationError(f"{item['branch']}: canonical preservation missing: {missing}")


def candidate_guard(
    root: Path,
    item: dict[str, Any],
    row: dict[str, Any],
    branch_state: dict[str, Any],
    open_pulls: list[dict[str, Any]],
) -> None:
    name = item["branch"]
    sha = item["source_head_sha"]
    commit = branch_state.get("commit") if isinstance(branch_state, dict) else None
    if not isinstance(commit, dict) or commit.get("sha") != sha:
        raise ValidationError(f"{name}: pre-delete SHA drift")
    if branch_state.get("protected") is True:
        raise ValidationError(f"{name}: branch became protected")
    if open_pulls:
        raise ValidationError(f"{name}: branch gained open PR")
    if row.get("active_claims"):
        raise ValidationError(f"{name}: branch gained active claim")
    canonical_preservation(root, item)
    if item["terminal_disposition"] == "PR_PROVENANCE_DELETE":
        exact = [
            pull
            for pull in row.get("pull_history", [])
            if pull.get("head_sha") == sha and pull.get("state") == "closed"
        ]
        if not exact:
            raise ValidationError(f"{name}: live exact PR provenance mismatch")


def unexplained_unregistered(row: dict[str, Any]) -> bool:
    return (
        not row.get("protected")
        and not row.get("open_pr_numbers")
        and not row.get("active_claims")
    )


def verify_non_candidates(
    before: dict[str, str], candidate_names: set[str], after: dict[str, str]
) -> None:
    missing = sorted(
        name for name in before if name not in candidate_names and name not in after
    )
    drifted = sorted(
        {"branch": name, "before": before[name], "after": after[name]}
        for name in before
        if name not in candidate_names and name in after and before[name] != after[name]
    )
    if missing or drifted:
        raise ValidationError(
            f"non-candidate refs changed during apply: missing={missing}, drifted={drifted}"
        )


def remote_ref_sha(root: Path, branch: str) -> str | None:
    result = audit.run_git(
        root,
        ["ls-remote", "--heads", "origin", f"refs/heads/{branch}"],
        purpose=f"authoritative remote ref check {branch}",
        timeout=30,
    )
    lines = [line for line in result.stdout.splitlines() if line.strip()]
    if not lines:
        return None
    if len(lines) != 1:
        raise ValidationError(f"{branch}: unexpected remote ref cardinality {len(lines)}")
    sha, sep, ref = lines[0].partition("\t")
    if sep != "\t" or ref != f"refs/heads/{branch}" or not SHA_RE.fullmatch(sha):
        raise ValidationError(f"{branch}: malformed ls-remote result")
    return sha


def assert_main_ancestor(root: Path, reviewed: str, current: str) -> None:
    if audit.run_git(
        root,
        ["merge-base", "--is-ancestor", reviewed, current],
        allow=(0, 1),
        purpose="reviewed main ancestry",
    ).returncode:
        raise ValidationError(
            f"current main {current} does not descend from reviewed main {reviewed}"
        )


def assert_reviewed_main_drift(
    root: Path, registry: dict[str, Any], current: str
) -> None:
    if registry.get("registry_phase") != "reviewed_for_deletion":
        return
    reviewed = registry["reviewed_main_sha"]
    lines = audit.run_git(
        root,
        ["diff", "--name-only", f"{reviewed}..{current}"],
        purpose="reviewed main drift paths",
    ).stdout.splitlines()
    allowed = registry.get("apply_main_change_allowlist")
    if not isinstance(allowed, list) or any(
        not isinstance(item, str) or not item for item in allowed
    ):
        raise ValidationError("apply_main_change_allowlist missing/invalid")
    extras = sorted(set(line for line in lines if line.strip()) - set(allowed))
    if extras:
        raise ValidationError(
            f"protected main changed outside reviewed reconciliation paths: {extras}"
        )


def live_state(
    client: GitHubClient,
    *,
    root: Path,
    default_branch: str,
    registry: dict[str, Any],
    entries: list[dict[str, Any]],
    require_present: bool,
) -> dict[str, Any]:
    report, _ = audit.build_audit(
        client,
        root=root,
        default_branch=default_branch,
        script_path=Path("tools/agents/historical_branch_audit.py"),
    )
    main_sha = report["default_branch_sha"]
    assert_main_ancestor(root, registry["reviewed_main_sha"], main_sha)
    assert_reviewed_main_drift(root, registry, main_sha)
    live = {item["branch"]: item for item in report["branches"]}
    registered = {item["branch"]: item for item in entries}
    accounted: list[dict[str, Any]] = []
    present: list[dict[str, Any]] = []
    absent: list[dict[str, Any]] = []
    unexplained: list[dict[str, Any]] = []
    for item in entries:
        name = item["branch"]
        expected = item["source_head_sha"]
        row = live.get(name)
        if row is None:
            absent.append({"branch": name, "source_head_sha": expected})
            if require_present:
                unexplained.append(
                    {"branch": name, "reason": "reviewed source disappeared before mutation"}
                )
            continue
        reasons = []
        if row["head_sha"] != expected:
            reasons.append("SHA_DRIFT")
        if row.get("protected"):
            reasons.append("PROTECTED")
        if row.get("open_pr_numbers"):
            reasons.append("OPEN_PR")
        if row.get("active_claims"):
            reasons.append("ACTIVE_CLAIM")
        if reasons:
            unexplained.append(
                {"branch": name, "reason": reasons, "live_sha": row["head_sha"]}
            )
            continue
        present.append(
            {
                "branch": name,
                "head_sha": expected,
                "terminal_disposition": item["terminal_disposition"],
            }
        )
        accounted.append(
            {
                "branch": name,
                "head_sha": expected,
                "classification": item["terminal_disposition"],
            }
        )
    for name, row in sorted(live.items()):
        if name in registered:
            continue
        if name == default_branch or row.get("protected"):
            accounted.append(
                {"branch": name, "head_sha": row["head_sha"], "classification": "PROTECTED"}
            )
        elif row.get("open_pr_numbers") or row.get("active_claims"):
            accounted.append(
                {
                    "branch": name,
                    "head_sha": row["head_sha"],
                    "classification": "ACTIVE",
                    "open_pr_numbers": row.get("open_pr_numbers", []),
                    "active_claims": row.get("active_claims", []),
                }
            )
        elif unexplained_unregistered(row):
            unexplained.append(
                {
                    "branch": name,
                    "head_sha": row["head_sha"],
                    "legacy_disposition": row.get("disposition"),
                    "reason": "UNREVIEWED_LIVE_REF",
                }
            )
        else:
            raise ValidationError(f"{name}: impossible live-ref accounting state")
    return {
        "accounted": sorted(accounted, key=lambda item: item["branch"]),
        "branch_count": len(live),
        "default_branch": default_branch,
        "default_branch_sha": main_sha,
        "registered_present": sorted(present, key=lambda item: item["branch"]),
        "registered_absent": sorted(absent, key=lambda item: item["branch"]),
        "unexplained": sorted(unexplained, key=lambda item: item["branch"]),
        "unexplained_count": len(unexplained),
    }


def assert_main(client: GitHubClient, branch: str, sha: str) -> None:
    state = client.get_branch(branch)
    commit = state.get("commit") if isinstance(state, dict) else None
    if (
        not isinstance(commit, dict)
        or commit.get("sha") != sha
        or state.get("protected") is not True
    ):
        raise ValidationError(f"protected {branch} drifted or lost protection")


def restore_probe(
    client: GitHubClient, root: Path, sha: str, run_id: str
) -> dict[str, Any]:
    name = "historical-reconciliation-restore-probe-" + re.sub(
        r"[^0-9A-Za-z_-]", "-", run_id or "local"
    )
    if client.get_ref(name) is not None:
        raise ValidationError(f"restore probe exists: {name}")
    present = False
    try:
        client.create_branch(name, sha)
        present = True
        if remote_ref_sha(root, name) != sha:
            raise ValidationError("restore probe create mismatch")
        client.delete_branch(name, expected_sha=sha)
        present = False
        if remote_ref_sha(root, name) is not None:
            raise ValidationError("restore probe delete failed")
        client.create_branch(name, sha)
        present = True
        if remote_ref_sha(root, name) != sha:
            raise ValidationError("restore probe recreate mismatch")
        client.delete_branch(name, expected_sha=sha)
        present = False
        if remote_ref_sha(root, name) is not None:
            raise ValidationError("restore probe cleanup failed")
        return {
            "branch": name,
            "object_sha": sha,
            "create": True,
            "delete": True,
            "restore": True,
            "final_absence": True,
            "result": "PASS",
        }
    finally:
        if present:
            client.delete_branch(name, expected_sha=sha)


def restore_deleted_refs(
    client: GitHubClient, root: Path, deleted: list[dict[str, str]]
) -> None:
    failures: list[str] = []
    for item in reversed(deleted):
        name = item["branch"]
        sha = item["head_sha"]
        try:
            current = remote_ref_sha(root, name)
            if current == sha:
                continue
            if current is not None:
                failures.append(f"{name}: restore blocked by live SHA {current}")
                continue
            client.create_branch(name, sha)
            restored = remote_ref_sha(root, name)
            if restored != sha:
                failures.append(
                    f"{name}: restore verification expected {sha}, got {restored}"
                )
        except Exception as exc:  # noqa: BLE001 - collect all rollback failures
            failures.append(f"{name}: {exc}")
    if failures:
        raise ValidationError("reviewed deletion rollback failed: " + "; ".join(failures))


def atomic_delete_reviewed_refs(
    client: GitHubClient,
    root: Path,
    entries: list[dict[str, Any]],
) -> list[dict[str, str]]:
    if not entries:
        return []
    remote = client._validated_git_remote()  # repository identity is checked by GitHubClient
    args = ["push", "--porcelain", "--atomic"]
    for item in entries:
        name = item["branch"]
        sha = item["source_head_sha"]
        args.append(f"--force-with-lease=refs/heads/{name}:{sha}")
    args.append(remote)
    args.extend(f":refs/heads/{item['branch']}" for item in entries)
    result = audit.run_git(
        root,
        args,
        allow=(0, 1, 128),
        timeout=120,
        purpose="atomic exact-head historical reconciliation deletion",
    )
    if result.returncode != 0:
        missing = []
        drifted = []
        for item in entries:
            name = item["branch"]
            expected = item["source_head_sha"]
            live = remote_ref_sha(root, name)
            if live is None:
                missing.append({"branch": name, "head_sha": expected})
            elif live != expected:
                drifted.append({"branch": name, "expected": expected, "live": live})
        if missing:
            restore_deleted_refs(client, root, missing)
        detail = (result.stderr or result.stdout).strip()
        raise ValidationError(
            "atomic reviewed deletion rejected; "
            f"drifted={drifted}; transport={detail[:500]}"
        )
    return [
        {"branch": item["branch"], "head_sha": item["source_head_sha"]}
        for item in entries
    ]


def apply(
    client: GitHubClient,
    *,
    root: Path,
    default_branch: str,
    main_sha: str,
    registry: dict[str, Any],
    entries: list[dict[str, Any]],
    run_id: str,
) -> tuple[dict[str, Any], dict[str, Any]]:
    if registry["registry_phase"] != "reviewed_for_deletion":
        raise ValidationError("apply requires reviewed_for_deletion registry")
    assert_main(client, default_branch, main_sha)
    pre = live_state(
        client,
        root=root,
        default_branch=default_branch,
        registry=registry,
        entries=entries,
        require_present=True,
    )
    if pre["unexplained_count"]:
        raise ValidationError(f"pre-apply unexplained refs: {pre['unexplained']}")
    report, _ = audit.build_audit(
        client,
        root=root,
        default_branch=default_branch,
        script_path=Path("tools/agents/historical_branch_audit.py"),
    )
    live = {item["branch"]: item for item in report["branches"]}
    for item in entries:
        name = item["branch"]
        candidate_guard(
            root,
            item,
            live[name],
            client.get_branch(name),
            client.open_pulls_for_branch(name),
        )
    before = {
        item["branch"]: item["head_sha"] for item in audit.list_live_branches(client)
    }
    candidate_names = {item["branch"] for item in entries}
    non_candidates = sorted(set(before) - candidate_names)
    probe = restore_probe(client, root, main_sha, run_id)
    deleted: list[dict[str, str]] = []
    try:
        assert_main(client, default_branch, main_sha)
        deleted = atomic_delete_reviewed_refs(client, root, entries)
        for item in deleted:
            if remote_ref_sha(root, item["branch"]) is not None:
                raise ValidationError(
                    f"{item['branch']}: authoritative Git transport absence verification failed"
                )
        assert_main(client, default_branch, main_sha)
        after = {
            item["branch"]: item["head_sha"]
            for item in audit.list_live_branches(client)
        }
        verify_non_candidates(before, candidate_names, after)
        post = live_state(
            client,
            root=root,
            default_branch=default_branch,
            registry=registry,
            entries=entries,
            require_present=False,
        )
        if post["unexplained_count"] or post["registered_present"]:
            raise ValidationError(f"post-apply reconciliation failed: {post}")
    except Exception as primary:  # noqa: BLE001 - rollback is required for any post-delete failure
        if deleted:
            try:
                restore_deleted_refs(client, root, deleted)
            except Exception as rollback:  # noqa: BLE001
                raise ValidationError(
                    "historical reconciliation apply failed and exact rollback also failed: "
                    f"primary={primary}; rollback={rollback}"
                ) from primary
        raise
    return post, {
        "issue": ISSUE_NUMBER,
        "main_sha": main_sha,
        "deletion_mode": "atomic_exact_head_batch",
        "deleted": deleted,
        "deleted_count": len(deleted),
        "non_candidate_ref_count_verified_present": len(non_candidates),
        "restore_probe": probe,
        "result": "PASS",
    }


def cli() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Issue #1072 historical work reconciliation")
    parser.add_argument("--mode", choices=("inventory", "apply"), required=True)
    parser.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", ""))
    parser.add_argument("--token", default=os.environ.get("GITHUB_TOKEN", ""))
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--default-branch", default=ALLOWED_DEFAULT_BRANCH)
    parser.add_argument("--registry", type=Path, default=REGISTRY)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--evidence", type=Path)
    parser.add_argument("--event-name", default=os.environ.get("GITHUB_EVENT_NAME", ""))
    parser.add_argument("--ref-name", default=os.environ.get("GITHUB_REF_NAME", ""))
    parser.add_argument("--event-sha", default=os.environ.get("GITHUB_SHA", ""))
    parser.add_argument("--expected-main-sha", default="")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = cli().parse_args(argv)
    if not args.repo or "/" not in args.repo or not args.token:
        raise ValidationError("repo owner/name and GitHub token are required")
    validate_repository_scope(args.repo, args.default_branch)
    root = audit.validate_repository_root(args.root.resolve())
    registry_path = args.registry if args.registry.is_absolute() else root / args.registry
    workflows = workflow_inventory(root)
    registry = load_json(registry_path)
    entries = validate_registry(registry, root=root, workflows=workflows)
    client = GitHubClient(args.repo, args.token, root=root)
    if args.mode == "inventory":
        report = live_state(
            client,
            root=root,
            default_branch=args.default_branch,
            registry=registry,
            entries=entries,
            require_present=registry["registry_phase"] == "reviewed_for_deletion",
        )
        report.update(
            {
                "issue": ISSUE_NUMBER,
                "registry_phase": registry["registry_phase"],
                "terminal_disposition_counts": registry["terminal_disposition_counts"],
                "workflow_safety": workflows,
            }
        )
        write_json(args.output, report)
        if report["unexplained_count"]:
            raise ValidationError(f"unexplained live refs: {report['unexplained']}")
        print(
            f"historical work inventory PASS: {report['branch_count']} refs; "
            f"{len(report['registered_present'])} reviewed present; "
            f"{len(report['registered_absent'])} terminally absent"
        )
        return 0
    if (
        args.event_name != "push"
        or args.ref_name != ALLOWED_DEFAULT_BRANCH
        or not SHA_RE.fullmatch(args.event_sha)
        or not SHA_RE.fullmatch(args.expected_main_sha)
        or args.event_sha != args.expected_main_sha
    ):
        raise ValidationError(
            "apply allowed only on the exact triggering push SHA to protected main"
        )
    if args.evidence is None:
        raise ValidationError("--evidence required for apply")
    post, evidence = apply(
        client,
        root=root,
        default_branch=args.default_branch,
        main_sha=args.expected_main_sha,
        registry=registry,
        entries=entries,
        run_id=os.environ.get("GITHUB_RUN_ID", "local"),
    )
    write_json(args.output, post)
    write_json(args.evidence, evidence)
    print(
        f"historical work apply PASS: deleted {evidence['deleted_count']} exact reviewed refs"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValidationError, ApiError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
