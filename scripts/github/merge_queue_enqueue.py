#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from typing import Any, Mapping, Protocol

EXPECTED_REPOSITORY = "Oteryn/Oteryn-Platform"
EXPECTED_BASE = "main"
REQUIRED_CHECK = "platform-gate"
SHA_RE = re.compile(r"^[0-9a-fA-F]{40}$")
API_VERSION = "2022-11-28"


class GitHubRequestError(RuntimeError):
    pass


class Client(Protocol):
    def rest(self, method: str, path: str) -> Mapping[str, Any]: ...

    def graphql(self, query: str, variables: Mapping[str, Any]) -> Mapping[str, Any]: ...


class GitHubClient:
    def __init__(self, token: str, api_url: str = "https://api.github.com") -> None:
        if not token.strip():
            raise ValueError("GitHub token is required")
        self.token = token.strip()
        self.api_url = api_url.rstrip("/")

    def _request(self, url: str, *, method: str, body: Mapping[str, Any] | None = None) -> Mapping[str, Any]:
        data = None if body is None else json.dumps(body).encode("utf-8")
        request = urllib.request.Request(
            url,
            data=data,
            method=method,
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {self.token}",
                "Content-Type": "application/json",
                "User-Agent": "oteryn-merge-queue-enqueue",
                "X-GitHub-Api-Version": API_VERSION,
            },
        )
        try:
            with urllib.request.urlopen(request, timeout=20) as response:
                raw = response.read().decode("utf-8")
        except urllib.error.HTTPError as exc:
            detail = exc.read().decode("utf-8", errors="replace")
            raise GitHubRequestError(f"GitHub API {method} {url} failed with HTTP {exc.code}: {detail}") from exc
        except urllib.error.URLError as exc:
            raise GitHubRequestError(f"GitHub API {method} {url} failed: {exc.reason}") from exc
        if not raw:
            return {}
        try:
            parsed = json.loads(raw)
        except json.JSONDecodeError as exc:
            raise GitHubRequestError(f"GitHub API returned invalid JSON for {method} {url}") from exc
        if not isinstance(parsed, dict):
            raise GitHubRequestError(f"GitHub API returned non-object JSON for {method} {url}")
        return parsed

    def rest(self, method: str, path: str) -> Mapping[str, Any]:
        return self._request(f"{self.api_url}{path}", method=method)

    def graphql(self, query: str, variables: Mapping[str, Any]) -> Mapping[str, Any]:
        response = self._request(
            f"{self.api_url}/graphql",
            method="POST",
            body={"query": query, "variables": dict(variables)},
        )
        errors = response.get("errors")
        if errors:
            raise GitHubRequestError(f"GitHub GraphQL returned errors: {json.dumps(errors, sort_keys=True)}")
        data = response.get("data")
        if not isinstance(data, dict):
            raise GitHubRequestError("GitHub GraphQL response is missing object data")
        return data


@dataclass(frozen=True)
class QualifiedPullRequest:
    node_id: str
    number: int
    head_sha: str


def normalize_inputs(repository: str, pr_number: int, expected_head_sha: str) -> tuple[str, int, str]:
    repository = repository.strip()
    expected_head_sha = expected_head_sha.strip().lower()
    if repository != EXPECTED_REPOSITORY:
        raise ValueError(f"repository must be exactly {EXPECTED_REPOSITORY}")
    if pr_number <= 0:
        raise ValueError("PR number must be positive")
    if not SHA_RE.fullmatch(expected_head_sha):
        raise ValueError("expected head SHA must be a full 40-character hexadecimal commit SHA")
    return repository, pr_number, expected_head_sha


