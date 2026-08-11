#!/usr/bin/env python3
"""Bounded, ownership-scoped GitHub Actions storage hygiene.

The script intentionally does not touch releases, packages, GHCR, secrets,
environments, branches or repository content.  It only uses Actions artifact,
cache and workflow-run endpoints for the current repository.
"""

from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from collections.abc import Iterable
from typing import Any

API_VERSION = "2022-11-28"
PR_CACHE_REF = re.compile(r"^refs/pull/(?P<number>[1-9][0-9]*)/merge$")
DEFAULT_ARTIFACT_RETENTION_DAYS = 14
DEFAULT_RUN_RETENTION_DAYS = 30
DEFAULT_DELETE_BUDGET = 700
DEFAULT_DELETE_DELAY_SECONDS = 0.40
MIN_REQUEST_RESERVE = 150


@dataclasses.dataclass(frozen=True)
class Candidate:
    kind: str
    resource_id: int
    size_in_bytes: int
    reason: str
    ref: str | None = None


class GitHubApi:
    def __init__(self, repository: str, token: str) -> None:
        if repository.count("/") != 1:
            raise ValueError("repository must be owner/name")
        self.repository = repository
        self.token = token
        self.base = f"https://api.github.com/repos/{repository}"
        self.rate_remaining: int | None = None
        self.rate_limit: int | None = None

    def request(
        self,
        method: str,
        path: str,
        *,
        params: dict[str, str | int] | None = None,
        tolerate_not_found: bool = False,
    ) -> Any:
        url = path if path.startswith("https://") else f"{self.base}/{path.lstrip('/')}"
        if params:
            url = f"{url}?{urllib.parse.urlencode(params)}"
        request = urllib.request.Request(
            url,
            method=method,
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {self.token}",
                "X-GitHub-Api-Version": API_VERSION,
                "User-Agent": "oteryn-actions-storage-hygiene/1",
            },
        )
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                self._capture_rate_headers(response.headers)
                payload = response.read()
                if not payload:
                    return None
                return json.loads(payload.decode("utf-8"))
        except urllib.error.HTTPError as exc:
            self._capture_rate_headers(exc.headers)
            if tolerate_not_found and exc.code == 404:
                return None
            body = exc.read().decode("utf-8", errors="replace")[:500]
            raise RuntimeError(
                f"GitHub API {method} {path} failed with HTTP {exc.code}: {body}"
            ) from exc

    def _capture_rate_headers(self, headers: Any) -> None:
        remaining = headers.get("x-ratelimit-remaining")
        limit = headers.get("x-ratelimit-limit")
        if remaining is not None:
            self.rate_remaining = int(remaining)
        if limit is not None:
            self.rate_limit = int(limit)

    def paginate(
        self,
        path: str,
        key: str | None,
        *,
        params: dict[str, str | int] | None = None,
    ) -> list[dict[str, Any]]:
        base_params = dict(params or {})
        result: list[dict[str, Any]] = []
        page = 1
        while True:
            page_params = {**base_params, "per_page": 100, "page": page}
            payload = self.request("GET", path, params=page_params)
            if key is None:
                items = payload
            else:
                items = payload.get(key, [])
            if not isinstance(items, list):
                raise RuntimeError(f"Unexpected pagination payload for {path}")
            result.extend(items)
            if len(items) < 100:
                break
            page += 1
        return result


def parse_time(value: str) -> dt.datetime:
    return dt.datetime.fromisoformat(value.replace("Z", "+00:00"))


def cutoff(now: dt.datetime, days: int) -> dt.datetime:
    return now - dt.timedelta(days=days)


def pr_number_from_cache_ref(ref: str) -> int | None:
    match = PR_CACHE_REF.fullmatch(ref)
    return int(match.group("number")) if match else None


def closed_pr_cache_candidates(
    caches: Iterable[dict[str, Any]], open_pr_numbers: set[int]
) -> list[Candidate]:
    candidates: list[Candidate] = []
    for cache in caches:
        ref = str(cache.get("ref", ""))
        pr_number = pr_number_from_cache_ref(ref)
        if pr_number is None or pr_number in open_pr_numbers:
            continue
        candidates.append(
            Candidate(
                kind="cache",
                resource_id=int(cache["id"]),
                size_in_bytes=int(cache.get("size_in_bytes", 0)),
                reason=f"closed pull request #{pr_number}",
                ref=ref,
            )
        )
    return candidates


