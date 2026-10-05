#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Callable

from classify_changes import GATES, classify_path, classify_paths, changed_paths

SHA_RE = re.compile(r"^[0-9a-f]{40}$")
API_VERSION = "2022-11-28"
DEFAULT_API_URL = "https://api.github.com"


@dataclass(frozen=True)
class ReuseEvidence:
    run_id: int
    head_sha: str
    job_name: str


@dataclass(frozen=True)
class ReuseDecision:
    required: bool
    decision: str
    reason: str
    material_digest: str = ""
    generation_paths: tuple[str, ...] = ()
    reused_run_id: int | None = None
    reused_head_sha: str = ""
    reused_job_name: str = ""


class GitHubApiError(RuntimeError):
    pass


class GitHubActionsClient:
    def __init__(
        self,
        repository: str,
        token: str = "",
        *,
        api_url: str = DEFAULT_API_URL,
        opener: Callable[..., object] = urllib.request.urlopen,
    ) -> None:
        if "/" not in repository:
            raise ValueError("repository must be owner/name")
        self.repository = repository
        self.token = token
        self.api_url = api_url.rstrip("/")
        self.opener = opener

    def _get_json(self, path: str) -> dict[str, object]:
        url = f"{self.api_url}{path}"
        headers = {
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": API_VERSION,
            "User-Agent": "oteryn-pr-generation-reuse",
        }
        if self.token:
            headers["Authorization"] = f"Bearer {self.token}"
        request = urllib.request.Request(url, headers=headers)
        try:
            with self.opener(request, timeout=20) as response:
                payload = response.read().decode("utf-8")
        except (OSError, urllib.error.URLError, urllib.error.HTTPError) as exc:
            raise GitHubApiError(f"GitHub API request failed: {exc}") from exc
        try:
            data = json.loads(payload)
        except json.JSONDecodeError as exc:
            raise GitHubApiError("GitHub API returned invalid JSON") from exc
        if not isinstance(data, dict):
            raise GitHubApiError("GitHub API returned non-object JSON")
        return data

    def successful_pr_runs(self, workflow_file: str) -> list[dict[str, object]]:
        encoded = urllib.parse.quote(workflow_file, safe="")
        runs: list[dict[str, object]] = []
        for page in range(1, 11):
            query = urllib.parse.urlencode(
                {
                    "event": "pull_request",
                    "status": "success",
                    "per_page": 100,
                    "page": page,
                }
            )
            data = self._get_json(
                f"/repos/{self.repository}/actions/workflows/{encoded}/runs?{query}"
            )
            page_runs = data.get("workflow_runs")
            if not isinstance(page_runs, list):
                raise GitHubApiError("workflow runs response is missing workflow_runs")
            runs.extend(item for item in page_runs if isinstance(item, dict))
            if len(page_runs) < 100:
                break
        return runs

    def jobs(self, run_id: int) -> list[dict[str, object]]:
        jobs: list[dict[str, object]] = []
        for page in range(1, 11):
            query = urllib.parse.urlencode(
                {"filter": "all", "per_page": 100, "page": page}
            )
            data = self._get_json(
                f"/repos/{self.repository}/actions/runs/{run_id}/jobs?{query}"
            )
            page_jobs = data.get("jobs")
            if not isinstance(page_jobs, list):
                raise GitHubApiError("workflow jobs response is missing jobs")
            jobs.extend(item for item in page_jobs if isinstance(item, dict))
            if len(page_jobs) < 100:
                break
        return jobs


def _git_output(args: list[str]) -> str:
    return subprocess.check_output(args, text=True)


