#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import quote
from urllib.request import Request, urlopen

REQUIRED_POLICY_KEYS = {
    "repository",
    "default_branch",
    "repository_settings",
    "branch_protection",
    "security_features",
    "environments",
}


def load_policy(path: Path) -> dict[str, Any]:
    try:
        policy = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise RuntimeError(f"Unable to read policy file: {path}") from error
    except json.JSONDecodeError as error:
        raise RuntimeError(f"Invalid repository policy JSON: {error}") from error

    if not isinstance(policy, dict):
        raise RuntimeError("Repository policy must decode to an object.")

    missing = sorted(REQUIRED_POLICY_KEYS - policy.keys())
    if missing:
        raise RuntimeError(
            "Repository policy is missing required keys: " + ", ".join(missing)
        )

    return policy


def repository_settings_patch(policy: dict[str, Any]) -> dict[str, Any]:
    settings = policy["repository_settings"]
    if not isinstance(settings, dict):
        raise RuntimeError("Repository settings policy must be an object.")
    return dict(settings)


def security_analysis_patch(policy: dict[str, Any]) -> dict[str, Any]:
    security = policy["security_features"]
    if not isinstance(security, dict):
        raise RuntimeError("Security features policy must be an object.")

    return {
        "secret_scanning": {
            "status": "enabled" if security.get("secret_scanning") else "disabled"
        },
        "secret_scanning_push_protection": {
            "status": (
                "enabled"
                if security.get("secret_scanning_push_protection")
                else "disabled"
            )
        },
    }


def repository_patch(policy: dict[str, Any]) -> dict[str, Any]:
    return {
        **repository_settings_patch(policy),
        "security_and_analysis": security_analysis_patch(policy),
    }


def branch_protection_payload(policy: dict[str, Any]) -> dict[str, Any]:
    protection = policy["branch_protection"]
    if not isinstance(protection, dict):
        raise RuntimeError("Branch protection policy must be an object.")
    return dict(protection)


def environment_payload(environment: dict[str, Any]) -> dict[str, Any]:
    reviewers: list[dict[str, Any]] = []

    for reviewer in environment.get("reviewers", []):
        if not isinstance(reviewer, dict) or "type" not in reviewer or "id" not in reviewer:
            raise RuntimeError("Every environment reviewer must contain type and id.")
        reviewers.append({"type": reviewer["type"], "id": reviewer["id"]})

    return {
        "wait_timer": int(environment.get("wait_timer", 0)),
        "prevent_self_review": bool(environment.get("prevent_self_review", False)),
        "reviewers": reviewers,
        "deployment_branch_policy": environment.get("deployment_branch_policy"),
    }


def append_drift(
    drift: list[dict[str, Any]], path: str, expected: Any, actual: Any
) -> None:
    if expected != actual:
        drift.append({"path": path, "expected": expected, "actual": actual})


def enabled(value: Any) -> bool:
    if isinstance(value, dict):
        return bool(value.get("enabled", False))
    return bool(value)


def repository_drift(
    policy: dict[str, Any], repository: dict[str, Any]
) -> list[dict[str, Any]]:
    drift: list[dict[str, Any]] = []
    for key, expected in repository_settings_patch(policy).items():
        append_drift(
            drift,
            f"repository_settings.{key}",
            expected,
            repository.get(key),
        )
    return drift


def branch_protection_drift(
    policy: dict[str, Any], actual: dict[str, Any] | None
) -> list[dict[str, Any]]:
    expected = branch_protection_payload(policy)
    if actual is None:
        return [{"path": "branch_protection", "expected": expected, "actual": None}]

    drift: list[dict[str, Any]] = []
    expected_checks = expected.get("required_status_checks") or {}
    actual_checks = actual.get("required_status_checks") or {}

    append_drift(
        drift,
        "branch_protection.required_status_checks.strict",
        expected_checks.get("strict"),
        actual_checks.get("strict"),
    )
    append_drift(
        drift,
        "branch_protection.required_status_checks.contexts",
        sorted(expected_checks.get("contexts", [])),
        sorted(actual_checks.get("contexts", [])),
    )
    append_drift(
        drift,
        "branch_protection.enforce_admins",
        bool(expected.get("enforce_admins", False)),
        enabled(actual.get("enforce_admins", False)),
    )

    expected_reviews = expected.get("required_pull_request_reviews") or {}
    actual_reviews = actual.get("required_pull_request_reviews") or {}
    for key in (
        "dismiss_stale_reviews",
        "require_code_owner_reviews",
        "required_approving_review_count",
        "require_last_push_approval",
    ):
        append_drift(
            drift,
            f"branch_protection.required_pull_request_reviews.{key}",
            expected_reviews.get(key),
            actual_reviews.get(key),
        )

    for key in (
        "required_linear_history",
        "allow_force_pushes",
        "allow_deletions",
        "block_creations",
        "required_conversation_resolution",
        "lock_branch",
        "allow_fork_syncing",
    ):
        append_drift(
            drift,
            f"branch_protection.{key}",
            bool(expected.get(key, False)),
            enabled(actual.get(key, False)),
        )

    if expected.get("restrictions") is None:
        append_drift(
            drift,
            "branch_protection.restrictions",
            None,
            actual.get("restrictions"),
        )

    return drift