def exact_pr_cache_candidates(
    caches: Iterable[dict[str, Any]], pr_number: int
) -> list[Candidate]:
    expected_ref = f"refs/pull/{pr_number}/merge"
    return [
        Candidate(
            kind="cache",
            resource_id=int(cache["id"]),
            size_in_bytes=int(cache.get("size_in_bytes", 0)),
            reason=f"closed pull request #{pr_number}",
            ref=expected_ref,
        )
        for cache in caches
        if cache.get("ref") == expected_ref
    ]


def old_artifact_candidates(
    artifacts: Iterable[dict[str, Any]], now: dt.datetime, retention_days: int
) -> list[Candidate]:
    threshold = cutoff(now, retention_days)
    candidates: list[Candidate] = []
    for artifact in artifacts:
        created = parse_time(str(artifact["created_at"]))
        if created >= threshold:
            continue
        candidates.append(
            Candidate(
                kind="artifact",
                resource_id=int(artifact["id"]),
                size_in_bytes=int(artifact.get("size_in_bytes", 0)),
                reason=f"artifact older than {retention_days} days",
            )
        )
    return candidates


def old_completed_run_candidates(
    runs: Iterable[dict[str, Any]], now: dt.datetime, retention_days: int
) -> list[Candidate]:
    threshold = cutoff(now, retention_days)
    candidates: list[Candidate] = []
    for run in runs:
        if run.get("status") != "completed":
            continue
        created = parse_time(str(run["created_at"]))
        if created >= threshold:
            continue
        candidates.append(
            Candidate(
                kind="run",
                resource_id=int(run["id"]),
                size_in_bytes=0,
                reason=f"completed workflow run older than {retention_days} days",
            )
        )
    return candidates


def sum_bytes(items: Iterable[dict[str, Any]]) -> int:
    return sum(int(item.get("size_in_bytes", 0)) for item in items)


def choose_candidates(
    candidates: Iterable[Candidate], delete_budget: int
) -> list[Candidate]:
    # Maximise reclaimed storage inside GitHub API budgets. Runs are retained until
    # their 30-day threshold and therefore only win ties after sized resources.
    ordered = sorted(
        candidates,
        key=lambda item: (item.size_in_bytes, item.kind == "run", item.resource_id),
        reverse=True,
    )
    return ordered[: max(0, delete_budget)]


def deletion_path(candidate: Candidate) -> str:
    if candidate.kind == "cache":
        return f"actions/caches/{candidate.resource_id}"
    if candidate.kind == "artifact":
        return f"actions/artifacts/{candidate.resource_id}"
    if candidate.kind == "run":
        return f"actions/runs/{candidate.resource_id}"
    raise ValueError(f"unsupported candidate kind: {candidate.kind}")


def write_summary(summary: dict[str, Any]) -> None:
    print(json.dumps(summary, sort_keys=True))
    step_summary = os.getenv("GITHUB_STEP_SUMMARY")
    if not step_summary:
        return
    with open(step_summary, "a", encoding="utf-8") as handle:
        handle.write("### GitHub Actions storage hygiene\n")
        for key, value in summary.items():
            handle.write(f"- {key}: `{value}`\n")


def list_open_pr_numbers(api: GitHubApi) -> set[int]:
    pulls = api.paginate("pulls", None, params={"state": "open"})
    return {int(pr["number"]) for pr in pulls}


def list_artifacts(api: GitHubApi) -> list[dict[str, Any]]:
    return api.paginate("actions/artifacts", "artifacts")


def list_caches(api: GitHubApi, ref: str | None = None) -> list[dict[str, Any]]:
    params: dict[str, str | int] = {}
    if ref:
        params["ref"] = ref
    return api.paginate("actions/caches", "actions_caches", params=params)


