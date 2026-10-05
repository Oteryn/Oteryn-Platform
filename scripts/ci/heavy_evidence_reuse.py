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
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from classify_changes import classify_path  # noqa: E402

REUSABLE_CLASSES = frozenset(("docs_only", "agent_governance"))
DEFAULT_MAX_DEPTH = 25


@dataclass(frozen=True)
class ReuseDecision:
    reuse: bool
    reason: str
    material_tree_digest: str = ""
    reused_head_sha: str = ""
    reused_run_id: int | None = None


def _git(*args: str) -> str:
    return subprocess.check_output(
        ["git", *args],
        text=True,
        stderr=subprocess.STDOUT,
    ).strip()


def commit_parents(sha: str) -> list[str]:
    parts = _git("rev-list", "--parents", "-n", "1", sha).split()
    if not parts or parts[0] != sha:
        raise ValueError(f"unable to resolve commit parents for {sha}")
    return parts[1:]


def diff_paths(base: str, head: str) -> list[str]:
    output = _git(
        "diff",
        "--name-only",
        "--diff-filter=ACMRD",
        base,
        head,
    )
    return [line for line in output.splitlines() if line.strip()]


def reusable_successor_paths(paths: list[str]) -> bool:
    if not paths:
        return False
    return all(classify_path(path).change_class in REUSABLE_CLASSES for path in paths)


def material_tree_digest(sha: str) -> str:
    output = _git("ls-tree", "-r", "--full-tree", sha)
    material_entries: list[str] = []
    for line in output.splitlines():
        if not line.strip() or "\t" not in line:
            raise ValueError(f"unexpected git ls-tree output for {sha}")
        _, path = line.split("\t", 1)
        if classify_path(path).change_class in REUSABLE_CLASSES:
            continue
        material_entries.append(line)

    payload = ("\n".join(material_entries) + "\n").encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def _github_json(url: str, token: str) -> dict[str, Any]:
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
        payload = json.load(response)
    if not isinstance(payload, dict):
        raise ValueError("GitHub API returned a non-object payload")
    return payload


def find_successful_heavy_run(
    *,
    repository: str,
    pr_number: int,
    workflow: str,
    job_name: str,
    head_sha: str,
    token: str,
) -> int | None:
    encoded_workflow = urllib.parse.quote(workflow, safe="")
    query = urllib.parse.urlencode(
        {
            "event": "pull_request",
            "head_sha": head_sha,
            "status": "completed",
            "per_page": 20,
        }
    )
    runs_url = (
        f"https://api.github.com/repos/{repository}/actions/workflows/"
        f"{encoded_workflow}/runs?{query}"
    )
    runs_payload = _github_json(runs_url, token)
    runs = runs_payload.get("workflow_runs", [])
    if not isinstance(runs, list):
        raise ValueError("GitHub workflow runs response is malformed")

    for run in runs:
        if not isinstance(run, dict):
            continue
        if run.get("head_sha") != head_sha or run.get("conclusion") != "success":
            continue

        pull_requests = run.get("pull_requests", [])
        if not isinstance(pull_requests, list):
            continue
        if not any(
            isinstance(item, dict) and item.get("number") == pr_number
            for item in pull_requests
        ):
            continue

        run_id = run.get("id")
        if not isinstance(run_id, int):
            continue

        jobs_url = (
            f"https://api.github.com/repos/{repository}/actions/runs/{run_id}/jobs"
            "?filter=latest&per_page=100"
        )
        jobs_payload = _github_json(jobs_url, token)
        jobs = jobs_payload.get("jobs", [])
        if not isinstance(jobs, list):
            raise ValueError("GitHub workflow jobs response is malformed")

        if any(
            isinstance(job, dict)
            and job.get("name") == job_name
            and job.get("conclusion") == "success"
            for job in jobs
        ):
            return run_id

    return None