def normalize_reviewer(reviewer: dict[str, Any]) -> dict[str, Any]:
    nested = reviewer.get("reviewer")
    reviewer_data = nested if isinstance(nested, dict) else reviewer
    return {
        "type": reviewer.get("type", reviewer_data.get("type")),
        "id": reviewer_data.get("id"),
    }


def reviewer_sort_key(reviewer: dict[str, Any]) -> tuple[str, str]:
    return str(reviewer.get("type")), str(reviewer.get("id"))


def environment_drift(
    name: str, expected: dict[str, Any], actual: dict[str, Any]
) -> list[dict[str, Any]]:
    wait_timer = 0
    prevent_self_review = False
    actual_reviewers: list[dict[str, Any]] = []

    for rule in actual.get("protection_rules", []):
        if not isinstance(rule, dict):
            continue
        if rule.get("type") == "wait_timer":
            wait_timer = int(rule.get("wait_timer", 0))
        if rule.get("type") == "required_reviewers":
            prevent_self_review = bool(rule.get("prevent_self_review", False))
            actual_reviewers.extend(
                normalize_reviewer(reviewer)
                for reviewer in rule.get("reviewers", [])
                if isinstance(reviewer, dict)
            )

    expected_reviewers = [
        {"type": reviewer.get("type"), "id": reviewer.get("id")}
        for reviewer in expected.get("reviewers", [])
        if isinstance(reviewer, dict)
    ]

    drift: list[dict[str, Any]] = []
    append_drift(
        drift,
        f"environments.{name}.wait_timer",
        int(expected.get("wait_timer", 0)),
        wait_timer,
    )
    append_drift(
        drift,
        f"environments.{name}.prevent_self_review",
        bool(expected.get("prevent_self_review", False)),
        prevent_self_review,
    )
    append_drift(
        drift,
        f"environments.{name}.reviewers",
        sorted(expected_reviewers, key=reviewer_sort_key),
        sorted(actual_reviewers, key=reviewer_sort_key),
    )
    append_drift(
        drift,
        f"environments.{name}.deployment_branch_policy",
        expected.get("deployment_branch_policy"),
        actual.get("deployment_branch_policy"),
    )
    append_drift(
        drift,
        f"environments.{name}.can_admins_bypass",
        bool(expected.get("expected_can_admins_bypass", False)),
        bool(actual.get("can_admins_bypass", True)),
    )
    return drift


class GitHubApiClient:
    def __init__(self, repository: str, token: str | None) -> None:
        self.repository = repository
        self.token = token

    def request(
        self, method: str, path: str, body: dict[str, Any] | None = None
    ) -> tuple[int, Any]:
        url = f"https://api.github.com/repos/{self.repository}{path}"
        headers = {
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "Oteryn-Repository-Policy",
        }
        if self.token:
            headers["Authorization"] = f"Bearer {self.token}"

        data = None
        if body is not None:
            headers["Content-Type"] = "application/json"
            data = json.dumps(body).encode("utf-8")

        request = Request(url, data=data, headers=headers, method=method)
        try:
            with urlopen(request, timeout=20) as response:
                raw = response.read()
                return response.status, json.loads(raw) if raw else None
        except HTTPError as error:
            raw = error.read()
            try:
                parsed = json.loads(raw) if raw else None
            except json.JSONDecodeError:
                parsed = raw.decode("utf-8", errors="replace")
            return error.code, parsed
        except URLError as error:
            raise RuntimeError(
                f"GitHub API request failed before receiving an HTTP response: "
                f"{method} {path}: {error.reason}"
            ) from error


