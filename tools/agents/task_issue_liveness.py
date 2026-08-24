#!/usr/bin/env python3
"""Validate active task packets against their governing live GitHub Issue."""

from __future__ import annotations

import argparse
import dataclasses
import json
import os
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, Protocol

DEFAULT_CONTRACT = Path("docs/agents/GOVERNANCE_CONTRACT.json")
DEFAULT_ACTIVE = Path("docs/agents/tasks/active")


class IssueLivenessError(RuntimeError):
    pass


class GitHubState(Protocol):
    def get_issue(self, number: int) -> dict[str, Any]: ...


@dataclasses.dataclass(frozen=True)
class Policy:
    schema_version: int
    governing_issue_field: str
    allowed_issue_states: frozenset[str]
    report_schema_version: int


@dataclasses.dataclass(frozen=True)
class Finding:
    severity: str
    code: str
    message: str

    def as_dict(self) -> dict[str, str]:
        return dataclasses.asdict(self)


@dataclasses.dataclass(frozen=True)
class TaskResult:
    task_id: str
    path: str
    governing_issue: str
    live_valid: bool
    issue_state: str
    findings: tuple[Finding, ...]

    def as_dict(self) -> dict[str, object]:
        value = dataclasses.asdict(self)
        value["findings"] = [item.as_dict() for item in self.findings]
        return value


class GitHubClient:
    def __init__(
        self,
        repository: str,
        token: str,
        api_url: str = "https://api.github.com",
        timeout: float = 30.0,
    ) -> None:
        if repository.count("/") != 1:
            raise IssueLivenessError("repository must use owner/name form")
        if not token:
            raise IssueLivenessError("GitHub token is required for live Issue validation")
        self.repository = repository
        self.token = token
        self.api_url = api_url.rstrip("/")
        self.timeout = timeout

    def get_issue(self, number: int) -> dict[str, Any]:
        path = f"/repos/{self.repository}/issues/{number}"
        request = urllib.request.Request(
            f"{self.api_url}{path}",
            method="GET",
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {self.token}",
                "User-Agent": "oteryn-task-issue-liveness/1",
                "X-GitHub-Api-Version": "2022-11-28",
            },
        )
        try:
            with urllib.request.urlopen(request, timeout=self.timeout) as response:
                raw = response.read().decode("utf-8")
        except urllib.error.HTTPError as exc:
            raise IssueLivenessError(
                f"GitHub Issue state unavailable: GET {path} returned HTTP {exc.code}"
            ) from exc
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            raise IssueLivenessError(
                f"GitHub Issue state unavailable: GET {path} failed"
            ) from exc
        try:
            payload = json.loads(raw)
        except json.JSONDecodeError as exc:
            raise IssueLivenessError(
                f"GitHub Issue state unavailable: GET {path} returned invalid JSON"
            ) from exc
        if not isinstance(payload, dict):
            raise IssueLivenessError(
                f"GitHub Issue state unavailable: GET {path} returned a non-object"
            )
        return payload


