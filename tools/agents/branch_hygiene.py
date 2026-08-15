#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import re
import sys
from pathlib import Path
from typing import Any

import historical_branch_audit as audit
from branch_lifecycle import ApiError, GitHubClient, ValidationError

ALLOWED_REPOSITORY = "blakinio/Oteryn-Platform"
DEFAULT_BRANCH = "main"
FORBIDDEN_ACTIVE_TOP_LEVEL = {"archive", "backup", "recovery", "rollback", "tmp"}
BOT_OR_SYSTEM_PREFIXES = (
    "dependabot/",
    "github-actions/",
    "renovate/",
    "copilot/",
)
PREFERRED_TASK_BRANCH_RE = re.compile(
    r"^(?:feat|fix|refactor|docs|ops|research|test|chore|repair|audit)/"
    r"issue-[1-9][0-9]*-[a-z0-9][a-z0-9._-]*$"
)
EXPECTED_REPOSITORY_SETTINGS = {
    "default_branch": "main",
    "allow_squash_merge": True,
    "allow_merge_commit": False,
    "allow_rebase_merge": False,
    "delete_branch_on_merge": True,
}


def canonical(value: object) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(canonical(value), encoding="utf-8")


def validate_scope(repo: str, default_branch: str) -> None:
    if repo.casefold() != ALLOWED_REPOSITORY.casefold():
        raise ValidationError(
            f"steady-state branch hygiene is scoped to {ALLOWED_REPOSITORY}; got {repo!r}"
        )
    if default_branch != DEFAULT_BRANCH:
        raise ValidationError(
            f"steady-state branch hygiene requires default branch {DEFAULT_BRANCH!r}; "
            f"got {default_branch!r}"
        )


def _same_repo_open_pull(pull: object, repo: str) -> tuple[str, int] | None:
    if not isinstance(pull, dict) or pull.get("state") != "open":
        return None
    head = pull.get("head")
    if not isinstance(head, dict):
        return None
    head_repo = head.get("repo")
    branch = head.get("ref")
    number = pull.get("number")
    if (
        not isinstance(head_repo, dict)
        or not isinstance(head_repo.get("full_name"), str)
        or head_repo["full_name"].casefold() != repo.casefold()
        or not isinstance(branch, str)
        or not branch
        or not isinstance(number, int)
        or isinstance(number, bool)
    ):
        return None
    return branch, number


def open_pr_numbers_by_branch(client: GitHubClient) -> dict[str, list[int]]:
    raw = client.paginate(f"/repos/{client.repo}/pulls?state=open&per_page=100")
    grouped: dict[str, set[int]] = {}
    for pull in raw:
        parsed = _same_repo_open_pull(pull, client.repo)
        if parsed is None:
            continue
        branch, number = parsed
        grouped.setdefault(branch, set()).add(number)
    return {branch: sorted(numbers) for branch, numbers in sorted(grouped.items())}


def repository_settings(client: GitHubClient) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    payload, _ = client.request("GET", f"/repos/{client.repo}")
    if not isinstance(payload, dict):
        raise ValidationError("repository metadata response must be an object")
    observed = {
        key: payload.get(key)
        for key in EXPECTED_REPOSITORY_SETTINGS
    }

    merge_fields = (
        "allow_squash_merge",
        "allow_merge_commit",
        "allow_rebase_merge",
        "delete_branch_on_merge",
    )
    if any(observed.get(field) is None for field in merge_fields):
        owner, name = client.repo.split("/", 1)
        query = """
        query RepositoryMergeSettings($owner: String!, $name: String!) {
          repository(owner: $owner, name: $name) {
            defaultBranchRef { name }
            mergeCommitAllowed
            rebaseMergeAllowed
            squashMergeAllowed
            deleteBranchOnMerge
          }
        }
        """
        graphql, _ = client.request(
            "POST",
            "/graphql",
            data={"query": query, "variables": {"owner": owner, "name": name}},
        )
        repository = (
            graphql.get("data", {}).get("repository")
            if isinstance(graphql, dict)
            else None
        )
        if not isinstance(repository, dict):
            raise ValidationError(
                "repository merge settings are unavailable from REST and GraphQL"
            )
        default_ref = repository.get("defaultBranchRef")
        if observed.get("default_branch") is None and isinstance(default_ref, dict):
            observed["default_branch"] = default_ref.get("name")
        observed.update(
            {
                "allow_squash_merge": repository.get("squashMergeAllowed"),
                "allow_merge_commit": repository.get("mergeCommitAllowed"),
                "allow_rebase_merge": repository.get("rebaseMergeAllowed"),
                "delete_branch_on_merge": repository.get("deleteBranchOnMerge"),
            }
        )

    findings = repository_setting_findings(observed)
    return observed, findings