def list_old_completed_runs(
    api: GitHubApi, now: dt.datetime, retention_days: int
) -> list[dict[str, Any]]:
    threshold = cutoff(now, retention_days).date().isoformat()
    # The date filter avoids traversing the full recent run history. If the
    # provider reports 1,000 matching runs (its filtered-result ceiling), fail
    # closed rather than silently claiming complete coverage.
    runs = api.paginate(
        "actions/runs",
        "workflow_runs",
        params={"status": "completed", "created": f"<{threshold}"},
    )
    if len(runs) >= 1000:
        raise RuntimeError(
            "old workflow-run query reached GitHub's filtered-result ceiling; "
            "split the date range before cleanup"
        )
    return runs


def compute_delete_budget(api: GitHubApi, requested: int) -> int:
    remaining = api.rate_remaining
    if remaining is None:
        return min(requested, DEFAULT_DELETE_BUDGET)
    return max(0, min(requested, remaining - MIN_REQUEST_RESERVE))


def run_audit_or_cleanup(args: argparse.Namespace, api: GitHubApi) -> int:
    now = dt.datetime.now(dt.timezone.utc)
    open_prs = list_open_pr_numbers(api)
    caches = list_caches(api)
    cache_usage = api.request("GET", "actions/cache/usage")
    artifacts = list_artifacts(api)
    old_runs = list_old_completed_runs(api, now, args.run_retention_days)

    cache_candidates = closed_pr_cache_candidates(caches, open_prs)
    artifact_candidates = old_artifact_candidates(
        artifacts, now, args.artifact_retention_days
    )
    run_candidates = old_completed_run_candidates(
        old_runs, now, args.run_retention_days
    )

    # If a run itself will be deleted, its artifacts disappear with it. Avoid
    # spending a second request on those artifacts.
    run_candidate_ids = {candidate.resource_id for candidate in run_candidates}
    filtered_artifact_candidates: list[Candidate] = []
    artifact_by_id = {int(item["id"]): item for item in artifacts}
    for candidate in artifact_candidates:
        workflow_run = artifact_by_id[candidate.resource_id].get("workflow_run") or {}
        if int(workflow_run.get("id", -1)) in run_candidate_ids:
            continue
        filtered_artifact_candidates.append(candidate)

    all_candidates = cache_candidates + filtered_artifact_candidates + run_candidates
    requested_budget = 0 if args.mode == "audit" else args.delete_budget
    actual_budget = compute_delete_budget(api, requested_budget)
    selected = choose_candidates(all_candidates, actual_budget)

    pre = {
        "mode": args.mode,
        "artifact_retention_days": args.artifact_retention_days,
        "run_retention_days": args.run_retention_days,
        "pre_artifact_count": len(artifacts),
        "pre_artifact_bytes": sum_bytes(artifacts),
        "pre_cache_count": int(cache_usage.get("active_caches_count", len(caches))),
        "pre_cache_bytes": int(cache_usage.get("active_caches_size_in_bytes", sum_bytes(caches))),
        "eligible_closed_pr_cache_count": len(cache_candidates),
        "eligible_closed_pr_cache_bytes": sum(item.size_in_bytes for item in cache_candidates),
        "eligible_old_artifact_count": len(filtered_artifact_candidates),
        "eligible_old_artifact_bytes": sum(item.size_in_bytes for item in filtered_artifact_candidates),
        "eligible_old_run_count": len(run_candidates),
        "delete_budget": actual_budget,
        "selected_for_delete": len(selected),
        "rate_limit": api.rate_limit,
        "rate_remaining_before_delete": api.rate_remaining,
    }
    write_summary(pre)

    if args.mode == "audit":
        return 0

    deleted = {"cache": 0, "artifact": 0, "run": 0}
    deleted_bytes = {"cache": 0, "artifact": 0, "run": 0}
    for candidate in selected:
        api.request(
            "DELETE",
            deletion_path(candidate),
            tolerate_not_found=True,
        )
        deleted[candidate.kind] += 1
        deleted_bytes[candidate.kind] += candidate.size_in_bytes
        time.sleep(args.delete_delay_seconds)

    post_cache_usage = api.request("GET", "actions/cache/usage")
    post_artifact_probe = api.request(
        "GET", "actions/artifacts", params={"per_page": 1, "page": 1}
    )
    post_run_probe = api.request(
        "GET", "actions/runs", params={"per_page": 1, "page": 1}
    )

    remaining_candidates = len(all_candidates) - len(selected)
    post = {
        "deleted_cache_count": deleted["cache"],
        "deleted_cache_bytes": deleted_bytes["cache"],
        "deleted_artifact_count": deleted["artifact"],
        "deleted_artifact_bytes": deleted_bytes["artifact"],
        "deleted_run_count": deleted["run"],
        "remaining_eligible_candidates_due_to_budget": max(0, remaining_candidates),
        "post_artifact_count": int(post_artifact_probe.get("total_count", 0)),
        "post_cache_count": int(post_cache_usage.get("active_caches_count", 0)),
        "post_cache_bytes": int(post_cache_usage.get("active_caches_size_in_bytes", 0)),
        "post_workflow_run_count": int(post_run_probe.get("total_count", 0)),
        "rate_remaining_after_delete": api.rate_remaining,
    }
    write_summary(post)
    return 0


