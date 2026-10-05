#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from classify_changes import GATES, classify_path  # noqa: E402


@dataclass(frozen=True)
class ReusePlan:
    candidate: bool
    material_head: str | None
    equivalent_heads: tuple[str, ...]
    material_digest: str | None
    current_digest: str | None
    reason: str


@dataclass(frozen=True)
class ReuseDecision:
    validation_required: bool
    state: str
    material_head: str | None
    reused_run_id: int | None
    material_digest: str | None
    current_digest: str | None
    reason: str


def _git_text(*args: str) -> str:
    return subprocess.check_output(
        ["git", *args],
        text=True,
        stderr=subprocess.STDOUT,
    ).strip()


def _git_bytes(*args: str) -> bytes:
    return subprocess.check_output(
        ["git", *args],
        stderr=subprocess.STDOUT,
    )


def resolve_commit(value: str) -> str:
    if not value:
        raise ValueError("missing commit")
    return _git_text("rev-parse", "--verify", f"{value}^{{commit}}")


def commit_parents(commit: str) -> list[str]:
    raw = _git_text("show", "-s", "--format=%P", commit)
    return raw.split() if raw else []


def diff_paths(base: str, head: str) -> list[str]:
    raw = _git_text("diff", "--name-only", "--diff-filter=ACMRD", base, head)
    return [line for line in raw.splitlines() if line.strip()]


def gate_affected(paths: list[str], gate: str) -> bool:
    return any(gate in classify_path(path).gates for path in paths)


def _linear_edges(base: str, head: str, *, limit: int = 2000) -> list[tuple[str, str]]:
    if base == head:
        return []

    edges: list[tuple[str, str]] = []
    current = head
    visited: set[str] = set()

    while current != base:
        if current in visited:
            raise ValueError("commit ancestry loop detected")
        visited.add(current)
        if len(edges) >= limit:
            raise ValueError("commit ancestry exceeds bounded reuse limit")

        parents = commit_parents(current)
        if len(parents) != 1:
            raise ValueError(
                f"reuse requires linear PR ancestry; {current} has {len(parents)} parent(s)"
            )
        parent = parents[0]
        edges.append((parent, current))
        current = parent

    edges.reverse()
    return edges


def material_tree_digest(commit: str, gate: str) -> str:
    tree = _git_bytes("ls-tree", "-r", "-z", "--full-tree", commit)
    digest = hashlib.sha256()

    for entry in tree.split(b"\0"):
        if not entry:
            continue
        try:
            metadata, path_raw = entry.split(b"\t", 1)
        except ValueError as exc:
            raise ValueError("unexpected git ls-tree record") from exc
        path = path_raw.decode("utf-8", errors="surrogateescape")
        if gate not in classify_path(path).gates:
            continue
        digest.update(metadata)
        digest.update(b"\t")
        digest.update(path_raw)
        digest.update(b"\0")

    return digest.hexdigest()


def plan_reuse(*, event_name: str, base: str, head: str, gate: str) -> ReusePlan:
    if gate not in GATES:
        raise ValueError(f"unsupported heavy gate: {gate}")

    if event_name != "pull_request":
        return ReusePlan(False, None, (), None, None, "non-pull-request events always validate")

    try:
        resolved_base = resolve_commit(base)
        resolved_head = resolve_commit(head)
        edges = _linear_edges(resolved_base, resolved_head)

        accumulated_paths = diff_paths(resolved_base, resolved_head)
        if not gate_affected(accumulated_paths, gate):
            return ReusePlan(
                False,
                None,
                (),
                None,
                None,
                "gate is not affected by the accumulated PR diff",
            )

        material_head: str | None = None
        for parent, commit in edges:
            if gate_affected(diff_paths(parent, commit), gate):
                material_head = commit

        if material_head is None:
            return ReusePlan(
                False,
                None,
                (),
                None,
                None,
                "no attributable material commit found; validate fail-closed",
            )

        if material_head == resolved_head:
            digest = material_tree_digest(resolved_head, gate)
            return ReusePlan(
                False,
                material_head,
                (),
                digest,
                digest,
                "final head contains a material change for this gate",
            )

        material_digest = material_tree_digest(material_head, gate)
        current_digest = material_tree_digest(resolved_head, gate)
        if material_digest != current_digest:
            return ReusePlan(
                False,
                material_head,
                (),
                material_digest,
                current_digest,
                "gate-specific material tree changed after the candidate material head",
            )

        commit_heads = [commit for _, commit in edges]
        material_index = commit_heads.index(material_head)
        equivalent_heads = tuple(reversed(commit_heads[material_index:-1]))
        if not equivalent_heads:
            return ReusePlan(
                False,
                material_head,
                (),
                material_digest,
                current_digest,
                "no earlier equivalent PR head exists; validate fail-closed",
            )
        if len(equivalent_heads) > 100:
            return ReusePlan(
                False,
                material_head,
                (),
                material_digest,
                current_digest,
                "equivalent-head evidence search exceeds bounded limit; validate fail-closed",
            )

        return ReusePlan(
            True,
            material_head,
            equivalent_heads,
            material_digest,
            current_digest,
            "later commits leave the gate-specific material tree byte-identical",
        )
    except (subprocess.CalledProcessError, ValueError) as exc:
        return ReusePlan(
            False,
            None,
            (),
            None,
            None,
            f"reuse planning is ambiguous; validate fail-closed: {exc}",
        )


