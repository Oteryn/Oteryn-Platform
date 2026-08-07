#!/usr/bin/env python3
"""Validate durable active agent tasks against live GitHub ownership state."""

from __future__ import annotations

import argparse
import dataclasses
import json
import os
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any, Protocol

DEFAULT_CONTRACT = Path("docs/agents/GOVERNANCE_CONTRACT.json")
DEFAULT_ACTIVE = Path("docs/agents/tasks/active")
DEFAULT_ARCHIVE = Path("docs/agents/tasks/archive")
NONE_VALUES = {"", "none", "null", "n/a", "na"}
TASK_ID_RE = re.compile(r"\b(?:FTAI|OTH|OTERYN|CAN|OTC2?|OTS)-[A-Z0-9][A-Z0-9-]*\b", re.I)
CHECKPOINT_RE = re.compile(r"(?m)^## Context checkpoint\s*$")


class LivenessError(RuntimeError):
    pass


class GitHubState(Protocol):
    def get_pull_request(self, number: int) -> dict[str, Any]: ...
    def get_branch(self, branch: str) -> dict[str, Any] | None: ...
    def get_pull_requests_for_branch(self, branch: str) -> list[dict[str, Any]]: ...


@dataclasses.dataclass(frozen=True)
class Policy:
    schema_version: int
    terminal_policy_field: str
    archive_pending_value: str
    terminal_allowed_statuses: frozenset[str]
    terminal_next_action_requires: tuple[str, ...]
    terminal_next_action_forbids: tuple[str, ...]
    no_pr_no_branch_allowed_statuses: frozenset[str]
    report_schema_version: int


@dataclasses.dataclass(frozen=True)
class TaskRecord:
    task_id: str
    path: str
    status: str
    branch: str
    pr: str
    next_action: str
    terminal_policy: str


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
    schema_valid: bool
    live_valid: bool
    live_state: str
    ownership_active: bool
    pr: str
    branch: str
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
            raise LivenessError("repository must use owner/name form")
        if not token:
            raise LivenessError("GitHub token is required for live task validation")
        self.repository = repository
        self.token = token
        self.api_url = api_url.rstrip("/")
        self.timeout = timeout

    def _request_json(self, path: str) -> Any:
        url = f"{self.api_url}{path}"
        request = urllib.request.Request(
            url,
            method="GET",
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {self.token}",
                "User-Agent": "oteryn-task-liveness/1",
                "X-GitHub-Api-Version": "2022-11-28",
            },
        )
        try:
            with urllib.request.urlopen(request, timeout=self.timeout) as response:
                raw = response.read().decode("utf-8")
        except urllib.error.HTTPError as exc:
            raise LivenessError(
                f"GitHub state unavailable: GET {path} returned HTTP {exc.code}"
            ) from exc
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            raise LivenessError(f"GitHub state unavailable: GET {path} failed") from exc
        try:
            return json.loads(raw)
        except json.JSONDecodeError as exc:
            raise LivenessError(
                f"GitHub state unavailable: GET {path} returned invalid JSON"
            ) from exc

    def _request(self, path: str) -> dict[str, Any]:
        payload = self._request_json(path)
        if not isinstance(payload, dict):
            raise LivenessError(f"GitHub state unavailable: GET {path} returned a non-object")
        return payload

    def _request_list(self, path: str) -> list[dict[str, Any]]:
        payload = self._request_json(path)
        if not isinstance(payload, list) or not all(isinstance(item, dict) for item in payload):
            raise LivenessError(
                f"GitHub state unavailable: GET {path} returned a non-object list"
            )
        return payload

    def get_pull_request(self, number: int) -> dict[str, Any]:
        return self._request(f"/repos/{self.repository}/pulls/{number}")

    def get_branch(self, branch: str) -> dict[str, Any] | None:
        encoded = "heads/" + urllib.parse.quote(branch, safe="/")
        path = f"/repos/{self.repository}/git/ref/{encoded}"
        try:
            return self._request(path)
        except LivenessError as exc:
            if "HTTP 404" in str(exc):
                return None
            raise

    def get_pull_requests_for_branch(self, branch: str) -> list[dict[str, Any]]:
        owner = self.repository.split("/", 1)[0]
        query = urllib.parse.urlencode(
            {
                "state": "all",
                "head": f"{owner}:{branch}",
                "per_page": "100",
                "sort": "updated",
                "direction": "desc",
            }
        )
        path = f"/repos/{self.repository}/pulls?{query}"
        pulls = self._request_list(path)
        if len(pulls) >= 100:
            raise LivenessError(
                f"GitHub state unavailable: branch {branch!r} has at least 100 PR records; "
                "history is ambiguous"
            )
        return pulls


