#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import re
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request
from collections import Counter
from pathlib import Path
from typing import Any, Iterable

POLICY_PATH = Path("docs/agents/BRANCH_LIFECYCLE_POLICY.json")
MANIFEST_PATH = Path("docs/agents/BRANCH_DELETION_MANIFEST.json")
ACTIVE_TASKS_PATH = Path("docs/agents/tasks/active")
ISSUE_NUMBER = 658
SCHEMA_VERSION = 1
MANIFEST_SCHEMA_VERSION = 1
CLASSIFICATIONS = {
    "PROTECTED", "OPEN_PR", "ACTIVE_CLAIM", "RELEASE", "ROLLBACK",
    "RECOVERY", "TERMINAL_MERGED", "UNMERGED_ORPHAN", "UNKNOWN",
}
RETENTION_CLASSIFICATIONS = {"PROTECTED", "RELEASE", "ROLLBACK", "RECOVERY"}
RESERVED_NAME_PARTS = ("release", "rollback", "recovery", "backup")
CONFIRMATION = "DELETE_REVIEWED_TERMINAL_MERGED_BRANCHES_ISSUE_658"
TASK_BRANCH_RE = re.compile(r"^\s*(?:branch|lock_branch):\s*([^\s#]+)\s*$", re.M)
REPAIR_ISSUE_RE = re.compile(r"^repair/issue-(\d+)$")
FULL_SHA_RE = re.compile(r"^[0-9a-fA-F]{40}$")
SCP_GITHUB_REMOTE_RE = re.compile(r"^git@github\.com:([^/\s]+)/([^/\s]+)$", re.I)


class ValidationError(RuntimeError):
    pass


class ApiError(RuntimeError):
    def __init__(self, method: str, path: str, status: int, body: str) -> None:
        super().__init__(f"{method} {path}: GitHub API returned {status}: {body[:500]}")
        self.method = method
        self.path = path
        self.status = status
        self.body = body