def _http_get_json(url: str, token: str) -> dict[str, Any]:
    request = urllib.request.Request(
        url,
        headers={
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {token}",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "oteryn-heavy-evidence-reuse",
        },
    )
    with urllib.request.urlopen(request, timeout=20) as response:
        payload = response.read().decode("utf-8")
    value = json.loads(payload)
    if not isinstance(value, dict):
        raise ValueError("GitHub API response must be an object")
    return value


def _same_pull_request(run: dict[str, Any], pr_number: int) -> bool:
    pull_requests = run.get("pull_requests")
    if not isinstance(pull_requests, list):
        return False
    for pull_request in pull_requests:
        if isinstance(pull_request, dict) and pull_request.get("number") == pr_number:
            return True
    return False


def find_reusable_run(
    *,
    repository: str,
    workflow: str,
    pr_number: int,
    equivalent_heads: tuple[str, ...],
    evidence_job: str,
    token: str,
    api_url: str = "https://api.github.com",
    get_json: Callable[[str, str], dict[str, Any]] = _http_get_json,
) -> tuple[str, int] | None:
    if "/" not in repository or not workflow or pr_number < 1 or not evidence_job:
        raise ValueError("invalid GitHub evidence query context")
    if not equivalent_heads or len(equivalent_heads) > 100:
        raise ValueError("invalid bounded equivalent-head evidence search")

    workflow_id = urllib.parse.quote(Path(workflow).name, safe="")

    for evidence_head in equivalent_heads:
        query = urllib.parse.urlencode(
            {
                "event": "pull_request",
                "head_sha": evidence_head,
                "status": "success",
                "per_page": 100,
            }
        )
        runs_url = (
            f"{api_url.rstrip('/')}/repos/{repository}/actions/workflows/"
            f"{workflow_id}/runs?{query}"
        )
        run_payload = get_json(runs_url, token)
        runs = run_payload.get("workflow_runs")
        total_count = run_payload.get("total_count")
        if not isinstance(runs, list) or not isinstance(total_count, int):
            raise ValueError("malformed workflow-run evidence response")
        if total_count > len(runs):
            raise ValueError("workflow-run evidence response was truncated")

        candidates = [
            run
            for run in runs
            if isinstance(run, dict)
            and run.get("event") == "pull_request"
            and run.get("conclusion") == "success"
            and run.get("head_sha") == evidence_head
            and _same_pull_request(run, pr_number)
            and isinstance(run.get("id"), int)
        ]

        for run in sorted(candidates, key=lambda value: int(value["id"]), reverse=True):
            run_id = int(run["id"])
            jobs_url = (
                f"{api_url.rstrip('/')}/repos/{repository}/actions/runs/"
                f"{run_id}/jobs?per_page=100"
            )
            jobs_payload = get_json(jobs_url, token)
            jobs = jobs_payload.get("jobs")
            jobs_total = jobs_payload.get("total_count")
            if not isinstance(jobs, list) or not isinstance(jobs_total, int):
                raise ValueError("malformed workflow-job evidence response")
            if jobs_total > len(jobs):
                raise ValueError("workflow-job evidence response was truncated")

            if any(
                isinstance(job, dict)
                and job.get("name") == evidence_job
                and job.get("conclusion") == "success"
                for job in jobs
            ):
                return evidence_head, run_id

    return None