def collect_state(
    client: GitHubApiClient, policy: dict[str, Any]
) -> tuple[list[dict[str, Any]], list[str]]:
    drift: list[dict[str, Any]] = []
    errors: list[str] = []

    status, repository = client.request("GET", "")
    if status == 200 and isinstance(repository, dict):
        drift.extend(repository_drift(policy, repository))
        analysis = repository.get("security_and_analysis") or {}
        for feature in ("secret_scanning", "secret_scanning_push_protection"):
            expected = bool(policy["security_features"].get(feature, False))
            actual = (analysis.get(feature) or {}).get("status") == "enabled"
            append_drift(drift, f"security_features.{feature}", expected, actual)
    else:
        errors.append(f"Unable to read repository settings (HTTP {status}).")

    branch = quote(str(policy["default_branch"]), safe="")
    status, protection = client.request("GET", f"/branches/{branch}/protection")
    if status == 200 and isinstance(protection, dict):
        drift.extend(branch_protection_drift(policy, protection))
    elif status == 404:
        drift.extend(branch_protection_drift(policy, None))
    else:
        errors.append(f"Unable to read branch protection (HTTP {status}).")

    feature_endpoints = {
        "dependabot_alerts": "/vulnerability-alerts",
        "dependabot_security_updates": "/automated-security-fixes",
    }
    for feature, endpoint in feature_endpoints.items():
        status, payload = client.request("GET", endpoint)
        if status == 204:
            actual_enabled = True
        elif status == 404:
            actual_enabled = False
        elif status == 200 and isinstance(payload, dict) and isinstance(payload.get("enabled"), bool):
            actual_enabled = payload["enabled"]
        else:
            errors.append(f"Unable to read {feature} state (HTTP {status}).")
            continue
        append_drift(
            drift,
            f"security_features.{feature}",
            bool(policy["security_features"].get(feature, False)),
            actual_enabled,
        )

    status, private_reporting = client.request("GET", "/private-vulnerability-reporting")
    if status == 200 and isinstance(private_reporting, dict):
        append_drift(
            drift,
            "security_features.private_vulnerability_reporting",
            bool(
                policy["security_features"].get(
                    "private_vulnerability_reporting", False
                )
            ),
            bool(private_reporting.get("enabled", False)),
        )
    else:
        errors.append(
            f"Unable to read private vulnerability reporting state (HTTP {status})."
        )

    for name, expected in policy["environments"].items():
        status, environment = client.request("GET", f"/environments/{quote(name, safe='')}")
        if status == 200 and isinstance(environment, dict) and isinstance(expected, dict):
            drift.extend(environment_drift(name, expected, environment))
        else:
            errors.append(f"Unable to read environment {name} (HTTP {status}).")

    return drift, errors


def apply_policy(client: GitHubApiClient, policy: dict[str, Any]) -> list[str]:
    branch = quote(str(policy["default_branch"]), safe="")
    operations: list[tuple[str, str, dict[str, Any] | None, str]] = [
        ("PATCH", "", repository_settings_patch(policy), "repository settings"),
        (
            "PATCH",
            "",
            {"security_and_analysis": security_analysis_patch(policy)},
            "secret scanning settings",
        ),
        (
            "PUT",
            f"/branches/{branch}/protection",
            branch_protection_payload(policy),
            "branch protection",
        ),
        ("PUT", "/vulnerability-alerts", None, "Dependabot alerts"),
        ("PUT", "/automated-security-fixes", None, "Dependabot security updates"),
        (
            "PUT",
            "/private-vulnerability-reporting",
            None,
            "private vulnerability reporting",
        ),
    ]

    for name, environment in policy["environments"].items():
        if isinstance(environment, dict):
            operations.append(
                (
                    "PUT",
                    f"/environments/{quote(name, safe='')}",
                    environment_payload(environment),
                    f"environment {name}",
                )
            )

    errors: list[str] = []
    for method, path, body, label in operations:
        status, response = client.request(method, path, body)
        if status in (200, 201, 204):
            continue
        message = response.get("message", "unknown error") if isinstance(response, dict) else response
        errors.append(f"Failed to apply {label} (HTTP {status}): {message}")
    return errors


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Verify or apply the Oteryn Platform GitHub repository policy."
    )
    parser.add_argument(
        "--policy",
        type=Path,
        default=Path("docs/operations/github-repository-policy.json"),
    )
    parser.add_argument("--repo")
    parser.add_argument("--apply", action="store_true")
    parser.add_argument("--json", action="store_true")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)

    try:
        policy = load_policy(args.policy)
        repository = args.repo or str(policy["repository"])
        token = os.getenv("GITHUB_TOKEN")
        if args.apply and not token:
            raise RuntimeError(
                "--apply requires GITHUB_TOKEN with repository administration permission."
            )

        client = GitHubApiClient(repository, token)
        apply_errors = apply_policy(client, policy) if args.apply else []
        drift, read_errors = collect_state(client, policy)
        errors = [*apply_errors, *read_errors]
        result = {
            "repository": repository,
            "mode": "apply-and-verify" if args.apply else "verify",
            "compliant": not drift and not errors,
            "drift": drift,
            "errors": errors,
        }

        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            state = "COMPLIANT" if result["compliant"] else "NON_COMPLIANT"
            print(
                f"{repository}: {state} "
                f"({len(drift)} drift item(s), {len(errors)} error(s))"
            )
            for item in drift:
                print(
                    f"- DRIFT {item['path']}: expected "
                    f"{json.dumps(item['expected'], sort_keys=True)}, actual "
                    f"{json.dumps(item['actual'], sort_keys=True)}"
                )
            for error in errors:
                print(f"- ERROR {error}", file=sys.stderr)

        if errors:
            return 3
        return 0 if not drift else 2
    except (RuntimeError, TypeError, ValueError) as error:
        print(f"Repository policy error: {error}", file=sys.stderr)
        return 3


if __name__ == "__main__":
    raise SystemExit(main())