class GitHubClient:
    def __init__(
        self,
        repo: str,
        token: str,
        api_url: str = "https://api.github.com",
        *,
        root: Path | None = None,
        git_remote: str = "origin",
    ) -> None:
        if "/" not in repo:
            raise ValidationError("repository must use owner/name form")
        if (
            not git_remote
            or git_remote.startswith("-")
            or any(character.isspace() for character in git_remote)
        ):
            raise ValidationError("git remote name must be a non-option token without whitespace")
        self.repo = repo
        self.token = token
        self.api_url = api_url.rstrip("/")
        self.git_root = (root or Path(".")).resolve()
        self.git_remote = git_remote

    def request(
        self,
        method: str,
        path: str,
        *,
        data: dict[str, Any] | None = None,
        expected: Iterable[int] = (200,),
    ) -> tuple[Any, dict[str, str]]:
        url = path if path.startswith("http") else f"{self.api_url}{path}"
        body = None if data is None else json.dumps(data).encode("utf-8")
        request = urllib.request.Request(
            url,
            data=body,
            method=method,
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {self.token}",
                "User-Agent": "oteryn-branch-lifecycle/1",
                "X-GitHub-Api-Version": "2022-11-28",
                "Content-Type": "application/json",
            },
        )
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                raw = response.read().decode("utf-8")
                payload = json.loads(raw) if raw else None
                return payload, {key.lower(): value for key, value in response.headers.items()}
        except urllib.error.HTTPError as exc:
            raw = exc.read().decode("utf-8", errors="replace")
            if exc.code in set(expected):
                payload = json.loads(raw) if raw else None
                return payload, {key.lower(): value for key, value in exc.headers.items()}
            raise ApiError(method, path, exc.code, raw) from exc

    def paginate(self, path: str) -> list[Any]:
        items: list[Any] = []
        next_url: str | None = path
        while next_url:
            payload, headers = self.request("GET", next_url)
            if not isinstance(payload, list):
                raise ValidationError(f"paginated endpoint did not return an array: {next_url}")
            items.extend(payload)
            next_url = _next_link(headers.get("link", ""))
        return items

    def _run_git(
        self, command: list[str], *, timeout: int, purpose: str
    ) -> subprocess.CompletedProcess[str]:
        try:
            return subprocess.run(
                command,
                check=False,
                capture_output=True,
                text=True,
                timeout=timeout,
                cwd=self.git_root,
            )
        except FileNotFoundError as exc:
            raise ValidationError(
                f"{purpose} requires the git executable in the configured repository environment"
            ) from exc
        except subprocess.TimeoutExpired as exc:
            raise ValidationError(f"{purpose} timed out") from exc

    def _validated_git_remote(self) -> str:
        root_probe = self._run_git(
            ["git", "rev-parse", "--show-toplevel"],
            timeout=15,
            purpose="git repository root validation",
        )
        if root_probe.returncode != 0:
            raise ValidationError("configured --root is not a Git working tree")
        reported_root = root_probe.stdout.strip()
        if not reported_root:
            raise ValidationError("git repository root validation returned no path")
        if Path(reported_root).resolve() != self.git_root:
            raise ValidationError(
                "configured --root does not match the Git working tree used for destructive operations"
            )

        remote_probe = self._run_git(
            ["git", "remote", "get-url", "--push", "--all", self.git_remote],
            timeout=15,
            purpose="git remote identity validation",
        )
        if remote_probe.returncode != 0:
            raise ValidationError(
                f"configured git remote {self.git_remote!r} is missing or has no push URL"
            )
        push_urls = [line.strip() for line in remote_probe.stdout.splitlines() if line.strip()]
        if len(push_urls) != 1:
            raise ValidationError(
                f"configured git remote {self.git_remote!r} must resolve to exactly one push URL"
            )
        remote_repo = _github_repository_from_remote(push_urls[0])
        if remote_repo is None:
            raise ValidationError(
                f"configured git remote {self.git_remote!r} does not use a supported GitHub SSH or HTTPS repository URL"
            )
        if remote_repo.casefold() != self.repo.casefold():
            raise ValidationError(
                f"configured git remote repository mismatch: expected {self.repo}, got {remote_repo}"
            )
        return self.git_remote

    def _delete_ref_with_lease(self, branch: str, expected_sha: str) -> None:
        if not FULL_SHA_RE.fullmatch(expected_sha):
            raise ValidationError(
                f"atomic delete expected SHA for {branch} must be a full 40-character hexadecimal object ID"
            )
        ref = f"refs/heads/{branch}"
        remote = self._validated_git_remote()
        command = [
            "git",
            "push",
            "--porcelain",
            f"--force-with-lease={ref}:{expected_sha}",
            remote,
            f":{ref}",
        ]
        result = self._run_git(
            command,
            timeout=60,
            purpose=f"atomic branch deletion for {branch}",
        )
        if result.returncode == 0:
            return

        current_sha = _ref_sha(self.get_ref(branch))
        if current_sha is None:
            raise ValidationError(
                f"atomic branch deletion returned failure for {branch} but the remote ref is now missing; "
                "result is ambiguous and requires manual evidence review"
            )
        if current_sha != expected_sha:
            raise ValidationError(
                f"atomic delete lease rejected for {branch}: expected {expected_sha}, got {current_sha}"
            )
        raise ValidationError(
            f"atomic branch deletion push was rejected for {branch} while the reviewed SHA remained current"
        )

    def delete_branch(self, branch: str, *, expected_sha: str | None = None) -> None:
        current_sha = _ref_sha(self.get_ref(branch))
        if current_sha is None:
            raise ValidationError(f"pre-delete branch is missing: {branch}")
        if expected_sha is None:
            expected_sha = current_sha
        elif current_sha != expected_sha:
            raise ValidationError(
                f"pre-delete SHA drift for {branch}: expected {expected_sha}, got {current_sha}"
            )
        self._delete_ref_with_lease(branch, expected_sha)

    def create_branch(self, branch: str, sha: str) -> None:
        self.request(
            "POST",
            f"/repos/{self.repo}/git/refs",
            data={"ref": f"refs/heads/{branch}", "sha": sha},
            expected=(201,),
        )

    def get_ref(self, branch: str) -> dict[str, Any] | None:
        encoded = "heads/" + urllib.parse.quote(branch, safe="/")
        try:
            payload, _ = self.request("GET", f"/repos/{self.repo}/git/ref/{encoded}")
        except ApiError as exc:
            if exc.status == 404:
                return None
            raise
        if not isinstance(payload, dict):
            raise ValidationError("GitHub ref response must be an object")
        return payload

    def get_branch(self, branch: str) -> dict[str, Any] | None:
        encoded = urllib.parse.quote(branch, safe="")
        try:
            payload, _ = self.request("GET", f"/repos/{self.repo}/branches/{encoded}")
        except ApiError as exc:
            if exc.status == 404:
                return None
            raise
        if not isinstance(payload, dict):
            raise ValidationError("GitHub branch response must be an object")
        return payload

    def open_pulls_for_branch(self, branch: str) -> list[dict[str, Any]]:
        owner = self.repo.split("/", 1)[0]
        query = urllib.parse.urlencode({
            "state": "open",
            "head": f"{owner}:{branch}",
            "per_page": 100,
        })
        raw_pulls = self.paginate(f"/repos/{self.repo}/pulls?{query}")
        pulls: list[dict[str, Any]] = []
        for index, pull in enumerate(raw_pulls):
            if not isinstance(pull, dict):
                raise ValidationError(f"open pulls[{index}]: expected object")
            if (
                pull.get("state") == "open"
                and _same_repo_pull(pull, self.repo)
                and _pull_branch(pull) == branch
            ):
                pulls.append(pull)
        return pulls

    def get_issue_state(self, issue_number: int) -> str:
        payload, _ = self.request(
            "GET", f"/repos/{self.repo}/issues/{issue_number}", expected=(200, 404)
        )
        if isinstance(payload, dict) and payload.get("state") in {"open", "closed"}:
            return payload["state"]
        return "unknown"