def decide_reuse(
    *,
    event_name: str,
    base: str,
    head: str,
    gate: str,
    repository: str,
    workflow: str,
    pr_number: int,
    evidence_job: str,
    token: str,
    get_json: Callable[[str, str], dict[str, Any]] = _http_get_json,
) -> ReuseDecision:
    plan = plan_reuse(event_name=event_name, base=base, head=head, gate=gate)
    if not plan.candidate:
        return ReuseDecision(
            True,
            "RUN",
            plan.material_head,
            None,
            plan.material_digest,
            plan.current_digest,
            plan.reason,
        )

    if not repository or pr_number < 1 or not token:
        return ReuseDecision(
            True,
            "RUN",
            plan.material_head,
            None,
            plan.material_digest,
            plan.current_digest,
            "prior heavy evidence context is missing; validate fail-closed",
        )

    assert plan.material_head is not None
    try:
        reusable = find_reusable_run(
            repository=repository,
            workflow=workflow,
            pr_number=pr_number,
            equivalent_heads=plan.equivalent_heads,
            evidence_job=evidence_job,
            token=token,
            get_json=get_json,
        )
    except (urllib.error.URLError, TimeoutError, ValueError, json.JSONDecodeError) as exc:
        return ReuseDecision(
            True,
            "RUN",
            plan.material_head,
            None,
            plan.material_digest,
            plan.current_digest,
            f"prior heavy evidence lookup is ambiguous; validate fail-closed: {exc}",
        )

    if reusable is None:
        return ReuseDecision(
            True,
            "RUN",
            plan.material_head,
            None,
            plan.material_digest,
            plan.current_digest,
            "no same-PR successful heavy evidence job exists for an equivalent prior head",
        )

    evidence_head, run_id = reusable
    return ReuseDecision(
        False,
        "REUSE",
        evidence_head,
        run_id,
        plan.material_digest,
        plan.current_digest,
        f"reuse prior successful {evidence_job} evidence from equivalent head {evidence_head} run {run_id}",
    )


def write_github_output(path: Path, decision: ReuseDecision) -> None:
    values = {
        "validation_required": "true" if decision.validation_required else "false",
        "reuse_state": decision.state,
        "material_head": decision.material_head or "",
        "reused_run_id": str(decision.reused_run_id or ""),
        "material_digest": decision.material_digest or "",
        "current_digest": decision.current_digest or "",
        "reuse_reason": decision.reason.replace("\n", " "),
    }
    with path.open("a", encoding="utf-8") as handle:
        for key, value in values.items():
            handle.write(f"{key}={value}\n")


def write_summary(path: Path, decision: ReuseDecision, *, gate: str, workflow: str) -> None:
    lines = [
        "### Heavy evidence reuse",
        "",
        f"- workflow: `{workflow}`",
        f"- gate: `{gate}`",
        f"- decision: `{decision.state}`",
        f"- validation required: `{'YES' if decision.validation_required else 'NO'}`",
        f"- material head: `{decision.material_head or 'NONE'}`",
        f"- reused run id: `{decision.reused_run_id or 'NONE'}`",
        f"- material digest: `{decision.material_digest or 'NONE'}`",
        f"- current digest: `{decision.current_digest or 'NONE'}`",
        f"- reason: {decision.reason}",
        "",
        "Reuse is routing evidence only. Exact-final-head lightweight governance/metadata checks still run.",
        "",
    ]
    with path.open("a", encoding="utf-8") as handle:
        handle.write("\n".join(lines))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Fail-closed reuse of same-PR heavy GitHub Actions evidence."
    )
    parser.add_argument("--event-name", required=True)
    parser.add_argument("--base", default="")
    parser.add_argument("--head", required=True)
    parser.add_argument("--gate", required=True, choices=GATES)
    parser.add_argument("--repository", default="")
    parser.add_argument("--workflow", required=True)
    parser.add_argument("--pr-number", type=int, default=0)
    parser.add_argument("--evidence-job", required=True)
    parser.add_argument("--token-env", default="GITHUB_TOKEN")
    parser.add_argument("--github-output", type=Path)
    parser.add_argument("--summary", type=Path)
    parser.add_argument("--json", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    token = os.environ.get(args.token_env, "")
    decision = decide_reuse(
        event_name=args.event_name,
        base=args.base,
        head=args.head,
        gate=args.gate,
        repository=args.repository,
        workflow=args.workflow,
        pr_number=args.pr_number,
        evidence_job=args.evidence_job,
        token=token,
    )

    if args.github_output:
        write_github_output(args.github_output, decision)
    if args.summary:
        write_summary(args.summary, decision, gate=args.gate, workflow=args.workflow)
    if args.json or (not args.github_output and not args.summary):
        print(json.dumps(decision.__dict__, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