def _scalar(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {'"', "'"}:
        return value[1:-1]
    return value


def _read_scalar_lines(lines: list[str], values: dict[str, str]) -> None:
    for raw in lines:
        if not raw or raw[0].isspace() or raw.lstrip().startswith("#") or ":" not in raw:
            continue
        key, value = raw.split(":", 1)
        key = key.strip()
        value = _scalar(value)
        if key and value and value not in {"[]", "{}"}:
            values[key] = value


def scalar_map(text: str) -> dict[str, str]:
    """Read inert top-level scalar data from task frontmatter and checkpoint."""
    values: dict[str, str] = {}
    if text.startswith("---"):
        end = text.find("\n---", 3)
        if end >= 0:
            _read_scalar_lines(text[3:end].splitlines(), values)

    match = CHECKPOINT_RE.search(text)
    if match:
        remainder = text[match.end():]
        fence = re.search(r"```(?:yaml|yml)\s*\n", remainder, re.I)
        if fence:
            block_end = remainder.find("```", fence.end())
            if block_end >= 0:
                _read_scalar_lines(remainder[fence.end():block_end].splitlines(), values)
    return values


def infer_task_id(path: Path, text: str, values: dict[str, str]) -> str:
    explicit = values.get("task_id", "").strip()
    if explicit:
        return explicit
    match = TASK_ID_RE.search(path.stem) or TASK_ID_RE.search(text)
    return match.group(0) if match else path.stem


def parse_task(path: Path, policy: Policy) -> TaskRecord:
    text = path.read_text(encoding="utf-8")
    values = scalar_map(text)
    return TaskRecord(
        task_id=infer_task_id(path, text, values),
        path=path.as_posix(),
        status=values.get("status", "").strip().casefold().replace(" ", "_"),
        branch=values.get("branch", "none").strip(),
        pr=(values.get("pr") or values.get("pull_request") or "none").strip(),
        next_action=values.get("next_action", "").strip(),
        terminal_policy=values.get(policy.terminal_policy_field, "").strip(),
    )


def load_policy(path: Path = DEFAULT_CONTRACT) -> Policy:
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
        policy = raw["live_task_liveness"]
    except (OSError, json.JSONDecodeError, KeyError, TypeError) as exc:
        raise LivenessError(f"{path}: invalid live_task_liveness policy") from exc

    required = {
        "schema_version",
        "terminal_policy_field",
        "archive_pending_value",
        "terminal_allowed_statuses",
        "terminal_next_action_requires",
        "terminal_next_action_forbids",
        "no_pr_no_branch_allowed_statuses",
        "report_schema_version",
    }
    if not isinstance(policy, dict) or set(policy) != required:
        raise LivenessError(f"{path}: live_task_liveness policy fields do not match contract")

    def string_list(key: str) -> list[str]:
        value = policy[key]
        if not isinstance(value, list) or not all(
            isinstance(item, str) and item for item in value
        ):
            raise LivenessError(
                f"{path}: live_task_liveness.{key} must be a non-empty string list"
            )
        return value

    for key in ("schema_version", "report_schema_version"):
        if not isinstance(policy[key], int) or policy[key] < 1:
            raise LivenessError(
                f"{path}: live_task_liveness.{key} must be a positive integer"
            )

    for key in ("terminal_policy_field", "archive_pending_value"):
        if not isinstance(policy[key], str) or not policy[key].strip():
            raise LivenessError(f"{path}: live_task_liveness.{key} must be non-empty")

    return Policy(
        schema_version=policy["schema_version"],
        terminal_policy_field=policy["terminal_policy_field"],
        archive_pending_value=policy["archive_pending_value"],
        terminal_allowed_statuses=frozenset(
            item.casefold() for item in string_list("terminal_allowed_statuses")
        ),
        terminal_next_action_requires=tuple(
            item.casefold() for item in string_list("terminal_next_action_requires")
        ),
        terminal_next_action_forbids=tuple(
            item.casefold() for item in string_list("terminal_next_action_forbids")
        ),
        no_pr_no_branch_allowed_statuses=frozenset(
            item.casefold() for item in string_list("no_pr_no_branch_allowed_statuses")
        ),
        report_schema_version=policy["report_schema_version"],
    )


def _none(value: str) -> bool:
    return value.strip().casefold() in NONE_VALUES


def _numeric_pr(value: str) -> int | None:
    if _none(value):
        return None
    if not re.fullmatch(r"[1-9][0-9]*", value.strip()):
        raise LivenessError(
            f"invalid PR identity {value!r}; expected a positive integer or none"
        )
    return int(value)


def archive_task_ids(root: Path, policy: Policy) -> set[str]:
    result: set[str] = set()
    if not root.exists():
        return result
    for path in sorted(root.glob("*.md")):
        if path.name.casefold() == "readme.md":
            continue
        result.add(parse_task(path, policy).task_id)
    return result


def _pull_state(payload: dict[str, Any]) -> tuple[str, bool, bool, str, str]:
    state = payload.get("state")
    merged = payload.get("merged")
    draft = payload.get("draft")
    head = payload.get("head")
    if (
        state not in {"open", "closed"}
        or not isinstance(merged, bool)
        or not isinstance(draft, bool)
        or not isinstance(head, dict)
    ):
        raise LivenessError("GitHub pull request response is missing required state fields")
    ref = head.get("ref")
    repo = head.get("repo")
    repo_name = repo.get("full_name") if isinstance(repo, dict) else None
    if (
        not isinstance(ref, str)
        or not ref
        or not isinstance(repo_name, str)
        or not repo_name
    ):
        raise LivenessError("GitHub pull request response is missing head identity")
    return state, merged, draft, ref, repo_name


def _branch_sha(payload: dict[str, Any]) -> str:
    obj = payload.get("object")
    sha = obj.get("sha") if isinstance(obj, dict) else None
    if not isinstance(sha, str) or not re.fullmatch(r"[0-9a-fA-F]{40}", sha):
        raise LivenessError("GitHub branch response is missing a valid head SHA")
    return sha.casefold()


def _branch_pull_identity(
    payload: dict[str, Any],
) -> tuple[int, str, bool, str, str, str]:
    number = payload.get("number")
    state = payload.get("state")
    draft = payload.get("draft")
    head = payload.get("head")
    if (
        not isinstance(number, int)
        or number < 1
        or state not in {"open", "closed"}
        or not isinstance(draft, bool)
        or not isinstance(head, dict)
    ):
        raise LivenessError(
            "GitHub branch pull-request response is missing required state fields"
        )
    ref = head.get("ref")
    sha = head.get("sha")
    repo = head.get("repo")
    repo_name = repo.get("full_name") if isinstance(repo, dict) else None
    if (
        not isinstance(ref, str)
        or not ref
        or not isinstance(sha, str)
        or not re.fullmatch(r"[0-9a-fA-F]{40}", sha)
        or not isinstance(repo_name, str)
        or not repo_name
    ):
        raise LivenessError("GitHub branch pull-request response is missing head identity")
    return number, state, draft, ref, repo_name, sha.casefold()


def _reconcile_branch_only(
    task: TaskRecord,
    *,
    repository: str,
    branch_payload: dict[str, Any],
    client: GitHubState,
    findings: list[Finding],
) -> tuple[str, bool]:
    try:
        branch_sha = _branch_sha(branch_payload)
        branch_pulls = client.get_pull_requests_for_branch(task.branch)
        current: list[tuple[int, str, bool]] = []
        for payload in branch_pulls:
            number, state, draft, ref, repo_name, head_sha = _branch_pull_identity(payload)
            if repo_name.casefold() != repository.casefold():
                findings.append(
                    Finding(
                        "error",
                        "foreign_branch_pr_head",
                        f"PR #{number} head repository {repo_name!r} does not match {repository!r}",
                    )
                )
                continue
            if ref == task.branch and head_sha == branch_sha:
                current.append((number, state, draft))
    except LivenessError as exc:
        findings.append(Finding("error", "github_state_unavailable", str(exc)))
        return "UNKNOWN", False

    if len(current) > 1:
        numbers = ", ".join(f"#{number}" for number, _, _ in current)
        findings.append(
            Finding(
                "error",
                "ambiguous_branch_pr_history",
                f"claimed branch {task.branch!r} current head matches multiple PRs ({numbers}); "
                "explicit task PR identity is required",
            )
        )
        return "AMBIGUOUS_BRANCH_PR", False

    if not current:
        return "BRANCH_ONLY", True

    number, state, draft = current[0]
    if state == "open":
        findings.append(
            Finding(
                "error",
                "branch_pr_identity_omitted",
                f"claimed branch {task.branch!r} current head already has "
                f"{'draft ' if draft else ''}open PR #{number}; task must record that PR identity",
            )
        )
        return (
            "DRAFT_PR_IDENTITY_OMITTED" if draft else "OPEN_PR_IDENTITY_OMITTED",
            False,
        )

    findings.append(
        Finding(
            "error",
            "terminal_pr_identity_omitted",
            f"claimed branch {task.branch!r} current head belongs to terminal PR #{number}; "
            "retained branch existence cannot preserve active ownership",
        )
    )
    return "TERMINAL_PR_IDENTITY_OMITTED", False


def evaluate_task(
    task: TaskRecord,
    *,
    repository: str,
    client: GitHubState,
    archived_ids: set[str],
    policy: Policy,
) -> TaskResult:
    findings: list[Finding] = []
    ownership_active = False
    live_state = "UNKNOWN"

    if task.task_id in archived_ids:
        findings.append(
            Finding(
                "error",
                "duplicate_active_archive",
                "task exists in both active and archive state",
            )
        )

    try:
        pr_number = _numeric_pr(task.pr)
    except LivenessError as exc:
        findings.append(Finding("error", "invalid_pr_identity", str(exc)))
        pr_number = None

    if pr_number is None:
        if _none(task.branch):
            live_state = "WAITING_EXTERNAL"
            if task.status not in policy.no_pr_no_branch_allowed_statuses:
                findings.append(
                    Finding(
                        "error",
                        "active_task_without_live_identity",
                        "task has neither PR nor branch outside an allowed waiting/blocked state",
                    )
                )
        else:
            try:
                branch = client.get_branch(task.branch)
            except LivenessError as exc:
                findings.append(Finding("error", "github_state_unavailable", str(exc)))
            else:
                if branch is None:
                    live_state = "BRANCH_ONLY"
                    findings.append(
                        Finding(
                            "error",
                            "missing_branch",
                            f"claimed branch {task.branch!r} does not exist",
                        )
                    )
                else:
                    live_state, ownership_active = _reconcile_branch_only(
                        task,
                        repository=repository,
                        branch_payload=branch,
                        client=client,
                        findings=findings,
                    )
    else:
        try:
            payload = client.get_pull_request(pr_number)
            state, merged, draft, head_ref, head_repo = _pull_state(payload)
        except LivenessError as exc:
            findings.append(Finding("error", "github_state_unavailable", str(exc)))
        else:
            terminal = merged or state == "closed"
            if head_repo.casefold() != repository.casefold():
                findings.append(
                    Finding(
                        "error",
                        "foreign_pr_head",
                        f"PR #{pr_number} head repository {head_repo!r} does not match {repository!r}",
                    )
                )

            if terminal:
                live_state = "TERMINAL_ARCHIVE_PENDING"
                ownership_active = False
                action = task.next_action.casefold()
                stale_action = next(
                    (
                        token
                        for token in policy.terminal_next_action_forbids
                        if (
                            re.search(rf"\b{re.escape(token)}\b", action)
                            if " " not in token
                            else token in action
                        )
                    ),
                    None,
                )
                required_action_missing = not all(
                    token in action for token in policy.terminal_next_action_requires
                )
                explicit_transition = (
                    task.terminal_policy == policy.archive_pending_value
                    and task.status in policy.terminal_allowed_statuses
                    and not required_action_missing
                    and stale_action is None
                )
                if stale_action is not None:
                    findings.append(
                        Finding(
                            "error",
                            "terminal_pr_stale_next_action",
                            f"terminal PR #{pr_number} retains stale next action token {stale_action!r}",
                        )
                    )
                if not explicit_transition:
                    findings.append(
                        Finding(
                            "error",
                            "terminal_pr_active_task",
                            f"terminal PR #{pr_number} is still represented as active without an explicit archive-pending transition",
                        )
                    )
                if not _none(task.branch):
                    try:
                        retained = client.get_branch(task.branch)
                    except LivenessError as exc:
                        findings.append(Finding("error", "github_state_unavailable", str(exc)))
                    else:
                        if retained is not None:
                            findings.append(
                                Finding(
                                    "advisory",
                                    "terminal_branch_retained",
                                    f"terminal source branch {task.branch!r} remains and requires lifecycle classification",
                                )
                            )
            else:
                live_state = "DRAFT_PR" if draft else "OPEN_PR"
                ownership_active = True
                if _none(task.branch):
                    findings.append(
                        Finding(
                            "error",
                            "missing_branch_identity",
                            f"open PR #{pr_number} task has no claimed branch",
                        )
                    )
                elif task.branch != head_ref:
                    findings.append(
                        Finding(
                            "error",
                            "branch_pr_mismatch",
                            f"task branch {task.branch!r} does not match PR #{pr_number} head {head_ref!r}",
                        )
                    )
                else:
                    try:
                        branch = client.get_branch(task.branch)
                    except LivenessError as exc:
                        findings.append(Finding("error", "github_state_unavailable", str(exc)))
                    else:
                        if branch is None:
                            findings.append(
                                Finding(
                                    "error",
                                    "missing_branch",
                                    f"claimed branch {task.branch!r} for PR #{pr_number} does not exist",
                                )
                            )

    live_valid = not any(item.severity == "error" for item in findings)
    return TaskResult(
        task_id=task.task_id,
        path=task.path,
        schema_valid=True,
        live_valid=live_valid,
        live_state=live_state,
        ownership_active=ownership_active if live_valid else False,
        pr=task.pr,
        branch=task.branch,
        findings=tuple(findings),
    )


def evaluate_tasks(
    active_root: Path,
    archive_root: Path,
    *,
    repository: str,
    client: GitHubState,
    policy: Policy,
) -> dict[str, object]:
    archived = archive_task_ids(archive_root, policy)
    results: list[TaskResult] = []
    if active_root.exists():
        for path in sorted(active_root.glob("*.md")):
            if path.name.casefold() == "readme.md":
                continue
            task = parse_task(path, policy)
            results.append(
                evaluate_task(
                    task,
                    repository=repository,
                    client=client,
                    archived_ids=archived,
                    policy=policy,
                )
            )

    errors = sum(
        finding.severity == "error"
        for result in results
        for finding in result.findings
    )
    advisories = sum(
        finding.severity == "advisory"
        for result in results
        for finding in result.findings
    )
    return {
        "schema_version": policy.report_schema_version,
        "repository": repository,
        "live_valid": errors == 0,
        "errors": errors,
        "advisories": advisories,
        "tasks": [result.as_dict() for result in results],
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--active", type=Path, default=DEFAULT_ACTIVE)
    parser.add_argument("--archive", type=Path, default=DEFAULT_ARCHIVE)
    parser.add_argument("--contract", type=Path, default=DEFAULT_CONTRACT)
    parser.add_argument("--token-env", default="GITHUB_TOKEN")
    parser.add_argument("--api-url", default=os.getenv("GITHUB_API_URL", "https://api.github.com"))
    parser.add_argument("--report-json", type=Path, default=None)
    args = parser.parse_args(argv)

    try:
        policy = load_policy(args.contract)
        token = os.getenv(args.token_env, "")
        client = GitHubClient(args.repository, token, args.api_url)
        report = evaluate_tasks(
            args.active,
            args.archive,
            repository=args.repository,
            client=client,
            policy=policy,
        )
    except (OSError, LivenessError, json.JSONDecodeError) as exc:
        print(f"task-liveness error: {exc}", file=sys.stderr)
        return 1

    rendered = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    if args.report_json:
        args.report_json.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")

    if report["live_valid"]:
        print(
            f"Validated {len(report['tasks'])} active task(s) against live GitHub state; "
            f"{report['advisories']} advisory finding(s)."
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