def _next_link(header: str) -> str | None:
    for part in header.split(","):
        section = part.strip()
        if 'rel="next"' not in section:
            continue
        match = re.match(r"<([^>]+)>", section)
        return match.group(1) if match else None
    return None


def _github_repository_from_remote(value: str) -> str | None:
    remote = value.strip()
    if not remote:
        return None

    scp_match = SCP_GITHUB_REMOTE_RE.fullmatch(remote)
    if scp_match:
        owner, repo = scp_match.groups()
    else:
        parsed = urllib.parse.urlsplit(remote)
        scheme = parsed.scheme.casefold()
        if scheme not in {"https", "ssh"}:
            return None
        if (parsed.hostname or "").casefold() != "github.com":
            return None
        if parsed.query or parsed.fragment:
            return None
        if scheme == "https":
            if parsed.username is not None or parsed.password is not None:
                return None
            if parsed.port not in {None, 443}:
                return None
        else:
            if parsed.username != "git" or parsed.password is not None:
                return None
            if parsed.port not in {None, 22}:
                return None
        parts = [part for part in parsed.path.split("/") if part]
        if len(parts) != 2:
            return None
        owner, repo = parts

    if repo.casefold().endswith(".git"):
        repo = repo[:-4]
    if (
        not owner
        or not repo
        or owner in {".", ".."}
        or repo in {".", ".."}
        or any(character.isspace() for character in owner + repo)
    ):
        return None
    return f"{owner}/{repo}"


def canonical_json(value: object) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def load_json(path: Path) -> dict[str, Any]:
    try:
        raw = path.read_text(encoding="utf-8")
    except FileNotFoundError as exc:
        raise ValidationError(f"missing JSON file: {path.as_posix()}") from exc
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise ValidationError(
            f"{path.as_posix()}: invalid JSON at {exc.lineno}:{exc.colno}: {exc.msg}"
        ) from exc
    if not isinstance(value, dict):
        raise ValidationError(f"{path.as_posix()}: root must be an object")
    if raw != canonical_json(value):
        raise ValidationError(
            f"{path.as_posix()}: JSON must use sorted keys, two-space indentation and one trailing newline"
        )
    return value


