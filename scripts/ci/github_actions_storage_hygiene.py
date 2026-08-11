#!/usr/bin/env python3
"""Bounded, ownership-scoped GitHub Actions storage hygiene.

The script intentionally does not touch releases, packages, GHCR, secrets,
environments, branches or repository content. It only uses Actions artifact,
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
RUN_SEARCH_WINDOW_DAYS = 30
RUN_SEARCH_RESULT_CEILING = 1000
RUN_SEARCH_MIN_SPLIT_SECONDS = 60


@dataclasses.dataclass(frozen=True)
class Candidate:
    kind: str
    resource_id: int
    size_in_bytes: int
    reason: str
    ref: str | None = None
    parent_run_id: int | None = None


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
                "User-Agent": "oteryn-actions-storage-hygiene/2",
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
                return {"already_absent": True}
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


def format_search_time(value: dt.datetime) -> str:
    return value.astimezone(dt.timezone.utc).replace(microsecond=0).strftime(
        "%Y-%m-%dT%H:%M:%SZ"
    )


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


def artifact_parent_run_id(artifact: dict[str, Any]) -> int | None:
    workflow_run = artifact.get("workflow_run") or {}
    raw_id = workflow_run.get("id")
    return int(raw_id) if raw_id is not None else None


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
                parent_run_id=artifact_parent_run_id(artifact),
            )
        )
    return candidates


def artifact_bytes_by_run(
    artifacts: Iterable[dict[str, Any]],
) -> dict[int, int]:
    result: dict[int, int] = {}
    for artifact in artifacts:
        parent_run_id = artifact_parent_run_id(artifact)
        if parent_run_id is None:
            continue
        result[parent_run_id] = result.get(parent_run_id, 0) + int(
            artifact.get("size_in_bytes", 0)
        )
    return result


def old_completed_run_candidates(
    runs: Iterable[dict[str, Any]],
    now: dt.datetime,
    retention_days: int,
    run_artifact_bytes: dict[int, int] | None = None,
) -> list[Candidate]:
    threshold = cutoff(now, retention_days)
    bytes_by_run = run_artifact_bytes or {}
    candidates: list[Candidate] = []
    for run in runs:
        if run.get("status") != "completed":
            continue
        created = parse_time(str(run["created_at"]))
        if created >= threshold:
            continue
        run_id = int(run["id"])
        candidates.append(
            Candidate(
                kind="run",
                resource_id=run_id,
                size_in_bytes=int(bytes_by_run.get(run_id, 0)),
                reason=f"completed workflow run older than {retention_days} days",
            )
        )
    return candidates


def sum_bytes(items: Iterable[dict[str, Any]]) -> int:
    return sum(int(item.get("size_in_bytes", 0)) for item in items)


def choose_candidates(
    candidates: Iterable[Candidate], delete_budget: int
) -> list[Candidate]:
    """Select exact DELETE calls while avoiding redundant artifact/run deletion.

    A run candidate carries the aggregate byte size of its current artifacts.
    When that run is selected, its child artifacts are covered by the run DELETE
    and therefore do not consume separate API delete slots.
    """
    ordered = sorted(
        candidates,
        key=lambda item: (
            item.size_in_bytes,
            item.kind == "run",
            item.kind == "cache",
            item.resource_id,
        ),
        reverse=True,
    )
    selected: list[Candidate] = []
    selected_run_ids: set[int] = set()

    for item in ordered:
        if item.kind == "artifact" and item.parent_run_id in selected_run_ids:
            continue

        if item.kind == "run":
            selected = [
                current
                for current in selected
                if not (
                    current.kind == "artifact"
                    and current.parent_run_id == item.resource_id
                )
            ]
            selected_run_ids.add(item.resource_id)

        selected.append(item)
        if len(selected) >= max(0, delete_budget):
            break

    return selected if delete_budget > 0 else []


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


def repository_created_at(api: GitHubApi) -> dt.datetime:
    repository = api.request(
        "GET", f"https://api.github.com/repos/{api.repository}"
    )
    created_at = repository.get("created_at")
    if not created_at:
        raise RuntimeError("repository metadata did not contain created_at")
    return parse_time(str(created_at))


def list_completed_runs_window(
    api: GitHubApi, start: dt.datetime, end: dt.datetime
) -> list[dict[str, Any]]:
    """List a complete filtered run window, recursively splitting at the API cap."""
    if end <= start:
        return []

    created_range = f"{format_search_time(start)}..{format_search_time(end)}"
    params: dict[str, str | int] = {
        "status": "completed",
        "created": created_range,
    }
    probe = api.request(
        "GET",
        "actions/runs",
        params={**params, "per_page": 1, "page": 1},
    )
    total_count = int(probe.get("total_count", 0))

    if total_count >= RUN_SEARCH_RESULT_CEILING:
        span_seconds = (end - start).total_seconds()
        if span_seconds <= RUN_SEARCH_MIN_SPLIT_SECONDS:
            raise RuntimeError(
                "workflow-run search still reached GitHub's 1,000-result "
                f"ceiling inside {created_range}; refusing incomplete run cleanup"
            )
        midpoint = start + (end - start) / 2
        merged: dict[int, dict[str, Any]] = {}
        for run in list_completed_runs_window(api, start, midpoint):
            merged[int(run["id"])] = run
        for run in list_completed_runs_window(api, midpoint, end):
            merged[int(run["id"])] = run
        return list(merged.values())

    return api.paginate("actions/runs", "workflow_runs", params=params)


def list_old_completed_runs(
    api: GitHubApi, now: dt.datetime, retention_days: int
) -> list[dict[str, Any]]:
    """List every completed run older than the retention cutoff.

    GitHub caps filtered Actions run searches at 1,000 results. To keep the
    inventory complete, search the repository lifetime in bounded time windows
    and recursively split any window that reaches the provider ceiling.
    """
    threshold = cutoff(now, retention_days).replace(microsecond=0)
    cursor = repository_created_at(api).replace(microsecond=0)
    if cursor >= threshold:
        return []

    result: dict[int, dict[str, Any]] = {}
    while cursor < threshold:
        window_end = min(
            cursor + dt.timedelta(days=RUN_SEARCH_WINDOW_DAYS),
            threshold,
        )
        for run in list_completed_runs_window(api, cursor, window_end):
            if parse_time(str(run["created_at"])) < threshold:
                result[int(run["id"])] = run
        cursor = window_end

    return sorted(
        result.values(),
        key=lambda run: (str(run["created_at"]), int(run["id"])),
    )


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
    run_artifact_bytes = artifact_bytes_by_run(artifacts)
    run_candidates = old_completed_run_candidates(
        old_runs,
        now,
        args.run_retention_days,
        run_artifact_bytes,
    )

    all_candidates = cache_candidates + artifact_candidates + run_candidates
    requested_budget = 0 if args.mode == "audit" else args.delete_budget
    actual_budget = compute_delete_budget(api, requested_budget)
    selected = choose_candidates(all_candidates, actual_budget)

    selected_set = set(selected)
    selected_run_ids = {
        candidate.resource_id for candidate in selected if candidate.kind == "run"
    }
    artifacts_covered_by_selected_runs = [
        candidate
        for candidate in artifact_candidates
        if candidate.parent_run_id in selected_run_ids
    ]
    remaining_candidates = [
        candidate
        for candidate in all_candidates
        if candidate not in selected_set
        and not (
            candidate.kind == "artifact"
            and candidate.parent_run_id in selected_run_ids
        )
    ]

    pre = {
        "mode": args.mode,
        "artifact_retention_days": args.artifact_retention_days,
        "run_retention_days": args.run_retention_days,
        "pre_artifact_count": len(artifacts),
        "pre_artifact_bytes": sum_bytes(artifacts),
        "pre_cache_count": int(cache_usage.get("active_caches_count", len(caches))),
        "pre_cache_bytes": int(
            cache_usage.get("active_caches_size_in_bytes", sum_bytes(caches))
        ),
        "eligible_closed_pr_cache_count": len(cache_candidates),
        "eligible_closed_pr_cache_bytes": sum(
            item.size_in_bytes for item in cache_candidates
        ),
        "eligible_old_artifact_count": len(artifact_candidates),
        "eligible_old_artifact_bytes": sum(
            item.size_in_bytes for item in artifact_candidates
        ),
        "eligible_old_run_count": len(run_candidates),
        "selected_for_delete": len(selected),
        "selected_run_artifact_count_covered": len(
            artifacts_covered_by_selected_runs
        ),
        "selected_run_artifact_bytes_covered": sum(
            item.size_in_bytes for item in artifacts_covered_by_selected_runs
        ),
        "delete_budget": actual_budget,
        "rate_limit": api.rate_limit,
        "rate_remaining_before_delete": api.rate_remaining,
    }
    write_summary(pre)

    if args.mode == "audit":
        return 0

    deleted = {"cache": 0, "artifact": 0, "run": 0}
    deleted_bytes = {"cache": 0, "artifact": 0, "run": 0}
    already_absent = {"cache": 0, "artifact": 0, "run": 0}

    for candidate in selected:
        outcome = api.request(
            "DELETE",
            deletion_path(candidate),
            tolerate_not_found=True,
        )
        if isinstance(outcome, dict) and outcome.get("already_absent") is True:
            already_absent[candidate.kind] += 1
        else:
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

    post = {
        "deleted_cache_count": deleted["cache"],
        "deleted_cache_bytes": deleted_bytes["cache"],
        "deleted_artifact_count": deleted["artifact"],
        "deleted_artifact_bytes": deleted_bytes["artifact"],
        "deleted_run_count": deleted["run"],
        "deleted_run_artifact_bytes": deleted_bytes["run"],
        "already_absent_cache_count": already_absent["cache"],
        "already_absent_artifact_count": already_absent["artifact"],
        "already_absent_run_count": already_absent["run"],
        "artifacts_covered_by_deleted_runs": len(artifacts_covered_by_selected_runs),
        "remaining_eligible_candidates_due_to_budget": len(remaining_candidates),
        "post_artifact_count": int(post_artifact_probe.get("total_count", 0)),
        "post_cache_count": int(post_cache_usage.get("active_caches_count", 0)),
        "post_cache_bytes": int(
            post_cache_usage.get("active_caches_size_in_bytes", 0)
        ),
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
    deleted_count = 0
    deleted_bytes = 0
    already_absent = 0
    for candidate in sorted(
        candidates, key=lambda item: item.size_in_bytes, reverse=True
    ):
        outcome = api.request(
            "DELETE",
            deletion_path(candidate),
            tolerate_not_found=True,
        )
        if isinstance(outcome, dict) and outcome.get("already_absent") is True:
            already_absent += 1
        else:
            deleted_count += 1
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
            "deleted_cache_count": deleted_count,
            "deleted_cache_bytes": deleted_bytes,
            "already_absent_cache_count": already_absent,
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