def run_closed_pr_cleanup(args: argparse.Namespace, api: GitHubApi) -> int:
    if not args.closed_pr_number:
        raise ValueError("--closed-pr-number is required for closed-pr mode")
    ref = f"refs/pull/{args.closed_pr_number}/merge"
    caches = list_caches(api, ref=ref)
    candidates = exact_pr_cache_candidates(caches, args.closed_pr_number)
    budget = compute_delete_budget(api, args.delete_budget)
    if len(candidates) > budget:
        raise RuntimeError(
            f"closed PR #{args.closed_pr_number} has {len(candidates)} caches but "
            f"safe API budget is {budget}; refusing partial per-PR cleanup"
        )
    deleted_bytes = 0
    for candidate in sorted(candidates, key=lambda item: item.size_in_bytes, reverse=True):
        api.request("DELETE", deletion_path(candidate), tolerate_not_found=True)
        deleted_bytes += candidate.size_in_bytes
        time.sleep(args.delete_delay_seconds)
    remaining = list_caches(api, ref=ref)
    if remaining:
        raise RuntimeError(
            f"cache cleanup for closed PR #{args.closed_pr_number} did not converge"
        )
    write_summary(
        {
            "mode": "closed-pr",
            "closed_pr_number": args.closed_pr_number,
            "deleted_cache_count": len(candidates),
            "deleted_cache_bytes": deleted_bytes,
            "remaining_exact_ref_caches": len(remaining),
        }
    )
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repository", default=os.getenv("GITHUB_REPOSITORY", ""))
    parser.add_argument(
        "--mode", choices=("audit", "cleanup", "closed-pr"), default="audit"
    )
    parser.add_argument("--closed-pr-number", type=int)
    parser.add_argument(
        "--artifact-retention-days", type=int, default=DEFAULT_ARTIFACT_RETENTION_DAYS
    )
    parser.add_argument(
        "--run-retention-days", type=int, default=DEFAULT_RUN_RETENTION_DAYS
    )
    parser.add_argument("--delete-budget", type=int, default=DEFAULT_DELETE_BUDGET)
    parser.add_argument(
        "--delete-delay-seconds", type=float, default=DEFAULT_DELETE_DELAY_SECONDS
    )
    return parser


def main() -> int:
    args = build_parser().parse_args()
    token = os.getenv("GH_TOKEN") or os.getenv("GITHUB_TOKEN")
    if not args.repository:
        raise SystemExit("repository is required")
    if not token:
        raise SystemExit("GH_TOKEN or GITHUB_TOKEN is required")
    if args.artifact_retention_days < 1 or args.run_retention_days < 1:
        raise SystemExit("retention days must be positive")
    if args.delete_budget < 0:
        raise SystemExit("delete budget cannot be negative")
    api = GitHubApi(args.repository, token)
    if args.mode == "closed-pr":
        return run_closed_pr_cleanup(args, api)
    return run_audit_or_cleanup(args, api)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RuntimeError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc
