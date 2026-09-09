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
import uuid
from dataclasses import asdict, dataclass
from typing import Any, Mapping, Protocol

EXPECTED_REPOSITORY = "Oteryn/Oteryn-Platform"
EXPECTED_BASE = "main"
REQUIRED_CHECK = "platform-gate"
MERGE_ACTION = "merge_queue"
SHA_RE = re.compile(r"^[0-9a-fA-F]{40}$")
API_VERSION = "2026-03-10"


class GitHubRequestError(RuntimeError):
    pass


@dataclass(frozen=True)
class Response:
    status: int
    body: Mapping[str, Any]


class Client(Protocol):
    def rest(
        self,
        method: str,
        path: str,
        *,
        body: Mapping[str, Any] | None = None,
        allowed_statuses: tuple[int, ...] = (200,),
    ) -> Response: ...


class GitHubClient:
    def __init__(self, token: str, api_url: str = "https://api.github.com") -> None:
        if not token.strip():
            raise ValueError("GitHub token is required")
        self.token = token.strip()
        self.api_url = api_url.rstrip("/")

    def rest(
        self,
        method: str,
        path: str,
        *,
        body: Mapping[str, Any] | None = None,
        allowed_statuses: tuple[int, ...] = (200,),
    ) -> Response:
        data = None if body is None else json.dumps(body).encode("utf-8")
        request = urllib.request.Request(
            f"{self.api_url}{path}",
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
            with urllib.request.urlopen(request, timeout=20) as raw_response:
                status = raw_response.status
                raw = raw_response.read().decode("utf-8")
        except urllib.error.HTTPError as exc:
            status = exc.code
            raw = exc.read().decode("utf-8", errors="replace")
            if status not in allowed_statuses:
                raise GitHubRequestError(
                    f"GitHub API {method} {path} failed with HTTP {status}: {raw}"
                ) from exc
        except urllib.error.URLError as exc:
            raise GitHubRequestError(f"GitHub API {method} {path} failed: {exc.reason}") from exc

        if status not in allowed_statuses:
            raise GitHubRequestError(f"GitHub API {method} {path} returned unexpected HTTP {status}")
        if not raw:
            parsed: Any = {}
        else:
            try:
                parsed = json.loads(raw)
            except json.JSONDecodeError as exc:
                raise GitHubRequestError(
                    f"GitHub API returned invalid JSON for {method} {path}"
                ) from exc
        if not isinstance(parsed, dict):
            raise GitHubRequestError(f"GitHub API returned non-object JSON for {method} {path}")
        return Response(status=status, body=parsed)


@dataclass(frozen=True)
class QualifiedPullRequest:
    repository: str
    number: int
    base: str
    head_sha: str


@dataclass(frozen=True)
class Evidence:
    repository: str
    pr_number: int
    base: str
    head_sha: str
    merge_action: str
    status: str
    server_uuid: str
    sequence: int


class ExecutorSequence:
    """Process-local causal ordering; deliberately unavailable as CLI/workflow input."""

    def __init__(self) -> None:
        self._value = 0

    def next(self) -> int:
        self._value += 1
        return self._value


def _response_body(response: Response) -> Mapping[str, Any]:
    return response.body


def normalize_inputs(repository: str, pr_number: int, expected_head_sha: str) -> tuple[str, int, str]:
    repository = repository.strip()
    expected_head_sha = expected_head_sha.strip().lower()
    if repository != EXPECTED_REPOSITORY:
        raise ValueError(f"repository must be exactly {EXPECTED_REPOSITORY}")
    if isinstance(pr_number, bool) or pr_number <= 0:
        raise ValueError("PR number must be positive")
    if not SHA_RE.fullmatch(expected_head_sha):
        raise ValueError("expected head SHA must be a full 40-character hexadecimal commit SHA")
    return repository, pr_number, expected_head_sha


def _validate_target(pull: Mapping[str, Any], repository: str, expected_head_sha: str) -> None:
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


def _validate_target_identity(
    pull: Mapping[str, Any], repository: str, expected_head_sha: str
) -> None:
    base = pull.get("base")
    head = pull.get("head")
    if not isinstance(base, dict) or base.get("ref") != EXPECTED_BASE:
        raise ValueError(f"pull request base must be exactly {EXPECTED_BASE}")
    if not isinstance(head, dict) or str(head.get("sha") or "").lower() != expected_head_sha:
        raise ValueError("pull request reconciliation head does not match qualified head")
    head_repo = head.get("repo")
    if not isinstance(head_repo, dict) or head_repo.get("full_name") != repository:
        raise ValueError("pull request must use a same-repository head branch")


def qualify_pull_request(
    client: Client, *, repository: str, pr_number: int, expected_head_sha: str
) -> QualifiedPullRequest:
    repository, pr_number, expected_head_sha = normalize_inputs(repository, pr_number, expected_head_sha)
    owner, name = repository.split("/", 1)
    pull = _response_body(client.rest("GET", f"/repos/{owner}/{name}/pulls/{pr_number}"))
    _validate_target(pull, repository, expected_head_sha)

    params = urllib.parse.urlencode({"check_name": REQUIRED_CHECK, "filter": "latest", "per_page": "100"})
    checks = _response_body(
        client.rest("GET", f"/repos/{owner}/{name}/commits/{expected_head_sha}/check-runs?{params}")
    )
    raw_runs = checks.get("check_runs")
    if not isinstance(raw_runs, list) or not raw_runs:
        raise ValueError(f"no exact-head {REQUIRED_CHECK} check run was found")
    matching = [
        run for run in raw_runs
        if isinstance(run, dict) and run.get("name") == REQUIRED_CHECK
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
    return QualifiedPullRequest(repository, pr_number, EXPECTED_BASE, expected_head_sha)


def _valid_uuid(value: Any) -> str:
    if not isinstance(value, str):
        raise GitHubRequestError("merge-async response is missing a valid server UUID")
    try:
        parsed = uuid.UUID(value)
    except (ValueError, AttributeError) as exc:
        raise GitHubRequestError("merge-async response is missing a valid server UUID") from exc
    if str(parsed) != value.lower():
        raise GitHubRequestError("merge-async response is missing a canonical server UUID")
    return value.lower()


def _server_fields(payload: Mapping[str, Any], *, require_uuid: bool) -> tuple[str, str, str, str]:
    status = payload.get("status")
    details = payload.get("details")
    if not isinstance(status, str) or not status or not isinstance(details, dict):
        raise GitHubRequestError("merge-async response is missing status/details")
    server_uuid = _valid_uuid(details.get("uuid")) if require_uuid else ""
    action = details.get("merge_action")
    head = str(details.get("expected_head_sha") or "").lower()
    return status, server_uuid, str(action or ""), head


def _sequence(sequence: ExecutorSequence) -> int:
    value = sequence.next()
    if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
        raise GitHubRequestError("executor sequence must be a positive integer")
    return value


def _readback(
    async_client: Client,
    target_client: Client,
    pull: QualifiedPullRequest,
    server_uuid: str,
    sequence: ExecutorSequence,
    prior_sequence: int,
    *,
    require_eligible_target: bool,
) -> Evidence:
    owner, name = pull.repository.split("/", 1)
    async_result = _response_body(async_client.rest(
        "GET", f"/repos/{owner}/{name}/pulls/{pull.number}/merge-async/{server_uuid}"
    ))
    status, readback_uuid, action, head = _server_fields(async_result, require_uuid=True)
    if readback_uuid != server_uuid:
        raise GitHubRequestError("merge-async readback UUID does not match accepted server UUID")
    if action != MERGE_ACTION:
        raise GitHubRequestError("merge-async readback does not bind explicit merge_queue action")
    if head != pull.head_sha:
        raise GitHubRequestError("merge-async readback head does not match qualified head")

    fresh_pull = _response_body(target_client.rest("GET", f"/repos/{owner}/{name}/pulls/{pull.number}"))
    try:
        if require_eligible_target:
            _validate_target(fresh_pull, pull.repository, pull.head_sha)
        else:
            _validate_target_identity(fresh_pull, pull.repository, pull.head_sha)
    except ValueError as exc:
        raise GitHubRequestError(f"post-submission target mismatch: {exc}") from exc
    readback_sequence = _sequence(sequence)
    if readback_sequence <= prior_sequence:
        raise GitHubRequestError("post-submission executor sequence must be strictly greater than receipt sequence")
    return Evidence(
        pull.repository, pull.number, pull.base, pull.head_sha, action,
        status, readback_uuid, readback_sequence,
    )


def submit_merge_queue(
    mutation_client: Client,
    pull: QualifiedPullRequest,
    *,
    target_client: Client | None = None,
) -> Mapping[str, Any]:
    target_client = target_client or mutation_client
    sequence = ExecutorSequence()
    owner, name = pull.repository.split("/", 1)
    response = mutation_client.rest(
        "PUT",
        f"/repos/{owner}/{name}/pulls/{pull.number}/merge-async",
        body={"sha": pull.head_sha, "merge_action": MERGE_ACTION},
        allowed_statuses=(200, 202, 400, 403, 404, 409, 422),
    )
    if response.status in (400, 422):
        raise GitHubRequestError(f"REQUEST_REJECTED_HTTP_{response.status}: merge-async rejected the request")
    if response.status in (403, 404):
        raise GitHubRequestError(
            f"BLOCKED_CAPABILITY_UNAVAILABLE: merge-async returned HTTP {response.status}"
        )

    if response.status == 202:
        status, server_uuid, action, head = _server_fields(response.body, require_uuid=True)
        if action != MERGE_ACTION or head != pull.head_sha:
            raise GitHubRequestError("merge-async acceptance does not match qualified head and merge_queue action")
        receipt_sequence = _sequence(sequence)
        receipt = Evidence(
            pull.repository, pull.number, pull.base, pull.head_sha, action,
            status, server_uuid, receipt_sequence,
        )
        readback = _readback(
            mutation_client, target_client, pull, server_uuid, sequence, receipt_sequence,
            require_eligible_target=True,
        )
        return {"result": "REQUEST_ACCEPTED_NON_TERMINAL", "receipt": asdict(receipt), "readback": asdict(readback)}

    details = response.body.get("details")
    possible_uuid = details.get("uuid") if isinstance(details, dict) else None
    readback = None
    if possible_uuid is not None:
        server_uuid = _valid_uuid(possible_uuid)
        readback = asdict(_readback(
            mutation_client, target_client, pull, server_uuid, sequence, 0,
            require_eligible_target=False,
        ))
    else:
        fresh = _response_body(target_client.rest("GET", f"/repos/{owner}/{name}/pulls/{pull.number}"))
        _validate_target_identity(fresh, pull.repository, pull.head_sha)
    return {
        "result": "RECONCILIATION_REQUIRED",
        "http_status": response.status,
        "accepted": False,
        "receipt": None,
        "readback": readback,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Fail-closed native merge-async Merge Queue executor.")
    parser.add_argument("--repository", required=True)
    parser.add_argument("--pr-number", required=True, type=int)
    parser.add_argument("--expected-head-sha", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        api_url = os.environ.get("GITHUB_API_URL", "https://api.github.com")
        read_client = GitHubClient(os.environ.get("MQ_GITHUB_READ_TOKEN", ""), api_url)
        mutation_token = os.environ.get("MQ_GITHUB_MUTATION_TOKEN", "")
        if not mutation_token.strip():
            raise GitHubRequestError(
                "BLOCKED_CAPABILITY_UNAVAILABLE: OTERYN_MQ_TOKEN is not configured"
            )
        mutation_client = GitHubClient(mutation_token, api_url)
        pull = qualify_pull_request(
            read_client, repository=args.repository, pr_number=args.pr_number,
            expected_head_sha=args.expected_head_sha,
        )
        result = submit_merge_queue(mutation_client, pull, target_client=read_client)
    except (ValueError, GitHubRequestError) as exc:
        print(f"Merge Queue enqueue rejected: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