def _scalar(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {'"', "'"}:
        return value[1:-1]
    return value


def frontmatter_scalars(text: str) -> dict[str, str]:
    values: dict[str, str] = {}
    if not text.startswith("---"):
        return values
    end = text.find("\n---", 3)
    if end < 0:
        return values
    for raw in text[3:end].splitlines():
        if not raw or raw[0].isspace() or raw.lstrip().startswith("#") or ":" not in raw:
            continue
        key, value = raw.split(":", 1)
        key = key.strip()
        value = _scalar(value)
        if key and value and value not in {"[]", "{}"}:
            values[key] = value
    return values


def load_policy(path: Path = DEFAULT_CONTRACT) -> Policy:
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
        policy = raw["live_issue_liveness"]
    except (OSError, json.JSONDecodeError, KeyError, TypeError) as exc:
        raise IssueLivenessError(f"{path}: invalid live_issue_liveness policy") from exc
    required = {
        "schema_version",
        "governing_issue_field",
        "allowed_issue_states",
        "report_schema_version",
    }
    if not isinstance(policy, dict) or set(policy) != required:
        raise IssueLivenessError(
            f"{path}: live_issue_liveness policy fields do not match contract"
        )
    for key in ("schema_version", "report_schema_version"):
        if not isinstance(policy[key], int) or policy[key] < 1:
            raise IssueLivenessError(
                f"{path}: live_issue_liveness.{key} must be a positive integer"
            )
    field = policy["governing_issue_field"]
    if not isinstance(field, str) or not field.strip():
        raise IssueLivenessError(
            f"{path}: live_issue_liveness.governing_issue_field must be non-empty"
        )
    states = policy["allowed_issue_states"]
    if not isinstance(states, list) or not states or not all(
        isinstance(item, str) and item for item in states
    ):
        raise IssueLivenessError(
            f"{path}: live_issue_liveness.allowed_issue_states must be a non-empty string list"
        )
    return Policy(
        schema_version=policy["schema_version"],
        governing_issue_field=field,
        allowed_issue_states=frozenset(item.casefold() for item in states),
        report_schema_version=policy["report_schema_version"],
    )


def parse_task(path: Path, policy: Policy) -> tuple[str, str]:
    text = path.read_text(encoding="utf-8")
    values = frontmatter_scalars(text)
    task_id = values.get("task_id", path.stem).strip()
    issue = values.get(policy.governing_issue_field, "").strip()
    return task_id, issue


def _issue_number(value: str) -> int | None:
    if not value:
        return None
    if not re.fullmatch(r"[1-9][0-9]*", value):
        raise IssueLivenessError(
            f"invalid governing Issue identity {value!r}; expected a positive integer"
        )
    return int(value)


def evaluate_task(
    path: Path,
    *,
    repository: str,
    client: GitHubState,
    policy: Policy,
) -> TaskResult:
    task_id, raw_issue = parse_task(path, policy)
    findings: list[Finding] = []
    issue_state = "UNKNOWN"
    try:
        number = _issue_number(raw_issue)
    except IssueLivenessError as exc:
        findings.append(Finding("error", "invalid_governing_issue", str(exc)))
        number = None

    if number is None:
        findings.append(
            Finding(
                "error",
                "missing_governing_issue",
                f"active task must declare top-level {policy.governing_issue_field}: <positive issue number>",
            )
        )
    else:
        try:
            payload = client.get_issue(number)
        except IssueLivenessError as exc:
            findings.append(Finding("error", "github_issue_state_unavailable", str(exc)))
        else:
            observed_number = payload.get("number")
            state = payload.get("state")
            if observed_number != number or state not in {"open", "closed"}:
                findings.append(
                    Finding(
                        "error",
                        "invalid_issue_payload",
                        f"governing Issue #{number} response is missing required identity/state",
                    )
                )
            elif "pull_request" in payload:
                findings.append(
                    Finding(
                        "error",
                        "governing_identity_is_pull_request",
                        f"#{number} is a pull request, not the governing GitHub Issue",
                    )
                )
            else:
                issue_state = state.upper()
                if state.casefold() not in policy.allowed_issue_states:
                    findings.append(
                        Finding(
                            "error",
                            "governing_issue_terminal",
                            f"governing Issue #{number} is {state}; active task must be reconciled/archived",
                        )
                    )

    live_valid = not any(item.severity == "error" for item in findings)
    return TaskResult(
        task_id=task_id,
        path=path.as_posix(),
        governing_issue=raw_issue or "none",
        live_valid=live_valid,
        issue_state=issue_state,
        findings=tuple(findings),
    )


def evaluate_tasks(
    active_root: Path,
    *,
    repository: str,
    client: GitHubState,
    policy: Policy,
) -> dict[str, object]:
    results: list[TaskResult] = []
    if active_root.exists():
        for path in sorted(active_root.glob("*.md")):
            if path.name.casefold() == "readme.md":
                continue
            results.append(
                evaluate_task(
                    path,
                    repository=repository,
                    client=client,
                    policy=policy,
                )
            )
    errors = sum(
        finding.severity == "error"
        for result in results
        for finding in result.findings
    )
    return {
        "schema_version": policy.report_schema_version,
        "repository": repository,
        "live_valid": errors == 0,
        "errors": errors,
        "tasks": [result.as_dict() for result in results],
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--active", type=Path, default=DEFAULT_ACTIVE)
    parser.add_argument("--contract", type=Path, default=DEFAULT_CONTRACT)
    parser.add_argument("--token-env", default="GITHUB_TOKEN")
    parser.add_argument(
        "--api-url", default=os.getenv("GITHUB_API_URL", "https://api.github.com")
    )
    parser.add_argument("--report-json", type=Path, default=None)
    args = parser.parse_args(argv)

    try:
        policy = load_policy(args.contract)
        token = os.getenv(args.token_env, "")
        client = GitHubClient(args.repository, token, args.api_url)
        report = evaluate_tasks(
            args.active,
            repository=args.repository,
            client=client,
            policy=policy,
        )
    except (OSError, IssueLivenessError, json.JSONDecodeError) as exc:
        print(f"task-issue-liveness error: {exc}", file=sys.stderr)
        return 1

    rendered = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    if args.report_json:
        args.report_json.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")

    if report["live_valid"]:
        print(
            f"Validated {len(report['tasks'])} active task(s) against live governing Issue state."
        )
        return 0

    for task in report["tasks"]:
        for finding in task["findings"]:
            if finding["severity"] == "error":
                print(
                    f"ERROR: {task['task_id']}: {finding['code']}: {finding['message']}",
                    file=sys.stderr,
                )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