def repository_setting_findings(observed: dict[str, Any]) -> list[dict[str, Any]]:
    findings: list[dict[str, Any]] = []
    for field, expected in EXPECTED_REPOSITORY_SETTINGS.items():
        actual = observed.get(field)
        if actual == expected:
            continue
        findings.append(
            {
                "scope": "repository",
                "code": "REPOSITORY_SETTING_DRIFT",
                "field": field,
                "expected": expected,
                "actual": actual,
            }
        )
    return findings


def is_bot_or_system_branch(branch: str) -> bool:
    folded = branch.casefold()
    return any(folded.startswith(prefix) for prefix in BOT_OR_SYSTEM_PREFIXES)


def has_preferred_task_name(branch: str) -> bool:
    return bool(PREFERRED_TASK_BRANCH_RE.fullmatch(branch))


def _branch_finding(
    branch: str,
    code: str,
    *,
    detail: str,
) -> dict[str, Any]:
    return {
        "scope": "branch",
        "branch": branch,
        "code": code,
        "detail": detail,
    }


def evaluate_snapshot(
    *,
    branches: list[dict[str, Any]],
    open_prs: dict[str, list[int]],
    active_claims: dict[str, list[str]],
    settings: dict[str, Any],
    settings_findings: list[dict[str, Any]],
    default_branch: str = DEFAULT_BRANCH,
) -> dict[str, Any]:
    branch_findings: list[dict[str, Any]] = []
    advisories: list[dict[str, Any]] = []
    accounted: list[dict[str, Any]] = []
    new_unexplained: set[str] = set()

    for item in sorted(branches, key=lambda row: str(row.get("branch", ""))):
        branch = item.get("branch")
        head_sha = item.get("head_sha")
        protected = item.get("protected")
        if not isinstance(branch, str) or not branch:
            raise ValidationError("live branch row has invalid branch name")
        if not isinstance(head_sha, str) or not audit.FULL_SHA_RE.fullmatch(head_sha):
            raise ValidationError(f"{branch}: live branch row has invalid head SHA")
        if not isinstance(protected, bool):
            raise ValidationError(f"{branch}: live branch row has invalid protection state")

        prs = sorted(set(open_prs.get(branch, [])))
        claims = sorted(set(active_claims.get(branch, [])))

        if branch == default_branch:
            if not protected:
                branch_findings.append(
                    _branch_finding(
                        branch,
                        "DEFAULT_BRANCH_NOT_PROTECTED",
                        detail="protected main is required",
                    )
                )
            accounted.append(
                {
                    "branch": branch,
                    "head_sha": head_sha,
                    "classification": "PROTECTED" if protected else "INVALID",
                    "open_pr_numbers": prs,
                    "active_claims": claims,
                }
            )
            continue

        if protected:
            branch_findings.append(
                _branch_finding(
                    branch,
                    "NON_MAIN_PROTECTED_BRANCH",
                    detail="main is the only ordinary long-lived protected branch",
                )
            )

        top_level = re.split(r"[/_.-]+", branch, maxsplit=1)[0].casefold()
        if top_level in FORBIDDEN_ACTIVE_TOP_LEVEL:
            branch_findings.append(
                _branch_finding(
                    branch,
                    "FORBIDDEN_ACTIVE_NAMESPACE",
                    detail=(
                        "routine remote temporary/archive/recovery namespaces are forbidden; "
                        "use local execution state or the managed-recovery contract"
                    ),
                )
            )

        if len(prs) > 1:
            branch_findings.append(
                _branch_finding(
                    branch,
                    "MULTIPLE_OPEN_PRS",
                    detail=f"same branch has multiple open same-repository PRs: {prs}",
                )
            )
        if len(claims) > 1:
            branch_findings.append(
                _branch_finding(
                    branch,
                    "MULTIPLE_ACTIVE_CLAIMS",
                    detail=f"same branch has multiple active task claims: {claims}",
                )
            )

        if not prs and not claims:
            new_unexplained.add(branch)
            branch_findings.append(
                _branch_finding(
                    branch,
                    "UNEXPLAINED_REMOTE_BRANCH",
                    detail="branch has no open same-repository PR and no active task claim",
                )
            )

        classification = "ACTIVE" if prs or claims else "UNEXPLAINED"
        accounted.append(
            {
                "branch": branch,
                "head_sha": head_sha,
                "classification": classification,
                "protected": protected,
                "open_pr_numbers": prs,
                "active_claims": claims,
            }
        )

        if (
            classification == "ACTIVE"
            and not is_bot_or_system_branch(branch)
            and not has_preferred_task_name(branch)
        ):
            advisories.append(
                {
                    "branch": branch,
                    "code": "NON_PREFERRED_TASK_BRANCH_NAME",
                    "preferred_pattern": "<type>/issue-<number>-<slug>",
                }
            )

    all_findings = [*branch_findings, *settings_findings]
    return {
        "schema_version": 1,
        "repository": ALLOWED_REPOSITORY,
        "default_branch": default_branch,
        "hard_target": {"NEW_UNEXPLAINED_BRANCHES": 0},
        "raw_branch_count_cap": None,
        "branch_count": len(branches),
        "accounted": accounted,
        "new_unexplained_branches": sorted(new_unexplained),
        "new_unexplained_branch_count": len(new_unexplained),
        "branch_findings": branch_findings,
        "repository_settings": settings,
        "repository_setting_findings": settings_findings,
        "findings": all_findings,
        "finding_count": len(all_findings),
        "advisories": advisories,
        "advisory_count": len(advisories),
    }