def _exact_fields(value: object, expected: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValidationError(f"{context}: expected object")
    missing = sorted(expected - set(value))
    unknown = sorted(set(value) - expected)
    if missing:
        raise ValidationError(f"{context}: missing fields: {', '.join(missing)}")
    if unknown:
        raise ValidationError(f"{context}: unknown fields: {', '.join(unknown)}")
    return value


def _text(value: object, context: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValidationError(f"{context}: expected non-empty string")
    return value


def _ref_sha(ref: object) -> str | None:
    if not isinstance(ref, dict):
        return None
    value = ref.get("object")
    if not isinstance(value, dict):
        return None
    sha = value.get("sha")
    return sha if isinstance(sha, str) and sha else None


def _branch_sha(branch: object) -> str | None:
    if not isinstance(branch, dict):
        return None
    value = branch.get("commit")
    if not isinstance(value, dict):
        return None
    sha = value.get("sha")
    return sha if isinstance(sha, str) and sha else None


def validate_policy(policy: dict[str, Any], root: Path) -> dict[str, dict[str, Any]]:
    _exact_fields(
        policy,
        {
            "schema_version", "issue", "accepted_adr", "default_branch",
            "retention_exceptions", "deletion_requirements",
        },
        "policy",
    )
    if policy["schema_version"] != SCHEMA_VERSION:
        raise ValidationError(f"policy.schema_version: expected {SCHEMA_VERSION}")
    if policy["issue"] != ISSUE_NUMBER:
        raise ValidationError(f"policy.issue: expected {ISSUE_NUMBER}")
    accepted_adr = _text(policy["accepted_adr"], "policy.accepted_adr")
    if not (root / accepted_adr).is_file():
        raise ValidationError(f"policy.accepted_adr: missing local file {accepted_adr}")
    default_branch = _text(policy["default_branch"], "policy.default_branch")
    exceptions = policy["retention_exceptions"]
    if not isinstance(exceptions, list) or not exceptions:
        raise ValidationError("policy.retention_exceptions: expected non-empty array")

    result: dict[str, dict[str, Any]] = {}
    for index, raw in enumerate(exceptions):
        item = _exact_fields(
            raw,
            {"branch", "classification", "owner", "purpose", "review_trigger", "protected_required"},
            f"policy.retention_exceptions[{index}]",
        )
        branch = _text(item["branch"], f"policy.retention_exceptions[{index}].branch")
        if any(char in branch for char in "*?[]"):
            raise ValidationError(f"policy.retention_exceptions[{index}].branch: globs are forbidden")
        if branch.startswith("/") or branch.endswith("/") or ".." in branch.split("/"):
            raise ValidationError(f"policy.retention_exceptions[{index}].branch: malformed branch")
        classification = item["classification"]
        if classification not in RETENTION_CLASSIFICATIONS:
            raise ValidationError(
                f"policy.retention_exceptions[{index}].classification: unsupported {classification!r}"
            )
        _text(item["owner"], f"policy.retention_exceptions[{index}].owner")
        _text(item["purpose"], f"policy.retention_exceptions[{index}].purpose")
        _text(item["review_trigger"], f"policy.retention_exceptions[{index}].review_trigger")
        if item["protected_required"] is not True:
            raise ValidationError(
                f"policy.retention_exceptions[{index}].protected_required: must be true"
            )
        if branch in result:
            raise ValidationError(f"policy.retention_exceptions: duplicate branch {branch}")
        result[branch] = item

    default = result.get(default_branch)
    if default is None or default["classification"] != "PROTECTED":
        raise ValidationError(
            "policy.retention_exceptions: default branch must be an exact PROTECTED exception"
        )

    requirements = _exact_fields(
        policy["deletion_requirements"],
        {
            "classification", "exact_head_sha_required", "merged_pr_required",
            "open_pr_forbidden", "active_claim_forbidden", "protected_forbidden",
            "reserved_name_without_exception_forbidden",
        },
        "policy.deletion_requirements",
    )
    if requirements["classification"] != "TERMINAL_MERGED":
        raise ValidationError(
            "policy.deletion_requirements.classification must be TERMINAL_MERGED"
        )
    for field in (
        "exact_head_sha_required", "merged_pr_required", "open_pr_forbidden",
        "active_claim_forbidden", "protected_forbidden",
        "reserved_name_without_exception_forbidden",
    ):
        if requirements[field] is not True:
            raise ValidationError(f"policy.deletion_requirements.{field}: must be true")
    return result


def active_task_branches(root: Path) -> set[str]:
    result: set[str] = set()
    directory = root / ACTIVE_TASKS_PATH
    if not directory.is_dir():
        return result
    for path in sorted(directory.glob("*.md")):
        content = path.read_text(encoding="utf-8")
        for match in TASK_BRANCH_RE.finditer(content):
            branch = match.group(1).strip("\"'")
            if branch.casefold() not in {"null", "none", "unknown"}:
                result.add(branch)
    return result


def _same_repo_pull(pull: dict[str, Any], repo: str) -> bool:
    head = pull.get("head")
    if not isinstance(head, dict):
        return False
    head_repo = head.get("repo")
    return isinstance(head_repo, dict) and head_repo.get("full_name") == repo


def _pull_branch(pull: dict[str, Any]) -> str | None:
    head = pull.get("head")
    return head.get("ref") if isinstance(head, dict) and isinstance(head.get("ref"), str) else None


def _pull_sha(pull: dict[str, Any]) -> str | None:
    head = pull.get("head")
    return head.get("sha") if isinstance(head, dict) and isinstance(head.get("sha"), str) else None


def _pull_number(pull: dict[str, Any]) -> int | None:
    value = pull.get("number")
    return value if isinstance(value, int) and not isinstance(value, bool) else None


def _reserved_name(branch: str) -> str | None:
    parts = {part.casefold() for part in re.split(r"[/_.-]+", branch) if part}
    for marker in RESERVED_NAME_PARTS:
        if marker in parts:
            return marker
    return None


def classify_snapshot(
    policy: dict[str, Any], snapshot: dict[str, Any], *, root: Path
) -> dict[str, Any]:
    exceptions = validate_policy(policy, root)
    repo = _text(snapshot.get("repository"), "snapshot.repository")
    default_branch = _text(snapshot.get("default_branch"), "snapshot.default_branch")
    if default_branch != policy["default_branch"]:
        raise ValidationError(
            f"snapshot.default_branch {default_branch!r} does not match policy"
        )
    branches = snapshot.get("branches")
    pulls = snapshot.get("pulls")
    if not isinstance(branches, list) or not isinstance(pulls, list):
        raise ValidationError("snapshot branches and pulls must be arrays")
    task_branches_raw = snapshot.get("active_task_branches", [])
    if not isinstance(task_branches_raw, list) or not all(
        isinstance(item, str) for item in task_branches_raw
    ):
        raise ValidationError("snapshot.active_task_branches must be a string array")
    task_branches = set(task_branches_raw)
    issue_states = snapshot.get("issue_states", {})
    if not isinstance(issue_states, dict):
        raise ValidationError("snapshot.issue_states must be an object")

    prs_by_branch: dict[str, list[dict[str, Any]]] = {}
    for pull in pulls:
        if not isinstance(pull, dict) or not _same_repo_pull(pull, repo):
            continue
        branch = _pull_branch(pull)
        if branch:
            prs_by_branch.setdefault(branch, []).append(pull)

    output: list[dict[str, Any]] = []
    seen: set[str] = set()
    for index, raw in enumerate(branches):
        if not isinstance(raw, dict):
            raise ValidationError(f"snapshot.branches[{index}]: expected object")
        name = _text(raw.get("name"), f"snapshot.branches[{index}].name")
        sha = _text(raw.get("sha"), f"snapshot.branches[{index}].sha")
        protected = raw.get("protected")
        if not isinstance(protected, bool):
            raise ValidationError(f"snapshot.branches[{index}].protected: expected boolean")
        if name in seen:
            raise ValidationError(f"snapshot.branches: duplicate branch {name}")
        seen.add(name)

        evidence: list[str] = []
        classification: str
        matched_pr: dict[str, Any] | None = None
        exception = exceptions.get(name)
        branch_prs = prs_by_branch.get(name, [])
        open_prs = [pr for pr in branch_prs if pr.get("state") == "open"]
        exact_merged = [
            pr for pr in branch_prs
            if pr.get("merged_at") and _pull_sha(pr) == sha and isinstance(_pull_number(pr), int)
        ]

        if exception is not None:
            if exception["protected_required"] and not protected:
                classification = "UNKNOWN"
                evidence.append(
                    "retention exception requires protection but live branch is not protected"
                )
            else:
                classification = exception["classification"]
                evidence.extend([
                    "exact retention exception exists",
                    f"retention owner: {exception['owner']}",
                    f"review trigger: {exception['review_trigger']}",
                ])
        elif protected:
            classification = "PROTECTED"
            evidence.append("GitHub reports branch protected")
        elif open_prs:
            classification = "OPEN_PR"
            evidence.append(
                "open pull requests: " + ", ".join(f"#{_pull_number(pr)}" for pr in open_prs)
            )
        elif name in task_branches:
            classification = "ACTIVE_CLAIM"
            evidence.append("branch is referenced by an active task checkpoint")
        else:
            issue_match = REPAIR_ISSUE_RE.fullmatch(name)
            issue_state = issue_states.get(issue_match.group(1)) if issue_match else None
            if issue_state == "open":
                classification = "ACTIVE_CLAIM"
                evidence.append(f"linked remediation Issue #{issue_match.group(1)} is open")
            else:
                marker = _reserved_name(name)
                if marker:
                    classification = "UNKNOWN"
                    evidence.append(
                        f"branch name suggests {marker} purpose but no exact retention exception exists"
                    )
                elif exact_merged:
                    exact_merged.sort(
                        key=lambda pr: str(pr.get("merged_at") or ""), reverse=True
                    )
                    matched_pr = exact_merged[0]
                    classification = "TERMINAL_MERGED"
                    evidence.extend([
                        f"merged pull request #{_pull_number(matched_pr)} has exact head SHA",
                        "no open pull request, active task, open deterministic Issue, protection or retention exception found",
                    ])
                elif branch_prs:
                    classification = "UNMERGED_ORPHAN"
                    evidence.append(
                        "pull-request history exists but no merged pull request matches the current head SHA"
                    )
                else:
                    classification = "UNKNOWN"
                    evidence.append("no authoritative pull-request or ownership evidence found")

        entry: dict[str, Any] = {
            "branch": name,
            "classification": classification,
            "deletion_candidate": classification == "TERMINAL_MERGED",
            "evidence": evidence,
            "head_sha": sha,
            "protected": protected,
        }
        if matched_pr is not None:
            entry["merged_pr"] = {
                "merged_at": matched_pr.get("merged_at"),
                "number": _pull_number(matched_pr),
                "url": matched_pr.get("html_url"),
            }
        else:
            entry["merged_pr"] = None
        output.append(entry)

    output.sort(key=lambda item: item["branch"])
    counts = dict(sorted(Counter(item["classification"] for item in output).items()))
    default_entry = next(
        (item for item in output if item["branch"] == default_branch), None
    )
    if default_entry is None:
        raise ValidationError("snapshot does not contain the default branch")
    return {
        "schema_version": SCHEMA_VERSION,
        "repository": repo,
        "default_branch": default_branch,
        "default_branch_sha": default_entry["head_sha"],
        "generated_at": _text(snapshot.get("generated_at"), "snapshot.generated_at"),
        "policy_sha256": sha256_bytes(canonical_json(policy).encode("utf-8")),
        "counts": counts,
        "deletion_candidate_count": sum(
            1 for item in output if item["deletion_candidate"]
        ),
        "branches": output,
    }


def make_manifest(report: dict[str, Any]) -> dict[str, Any]:
    entries = []
    for item in report["branches"]:
        if not item["deletion_candidate"]:
            continue
        merged = item["merged_pr"]
        entries.append({
            "branch": item["branch"],
            "head_sha": item["head_sha"],
            "merged_at": merged["merged_at"],
            "merged_pr_number": merged["number"],
        })
    entries.sort(key=lambda item: item["branch"])
    return {
        "schema_version": MANIFEST_SCHEMA_VERSION,
        "issue": ISSUE_NUMBER,
        "apply_on_main": False,
        "confirmation": CONFIRMATION,
        "policy_sha256": report["policy_sha256"],
        "inventory_default_branch_sha": report["default_branch_sha"],
        "entries": entries,
    }


def validate_manifest(
    manifest: dict[str, Any], report: dict[str, Any], *, require_apply: bool
) -> list[dict[str, Any]]:
    _exact_fields(
        manifest,
        {
            "schema_version", "issue", "apply_on_main", "confirmation",
            "policy_sha256", "inventory_default_branch_sha", "entries",
        },
        "manifest",
    )
    if manifest["schema_version"] != MANIFEST_SCHEMA_VERSION:
        raise ValidationError("manifest.schema_version: unsupported")
    if manifest["issue"] != ISSUE_NUMBER:
        raise ValidationError(f"manifest.issue: expected {ISSUE_NUMBER}")
    if manifest["confirmation"] != CONFIRMATION:
        raise ValidationError("manifest.confirmation: exact phrase required")
    if manifest["policy_sha256"] != report["policy_sha256"]:
        raise ValidationError("manifest.policy_sha256: policy drift detected")
    if require_apply and manifest["apply_on_main"] is not True:
        raise ValidationError("manifest.apply_on_main must be true for apply mode")
    if not isinstance(manifest["entries"], list):
        raise ValidationError("manifest.entries: expected array")

    live = {
        item["branch"]: item for item in report["branches"]
        if item["classification"] == "TERMINAL_MERGED"
    }
    validated: list[dict[str, Any]] = []
    seen: set[str] = set()
    for index, raw in enumerate(manifest["entries"]):
        entry = _exact_fields(
            raw,
            {"branch", "head_sha", "merged_at", "merged_pr_number"},
            f"manifest.entries[{index}]",
        )
        branch = _text(entry["branch"], f"manifest.entries[{index}].branch")
        if branch in seen:
            raise ValidationError(f"manifest.entries: duplicate branch {branch}")
        seen.add(branch)
        current = live.get(branch)
        if current is None:
            raise ValidationError(
                f"manifest.entries[{index}]: {branch} is no longer TERMINAL_MERGED"
            )
        merged = current["merged_pr"]
        expected = {
            "branch": branch,
            "head_sha": current["head_sha"],
            "merged_at": merged["merged_at"],
            "merged_pr_number": merged["number"],
        }
        if entry != expected:
            raise ValidationError(
                f"manifest.entries[{index}]: live branch/PR evidence drift for {branch}"
            )
        validated.append(expected)
    if [item["branch"] for item in validated] != sorted(item["branch"] for item in validated):
        raise ValidationError("manifest.entries must be ordered by branch")
    return validated


def fetch_live_snapshot(client: GitHubClient, root: Path) -> dict[str, Any]:
    repository, _ = client.request("GET", f"/repos/{client.repo}")
    if not isinstance(repository, dict):
        raise ValidationError("repository metadata must be an object")
    default_branch = _text(repository.get("default_branch"), "repository.default_branch")
    raw_branches = client.paginate(f"/repos/{client.repo}/branches?per_page=100")
    raw_pulls = client.paginate(
        f"/repos/{client.repo}/pulls?state=all&per_page=100&sort=updated&direction=desc"
    )

    branches: list[dict[str, Any]] = []
    issue_numbers: set[str] = set()
    for index, raw in enumerate(raw_branches):
        if not isinstance(raw, dict):
            raise ValidationError(f"branches[{index}]: expected object")
        commit = raw.get("commit")
        if not isinstance(commit, dict):
            raise ValidationError(f"branches[{index}].commit: expected object")
        name = _text(raw.get("name"), f"branches[{index}].name")
        branches.append({
            "name": name,
            "sha": _text(commit.get("sha"), f"branches[{index}].commit.sha"),
            "protected": bool(raw.get("protected")),
        })
        issue_match = REPAIR_ISSUE_RE.fullmatch(name)
        if issue_match:
            issue_numbers.add(issue_match.group(1))

    issue_states: dict[str, str] = {}
    for number in sorted(issue_numbers, key=int):
        issue, _ = client.request(
            "GET", f"/repos/{client.repo}/issues/{number}", expected=(200, 404)
        )
        if isinstance(issue, dict) and issue.get("state") in {"open", "closed"}:
            issue_states[number] = issue["state"]
        else:
            issue_states[number] = "unknown"

    generated = dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()
    return {
        "repository": client.repo,
        "default_branch": default_branch,
        "generated_at": generated,
        "branches": branches,
        "pulls": raw_pulls,
        "active_task_branches": sorted(active_task_branches(root)),
        "issue_states": issue_states,
    }


def write_report_files(
    report: dict[str, Any], output: Path, generated_manifest: Path | None
) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(canonical_json(report), encoding="utf-8")
    if generated_manifest is not None:
        generated_manifest.parent.mkdir(parents=True, exist_ok=True)
        generated_manifest.write_text(
            canonical_json(make_manifest(report)), encoding="utf-8"
        )


def revalidate_delete_entry(
    client: GitHubClient,
    policy: dict[str, Any],
    report: dict[str, Any],
    entry: dict[str, Any],
    *,
    root: Path,
) -> None:
    exceptions = validate_policy(policy, root)
    current_policy_hash = sha256_bytes(canonical_json(policy).encode("utf-8"))
    if current_policy_hash != report["policy_sha256"]:
        raise ValidationError("pre-delete policy drift detected")

    branch = _text(entry.get("branch"), "pre-delete entry.branch")
    expected_sha = _text(entry.get("head_sha"), f"pre-delete {branch}.head_sha")
    default_branch = _text(report.get("default_branch"), "report.default_branch")
    default_branch_sha = _text(
        report.get("default_branch_sha"), "report.default_branch_sha"
    )

    if branch == default_branch:
        raise ValidationError("pre-delete default branch refusal")
    if branch in exceptions:
        raise ValidationError(f"pre-delete retention exception appeared for {branch}")
    marker = _reserved_name(branch)
    if marker:
        raise ValidationError(
            f"pre-delete reserved retention-sensitive branch refused: {branch} ({marker})"
        )

    live_branch = client.get_branch(branch)
    if live_branch is None:
        raise ValidationError(f"pre-delete branch disappeared: {branch}")
    live_branch_sha = _branch_sha(live_branch)
    if live_branch_sha != expected_sha:
        raise ValidationError(
            f"pre-delete SHA drift for {branch}: expected {expected_sha}, "
            f"got {live_branch_sha or 'missing'}"
        )
    protected = live_branch.get("protected")
    if protected is not False:
        if protected is True:
            raise ValidationError(f"pre-delete branch became protected: {branch}")
        raise ValidationError(f"pre-delete protection state is ambiguous: {branch}")

    open_pulls = client.open_pulls_for_branch(branch)
    if open_pulls:
        numbers = sorted(
            number for number in (_pull_number(pull) for pull in open_pulls)
            if number is not None
        )
        suffix = ", ".join(f"#{number}" for number in numbers) or "unknown"
        raise ValidationError(f"pre-delete open pull request appeared for {branch}: {suffix}")

    if branch in active_task_branches(root):
        raise ValidationError(f"pre-delete active task claim appeared for {branch}")

    issue_match = REPAIR_ISSUE_RE.fullmatch(branch)
    if issue_match:
        issue_number = int(issue_match.group(1))
        issue_state = client.get_issue_state(issue_number)
        if issue_state != "closed":
            raise ValidationError(
                f"pre-delete remediation Issue #{issue_number} is {issue_state} for {branch}"
            )

    live_default_sha = _ref_sha(client.get_ref(default_branch))
    if live_default_sha != default_branch_sha:
        raise ValidationError(
            f"pre-delete default branch drift: expected {default_branch_sha}, "
            f"got {live_default_sha or 'missing'}"
        )

    final_sha = _ref_sha(client.get_ref(branch))
    if final_sha != expected_sha:
        raise ValidationError(
            f"pre-delete final SHA drift for {branch}: expected {expected_sha}, "
            f"got {final_sha or 'missing'}"
        )


def apply_manifest(
    client: GitHubClient,
    report: dict[str, Any],
    manifest: dict[str, Any],
    *,
    policy: dict[str, Any],
    root: Path,
    evidence_path: Path,
    event_name: str,
    ref_name: str,
) -> None:
    if event_name != "push" or ref_name != "main":
        raise ValidationError("apply mode is allowed only on a push to main")
    entries = validate_manifest(manifest, report, require_apply=True)
    if not entries:
        raise ValidationError("apply manifest contains no reviewed entries")
    deleted: list[dict[str, Any]] = []
    for entry in entries:
        revalidate_delete_entry(client, policy, report, entry, root=root)
        client.delete_branch(entry["branch"], expected_sha=entry["head_sha"])
        if client.get_ref(entry["branch"]) is not None:
            raise ValidationError(f"branch deletion could not be verified: {entry['branch']}")
        deleted.append(entry)
    evidence = {
        "schema_version": 1,
        "issue": ISSUE_NUMBER,
        "applied_at": dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat(),
        "repository": report["repository"],
        "default_branch_sha": report["default_branch_sha"],
        "policy_sha256": report["policy_sha256"],
        "deleted": deleted,
    }
    evidence_path.parent.mkdir(parents=True, exist_ok=True)
    evidence_path.write_text(canonical_json(evidence), encoding="utf-8")


def recovery_test(
    client: GitHubClient, default_branch_sha: str, run_id: str, evidence_path: Path
) -> None:
    safe_run_id = re.sub(r"[^A-Za-z0-9_.-]+", "-", run_id).strip("-") or "manual"
    branch = f"recovery-test/issue-{ISSUE_NUMBER}-{safe_run_id}"
    if client.get_ref(branch) is not None:
        raise ValidationError(f"recovery test branch already exists: {branch}")
    client.create_branch(branch, default_branch_sha)
    client.delete_branch(branch)
    if client.get_ref(branch) is not None:
        raise ValidationError("recovery test initial deletion failed")
    client.create_branch(branch, default_branch_sha)
    restored = client.get_ref(branch)
    restored_sha = restored.get("object", {}).get("sha") if isinstance(restored, dict) else None
    if restored_sha != default_branch_sha:
        raise ValidationError("recovery test restored SHA mismatch")
    client.delete_branch(branch)
    if client.get_ref(branch) is not None:
        raise ValidationError("recovery test cleanup failed")
    evidence = {
        "schema_version": 1,
        "issue": ISSUE_NUMBER,
        "branch": branch,
        "sha": default_branch_sha,
        "result": "PASS",
        "tested_at": dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat(),
    }
    evidence_path.parent.mkdir(parents=True, exist_ok=True)
    evidence_path.write_text(canonical_json(evidence), encoding="utf-8")


def parser() -> argparse.ArgumentParser:
    value = argparse.ArgumentParser(description="Oteryn merged branch lifecycle control")
    value.add_argument(
        "--mode",
        choices=("validate-policy", "inventory", "validate-manifest", "apply"),
        required=True,
    )
    value.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", ""))
    value.add_argument("--token", default=os.environ.get("GITHUB_TOKEN", ""))
    value.add_argument("--root", type=Path, default=Path("."))
    value.add_argument("--policy", type=Path, default=POLICY_PATH)
    value.add_argument("--snapshot", type=Path)
    value.add_argument("--output", type=Path, default=Path("branch-lifecycle-report.json"))
    value.add_argument("--generate-manifest", type=Path)
    value.add_argument("--manifest", type=Path, default=MANIFEST_PATH)
    value.add_argument("--evidence", type=Path, default=Path("branch-deletion-evidence.json"))
    value.add_argument("--recovery-evidence", type=Path)
    value.add_argument("--event-name", default=os.environ.get("GITHUB_EVENT_NAME", ""))
    value.add_argument("--ref-name", default=os.environ.get("GITHUB_REF_NAME", ""))
    value.add_argument("--run-id", default=os.environ.get("GITHUB_RUN_ID", "manual"))
    return value


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    root = args.root.resolve()
    policy_path = args.policy if args.policy.is_absolute() else root / args.policy
    policy = load_json(policy_path)
    validate_policy(policy, root)
    if args.mode == "validate-policy":
        print(f"Validated branch lifecycle policy for Issue #{ISSUE_NUMBER}.")
        return 0

    if args.snapshot is not None:
        snapshot_path = args.snapshot if args.snapshot.is_absolute() else root / args.snapshot
        snapshot = load_json(snapshot_path)
        report = classify_snapshot(policy, snapshot, root=root)
        client = None
    else:
        if not args.repo or not args.token:
            raise ValidationError("--repo and --token (or GitHub environment variables) are required")
        client = GitHubClient(args.repo, args.token, root=root)
        report = classify_snapshot(policy, fetch_live_snapshot(client, root), root=root)

    output_path = args.output if args.output.is_absolute() else root / args.output
    manifest_output = (
        args.generate_manifest
        if args.generate_manifest is None or args.generate_manifest.is_absolute()
        else root / args.generate_manifest
    )
    write_report_files(report, output_path, manifest_output)

    if args.mode == "inventory":
        print(
            f"Inventoried {len(report['branches'])} branches; "
            f"{report['deletion_candidate_count']} deletion candidates."
        )
        return 0

    manifest_path = args.manifest if args.manifest.is_absolute() else root / args.manifest
    manifest = load_json(manifest_path)
    if args.mode == "validate-manifest":
        entries = validate_manifest(manifest, report, require_apply=False)
        print(f"Validated {len(entries)} reviewed manifest entries.")
        return 0

    if client is None:
        raise ValidationError("apply mode cannot use an offline snapshot")
    apply_manifest(
        client,
        report,
        manifest,
        policy=policy,
        root=root,
        evidence_path=args.evidence if args.evidence.is_absolute() else root / args.evidence,
        event_name=args.event_name,
        ref_name=args.ref_name,
    )
    if args.recovery_evidence is not None:
        recovery_test(
            client,
            report["default_branch_sha"],
            args.run_id,
            args.recovery_evidence if args.recovery_evidence.is_absolute()
            else root / args.recovery_evidence,
        )
    print("Applied reviewed branch deletion manifest and verified evidence.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValidationError, ApiError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