def commit_exists(sha: str) -> bool:
    if SHA_RE.fullmatch(sha) is None:
        return False
    result = subprocess.run(
        ["git", "cat-file", "-e", f"{sha}^{{commit}}"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    return result.returncode == 0


def is_ancestor(ancestor: str, descendant: str) -> bool:
    if not commit_exists(ancestor) or not commit_exists(descendant):
        return False
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", ancestor, descendant],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    return result.returncode == 0


def material_tree_entries(commit: str, gate: str) -> list[str]:
    if gate not in GATES:
        raise ValueError(f"unknown gate: {gate}")
    if not commit_exists(commit):
        raise ValueError(f"commit unavailable: {commit}")
    output = _git_output(["git", "ls-tree", "-r", "--full-tree", commit])
    entries: list[str] = []
    for raw in output.splitlines():
        if "\t" not in raw:
            raise ValueError(f"unexpected git ls-tree entry: {raw!r}")
        metadata, path = raw.split("\t", 1)
        if len(metadata.split()) != 3:
            raise ValueError(f"unexpected git ls-tree metadata: {metadata!r}")
        if gate in classify_path(path).gates:
            entries.append(f"{metadata}\t{path}")
    return sorted(entries)


def material_tree_digest(commit: str, gate: str) -> str:
    entries = material_tree_entries(commit, gate)
    payload = "".join(f"{entry}\n" for entry in entries).encode("utf-8")
    return f"sha256:{hashlib.sha256(payload).hexdigest()}"


def _run_matches_pr(run: dict[str, object], pr_number: int) -> bool:
    pulls = run.get("pull_requests")
    if not isinstance(pulls, list):
        return False
    for pull in pulls:
        if not isinstance(pull, dict):
            continue
        try:
            if int(pull.get("number", 0)) == pr_number:
                return True
        except (TypeError, ValueError):
            continue
    return False


def find_reusable_evidence(
    *,
    client: GitHubActionsClient,
    workflow_file: str,
    evidence_job: str,
    pr_number: int,
    gate: str,
    current_head: str,
    current_digest: str,
    digest_fn: Callable[[str, str], str] = material_tree_digest,
    ancestor_fn: Callable[[str, str], bool] = is_ancestor,
) -> ReuseEvidence | None:
    runs = client.successful_pr_runs(workflow_file)
    runs.sort(key=lambda item: int(item.get("id", 0) or 0), reverse=True)

    for run in runs:
        if not _run_matches_pr(run, pr_number):
            continue
        candidate_sha = str(run.get("head_sha", ""))
        if SHA_RE.fullmatch(candidate_sha) is None or candidate_sha == current_head:
            continue
        if not ancestor_fn(candidate_sha, current_head):
            continue
        try:
            candidate_digest = digest_fn(candidate_sha, gate)
        except (OSError, subprocess.CalledProcessError, ValueError):
            continue
        if candidate_digest != current_digest:
            continue
        try:
            run_id = int(run.get("id", 0))
        except (TypeError, ValueError):
            continue
        if run_id <= 0:
            continue
        for job in client.jobs(run_id):
            if (
                str(job.get("name", "")) == evidence_job
                and str(job.get("conclusion", "")) == "success"
            ):
                return ReuseEvidence(run_id, candidate_sha, evidence_job)
    return None


def resolve_decision(
    *,
    gate: str,
    workflow_file: str,
    evidence_job: str,
    event_name: str,
    event_action: str,
    accumulated_required: str,
    before: str,
    after: str,
    head: str,
    pr_number: str,
    repository: str,
    token: str = "",
    changed_paths_fn: Callable[[str, str], list[str]] = changed_paths,
    digest_fn: Callable[[str, str], str] = material_tree_digest,
    ancestor_fn: Callable[[str, str], bool] = is_ancestor,
    evidence_lookup_fn: Callable[..., ReuseEvidence | None] | None = None,
) -> ReuseDecision:
    if gate not in GATES:
        return ReuseDecision(True, "run", "unknown_gate")
    if accumulated_required not in {"true", "false"}:
        return ReuseDecision(True, "run", "ambiguous_accumulated_classification")
    if accumulated_required == "false":
        return ReuseDecision(False, "skip", "accumulated_scope_not_required")

    if event_name != "pull_request":
        return ReuseDecision(True, "run", "non_pull_request_event")
    if event_action != "synchronize":
        return ReuseDecision(True, "run", "non_synchronize_pull_request")
    if any(SHA_RE.fullmatch(value) is None for value in (before, after, head)):
        return ReuseDecision(True, "run", "missing_or_invalid_generation_sha")
    if after != head:
        return ReuseDecision(True, "run", "event_after_does_not_match_head")
    if before == head:
        return ReuseDecision(True, "run", "empty_or_duplicate_generation")
    if not repository or "/" not in repository:
        return ReuseDecision(True, "run", "invalid_repository_identity")
    try:
        parsed_pr = int(pr_number)
    except (TypeError, ValueError):
        return ReuseDecision(True, "run", "invalid_pull_request_identity")
    if parsed_pr <= 0:
        return ReuseDecision(True, "run", "invalid_pull_request_identity")

    try:
        if not ancestor_fn(before, head):
            return ReuseDecision(True, "run", "generation_not_fast_forward")
        generation_paths = changed_paths_fn(before, head)
        generation = classify_paths(generation_paths)
    except (OSError, subprocess.CalledProcessError, ValueError):
        return ReuseDecision(True, "run", "generation_diff_unavailable")

    gates = generation["gates"]
    assert isinstance(gates, dict)
    if bool(gates[gate]):
        return ReuseDecision(
            True,
            "run",
            "generation_changes_gate_material",
            generation_paths=tuple(generation_paths),
        )

    try:
        before_digest = digest_fn(before, gate)
        head_digest = digest_fn(head, gate)
    except (OSError, subprocess.CalledProcessError, ValueError):
        return ReuseDecision(
            True,
            "run",
            "material_digest_unavailable",
            generation_paths=tuple(generation_paths),
        )
    if before_digest != head_digest:
        return ReuseDecision(
            True,
            "run",
            "material_digest_changed",
            material_digest=head_digest,
            generation_paths=tuple(generation_paths),
        )

    if evidence_lookup_fn is None:
        client = GitHubActionsClient(repository, token)
        try:
            evidence = find_reusable_evidence(
                client=client,
                workflow_file=workflow_file,
                evidence_job=evidence_job,
                pr_number=parsed_pr,
                gate=gate,
                current_head=head,
                current_digest=head_digest,
                digest_fn=digest_fn,
                ancestor_fn=ancestor_fn,
            )
        except (GitHubApiError, OSError, urllib.error.URLError, ValueError):
            evidence = None
    else:
        try:
            evidence = evidence_lookup_fn(
                workflow_file=workflow_file,
                evidence_job=evidence_job,
                pr_number=parsed_pr,
                gate=gate,
                current_head=head,
                current_digest=head_digest,
            )
        except (GitHubApiError, OSError, urllib.error.URLError, ValueError):
            evidence = None

    if evidence is None:
        return ReuseDecision(
            True,
            "run",
            "no_proven_prior_success",
            material_digest=head_digest,
            generation_paths=tuple(generation_paths),
        )

    return ReuseDecision(
        False,
        "reuse",
        "prior_success_same_material_tree",
        material_digest=head_digest,
        generation_paths=tuple(generation_paths),
        reused_run_id=evidence.run_id,
        reused_head_sha=evidence.head_sha,
        reused_job_name=evidence.job_name,
    )


def write_github_output(path: Path, decision: ReuseDecision) -> None:
    values = {
        "required": "true" if decision.required else "false",
        "decision": decision.decision,
        "reason": decision.reason,
        "material_digest": decision.material_digest,
        "generation_paths_json": json.dumps(
            list(decision.generation_paths), separators=(",", ":")
        ),
        "reused_run_id": "" if decision.reused_run_id is None else str(decision.reused_run_id),
        "reused_head_sha": decision.reused_head_sha,
        "reused_job_name": decision.reused_job_name,
    }
    with path.open("a", encoding="utf-8") as handle:
        for key, value in values.items():
            handle.write(f"{key}={value}\n")


def write_summary(
    path: Path,
    *,
    gate: str,
    workflow_file: str,
    evidence_job: str,
    head: str,
    decision: ReuseDecision,
) -> None:
    lines = [
        "### Pull-request generation evidence",
        "",
        f"- gate: `{gate}`",
        f"- workflow: `{workflow_file}`",
        f"- evidence job: `{evidence_job}`",
        f"- exact final head: `{head or 'UNKNOWN'}`",
        f"- decision: `{'RUN' if decision.required else decision.decision.upper()}`",
        f"- reason: `{decision.reason}`",
        f"- material digest: `{decision.material_digest or 'NOT_COMPUTED'}`",
        f"- latest-generation changed paths: `{len(decision.generation_paths)}`",
    ]
    if decision.reused_run_id is not None:
        lines.extend(
            [
                f"- reused source head: `{decision.reused_head_sha}`",
                f"- reused workflow run: `{decision.reused_run_id}`",
                f"- reused evidence job: `{decision.reused_job_name}`",
            ]
        )
    lines.extend(
        [
            "- ambiguity, missing evidence, API failure or material change always falls back to RUN.",
            "",
        ]
    )
    with path.open("a", encoding="utf-8") as handle:
        handle.write("\n".join(lines))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Fail-closed heavy-CI reuse resolver for pull-request generations."
    )
    parser.add_argument("--gate", required=True, choices=GATES)
    parser.add_argument("--workflow-file", required=True)
    parser.add_argument("--evidence-job", required=True)
    parser.add_argument("--event-name", required=True)
    parser.add_argument("--event-action", default="")
    parser.add_argument("--accumulated-required", required=True)
    parser.add_argument("--before", default="")
    parser.add_argument("--after", default="")
    parser.add_argument("--head", required=True)
    parser.add_argument("--pr-number", default="")
    parser.add_argument("--repository", required=True)
    parser.add_argument("--token-env", default="GITHUB_TOKEN")
    parser.add_argument("--github-output", type=Path)
    parser.add_argument("--summary", type=Path)
    parser.add_argument("--json", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    decision = resolve_decision(
        gate=args.gate,
        workflow_file=args.workflow_file,
        evidence_job=args.evidence_job,
        event_name=args.event_name,
        event_action=args.event_action,
        accumulated_required=args.accumulated_required,
        before=args.before,
        after=args.after,
        head=args.head,
        pr_number=args.pr_number,
        repository=args.repository,
        token=os.environ.get(args.token_env, ""),
    )
    if args.github_output:
        write_github_output(args.github_output, decision)
    if args.summary:
        write_summary(
            args.summary,
            gate=args.gate,
            workflow_file=args.workflow_file,
            evidence_job=args.evidence_job,
            head=args.head,
            decision=decision,
        )
    if args.json or (not args.github_output and not args.summary):
        print(json.dumps(asdict(decision), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