def qualify_pull_request(
    client: Client,
    *,
    repository: str,
    pr_number: int,
    expected_head_sha: str,
) -> QualifiedPullRequest:
    repository, pr_number, expected_head_sha = normalize_inputs(repository, pr_number, expected_head_sha)
    owner, name = repository.split("/", 1)
    pull = client.rest("GET", f"/repos/{owner}/{name}/pulls/{pr_number}")

    if pull.get("state") != "open" or pull.get("merged") is True:
        raise ValueError("pull request must be open and unmerged")
    if pull.get("draft") is not False:
        raise ValueError("pull request must be ready for review, not draft")

    base = pull.get("base")
    if not isinstance(base, dict) or base.get("ref") != EXPECTED_BASE:
        raise ValueError(f"pull request base must be exactly {EXPECTED_BASE}")

    head = pull.get("head")
    if not isinstance(head, dict):
        raise ValueError("pull request head metadata is missing")
    actual_head_sha = str(head.get("sha") or "").lower()
    if actual_head_sha != expected_head_sha:
        raise ValueError(
            f"pull request head changed: expected {expected_head_sha}, found {actual_head_sha or 'UNKNOWN'}"
        )
    head_repo = head.get("repo")
    if not isinstance(head_repo, dict) or head_repo.get("full_name") != repository:
        raise ValueError("pull request must use a same-repository head branch")

    node_id = pull.get("node_id")
    if not isinstance(node_id, str) or not node_id:
        raise ValueError("pull request GraphQL node ID is missing")

    params = urllib.parse.urlencode(
        {"check_name": REQUIRED_CHECK, "filter": "latest", "per_page": "100"}
    )
    checks = client.rest(
        "GET",
        f"/repos/{owner}/{name}/commits/{expected_head_sha}/check-runs?{params}",
    )
    raw_runs = checks.get("check_runs")
    if not isinstance(raw_runs, list) or not raw_runs:
        raise ValueError(f"no exact-head {REQUIRED_CHECK} check run was found")

    matching = [
        run
        for run in raw_runs
        if isinstance(run, dict)
        and run.get("name") == REQUIRED_CHECK
        and str(run.get("head_sha") or "").lower() == expected_head_sha
    ]
    if not matching:
        raise ValueError(f"no {REQUIRED_CHECK} check run matches the exact expected head SHA")
    latest = max(matching, key=lambda run: int(run.get("id") or 0))
    if latest.get("status") != "completed" or latest.get("conclusion") != "success":
        raise ValueError(
            f"latest exact-head {REQUIRED_CHECK} must be completed/success, found "
            f"{latest.get('status')}/{latest.get('conclusion')}"
        )

    return QualifiedPullRequest(node_id=node_id, number=pr_number, head_sha=expected_head_sha)


ENQUEUE_MUTATION = """
mutation EnqueuePullRequest($pullRequestId: ID!, $expectedHeadOid: GitObjectID!) {
  enqueuePullRequest(input: {
    pullRequestId: $pullRequestId,
    expectedHeadOid: $expectedHeadOid
  }) {
    mergeQueueEntry {
      id
      position
      state
    }
  }
}
""".strip()


def enqueue_pull_request(client: Client, pull: QualifiedPullRequest) -> Mapping[str, Any]:
    data = client.graphql(
        ENQUEUE_MUTATION,
        {"pullRequestId": pull.node_id, "expectedHeadOid": pull.head_sha},
    )
    payload = data.get("enqueuePullRequest")
    if not isinstance(payload, dict):
        raise GitHubRequestError("enqueuePullRequest response payload is missing")
    entry = payload.get("mergeQueueEntry")
    if not isinstance(entry, dict) or not entry.get("id"):
        raise GitHubRequestError("enqueuePullRequest did not return a merge queue entry")
    return entry


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Fail-closed repository-native enqueue into the Oteryn Platform Merge Queue."
    )
    parser.add_argument("--repository", required=True)
    parser.add_argument("--pr-number", required=True, type=int)
    parser.add_argument("--expected-head-sha", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    token = os.environ.get("MQ_GITHUB_TOKEN", "")
    api_url = os.environ.get("GITHUB_API_URL", "https://api.github.com")
    try:
        client = GitHubClient(token, api_url)
        pull = qualify_pull_request(
            client,
            repository=args.repository,
            pr_number=args.pr_number,
            expected_head_sha=args.expected_head_sha,
        )
        entry = enqueue_pull_request(client, pull)
    except (ValueError, GitHubRequestError) as exc:
        print(f"Merge Queue enqueue rejected: {exc}", file=sys.stderr)
        return 1

    print(
        json.dumps(
            {
                "repository": EXPECTED_REPOSITORY,
                "pr_number": pull.number,
                "expected_head_sha": pull.head_sha,
                "merge_queue_entry_id": entry.get("id"),
                "position": entry.get("position"),
                "state": entry.get("state"),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