def build_report(
    client: GitHubClient,
    *,
    root: Path,
    default_branch: str,
) -> dict[str, Any]:
    validate_scope(client.repo, default_branch)
    branches = audit.list_live_branches(client)
    open_prs = open_pr_numbers_by_branch(client)
    active_claims = audit.active_task_branches(root)
    settings, settings_findings = repository_settings(client)
    return evaluate_snapshot(
        branches=branches,
        open_prs=open_prs,
        active_claims=active_claims,
        settings=settings,
        settings_findings=settings_findings,
        default_branch=default_branch,
    )


def parser() -> argparse.ArgumentParser:
    value = argparse.ArgumentParser(
        description="Oteryn steady-state remote branch hygiene audit"
    )
    value.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", ""))
    value.add_argument("--token", default=os.environ.get("GITHUB_TOKEN", ""))
    value.add_argument("--root", type=Path, default=Path("."))
    value.add_argument("--default-branch", default=DEFAULT_BRANCH)
    value.add_argument("--output", type=Path, required=True)
    return value


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    if not args.repo or "/" not in args.repo or not args.token:
        raise ValidationError("repo owner/name and GitHub token are required")
    validate_scope(args.repo, args.default_branch)
    root = audit.validate_repository_root(args.root.resolve())
    client = GitHubClient(args.repo, args.token, root=root)
    report = build_report(
        client,
        root=root,
        default_branch=args.default_branch,
    )
    write_json(args.output, report)
    print(
        "steady-state branch hygiene: "
        f"branches={report['branch_count']} "
        f"new_unexplained={report['new_unexplained_branch_count']} "
        f"findings={report['finding_count']} "
        f"advisories={report['advisory_count']}"
    )
    if report["finding_count"]:
        codes = sorted({item["code"] for item in report["findings"]})
        raise ValidationError(
            "steady-state branch hygiene findings: " + ", ".join(codes)
        )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValidationError, ApiError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