def decide_reuse(
    *,
    repository: str,
    pr_number: int,
    workflow: str,
    job_name: str,
    head_sha: str,
    token: str,
    max_depth: int = DEFAULT_MAX_DEPTH,
) -> ReuseDecision:
    if len(head_sha) != 40 or any(ch not in "0123456789abcdef" for ch in head_sha):
        return ReuseDecision(False, "invalid or non-canonical head SHA")
    if pr_number < 1:
        return ReuseDecision(False, "invalid pull request number")
    if not token:
        return ReuseDecision(False, "GitHub token unavailable; fail closed")

    try:
        current_digest = material_tree_digest(head_sha)
        current = head_sha

        for _ in range(max_depth):
            parents = commit_parents(current)
            if len(parents) != 1:
                return ReuseDecision(
                    False,
                    "ambiguous first-parent history; heavy validation required",
                    current_digest,
                )

            parent = parents[0]
            changed = diff_paths(parent, current)
            if not reusable_successor_paths(changed):
                return ReuseDecision(
                    False,
                    "latest unreused generation contains material or ambiguous changes",
                    current_digest,
                )

            parent_digest = material_tree_digest(parent)
            if parent_digest != current_digest:
                return ReuseDecision(
                    False,
                    "material tree digest changed despite reusable-path classification",
                    current_digest,
                )

            run_id = find_successful_heavy_run(
                repository=repository,
                pr_number=pr_number,
                workflow=workflow,
                job_name=job_name,
                head_sha=parent,
                token=token,
            )
            if run_id is not None:
                return ReuseDecision(
                    True,
                    "reused exact prior heavy evidence with identical material tree",
                    current_digest,
                    parent,
                    run_id,
                )

            current = parent

        return ReuseDecision(
            False,
            f"no reusable successful heavy evidence within {max_depth} first-parent commits",
            current_digest,
        )
    except (
        OSError,
        subprocess.CalledProcessError,
        ValueError,
        urllib.error.URLError,
        urllib.error.HTTPError,
        TimeoutError,
    ) as exc:
        return ReuseDecision(
            False,
            f"evidence resolution failed closed: {type(exc).__name__}: {exc}",
        )


def write_github_output(path: Path, decision: ReuseDecision) -> None:
    with path.open("a", encoding="utf-8") as handle:
        handle.write(f"reuse_heavy={'true' if decision.reuse else 'false'}\n")
        handle.write(f"reused_head_sha={decision.reused_head_sha}\n")
        handle.write(
            f"reused_run_id={decision.reused_run_id if decision.reused_run_id is not None else ''}\n"
        )
        handle.write(f"material_tree_digest={decision.material_tree_digest}\n")


def write_summary(path: Path, decision: ReuseDecision, workflow: str, job_name: str) -> None:
    lines = [
        "### Heavy workflow evidence reuse",
        "",
        f"- workflow: `{workflow}`",
        f"- heavy job: `{job_name}`",
        f"- decision: `{'REUSE' if decision.reuse else 'RUN_HEAVY'}`",
        f"- reason: {decision.reason}",
        f"- material tree digest: `{decision.material_tree_digest or 'UNKNOWN'}`",
        f"- reused exact head: `{decision.reused_head_sha or 'NONE'}`",
        f"- reused workflow run: `{decision.reused_run_id if decision.reused_run_id is not None else 'NONE'}`",
        "",
        "Reuse is routing evidence only and is valid only when an exact prior successful heavy job exists for the same PR and the material tree digest is unchanged.",
        "",
    ]
    with path.open("a", encoding="utf-8") as handle:
        handle.write("\n".join(lines))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Resolve fail-closed reuse of prior heavy workflow evidence."
    )
    parser.add_argument("--repository", required=True)
    parser.add_argument("--pr-number", required=True, type=int)
    parser.add_argument("--workflow", required=True)
    parser.add_argument("--job-name", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--token-env", default="GH_TOKEN")
    parser.add_argument("--max-depth", type=int, default=DEFAULT_MAX_DEPTH)
    parser.add_argument("--github-output", type=Path)
    parser.add_argument("--summary", type=Path)
    parser.add_argument("--json", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    token = os.environ.get(args.token_env, "")
    decision = decide_reuse(
        repository=args.repository,
        pr_number=args.pr_number,
        workflow=args.workflow,
        job_name=args.job_name,
        head_sha=args.head,
        token=token,
        max_depth=args.max_depth,
    )

    if args.github_output:
        write_github_output(args.github_output, decision)
    if args.summary:
        write_summary(args.summary, decision, args.workflow, args.job_name)
    if args.json or (not args.github_output and not args.summary):
        print(
            json.dumps(
                {
                    "reuse_heavy": decision.reuse,
                    "reason": decision.reason,
                    "material_tree_digest": decision.material_tree_digest,
                    "reused_head_sha": decision.reused_head_sha,
                    "reused_run_id": decision.reused_run_id,
                },
                indent=2,
                sort_keys=True,
            )
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
