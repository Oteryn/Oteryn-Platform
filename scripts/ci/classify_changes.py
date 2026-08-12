#!/usr/bin/env python3
from __future__ import annotations

import argparse
import fnmatch
import json
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

GATES = (
    "ci",
    "phase7",
    "edge",
    "db_outage",
    "game_auth_concurrency",
)

ALL_GATES = frozenset(GATES)
NO_GATES = frozenset()
POLICY_FIXTURES = Path(__file__).resolve().parents[2] / "tests/ci/fixtures/change-routing-cases.json"


@dataclass(frozen=True)
class PathClassification:
    change_class: str
    gates: frozenset[str]


def _matches(path: str, patterns: Iterable[str]) -> bool:
    return any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns)


def _is_root_document(path: str) -> bool:
    if "/" in path:
        return False
    return path.endswith(".md") or path == "LICENSE" or path.startswith("LICENSE.")


def classify_path(raw_path: str) -> PathClassification:
    path = raw_path.strip().replace("\\", "/").removeprefix("./")
    lowered = path.casefold()

    if not path:
        return PathClassification("shared", ALL_GATES)

    # The Agent Governance workflow is itself part of the governance surface.
    # Treating this one file as a generic workflow would fan a governance-only
    # change into Phase 7, edge, DB-outage and game-auth runtime proof. The
    # dedicated Agent Governance workflow validates its own executable change.
    if path == ".github/workflows/agent-governance.yml":
        return PathClassification("agent_governance", NO_GATES)

    if _matches(
        path,
        (
            ".github/workflows/**",
            "scripts/ci/**",
            "tests/ci/**",
        ),
    ):
        return PathClassification("workflow", ALL_GATES)

    if _matches(
        path,
        (
            "AGENTS.md",
            "AGENTS.override.md",
            "docs/agents/**",
            "tools/agents/**",
            ".github/ISSUE_TEMPLATE/**",
            ".github/PULL_REQUEST_TEMPLATE/**",
            ".github/PULL_REQUEST_TEMPLATE.md",
            ".github/pull_request_template.md",
        ),
    ):
        return PathClassification("agent_governance", NO_GATES)

    if _matches(path, ("docs/contracts/**",)):
        return PathClassification("shared", ALL_GATES)

    if _matches(path, ("docs/**",)) or _is_root_document(path):
        return PathClassification("docs_only", NO_GATES)

    if _matches(path, ("services/game-gateway/**",)):
        return PathClassification("go_gateway", NO_GATES)

    if _matches(
        path,
        (
            "composer.json",
            "composer.lock",
            "package.json",
            "package-lock.json",
            "pnpm-lock.yaml",
            "yarn.lock",
            "phpunit.xml",
            "phpunit.xml.dist",
            "phpstan.neon",
            "phpstan.neon.dist",
            "pint.json",
        ),
    ):
        return PathClassification("dependency", ALL_GATES)

    if _matches(
        path,
        (
            "database/**",
            "config/database.php",
            "tests/**/Database/**",
            "tests/**/*Migration*",
            "tests/**/*Database*",
        ),
    ):
        return PathClassification("database", ALL_GATES)

    if _matches(
        path,
        (
            "Dockerfile",
            "Dockerfile.*",
            "docker/**",
            "deploy/**",
            "ops/**",
            "compose.yml",
            "compose.yaml",
            "docker-compose.yml",
            "docker-compose.yaml",
            ".dockerignore",
            "scripts/operations/**",
            "tests/operations/**",
        ),
    ):
        return PathClassification("deployment", ALL_GATES)

    edge_tokens = (
        "edge",
        "proxy",
        "trustedproxy",
        "trusted_proxy",
        "securityheader",
        "security_header",
        "cloudflare",
        "tunnel",
        "tls",
        "waf",
        "hsts",
    )
    if _matches(path, ("tests/edge-emulation/**",)) or any(
        token in lowered for token in edge_tokens
    ):
        return PathClassification("edge", frozenset(("ci", "phase7", "edge")))

    auth_tokens = (
        "auth",
        "identity",
        "session",
        "password",
        "mfa",
        "rbac",
        "permission",
        "policy",
        "gameauth",
        "game_auth",
        "ticket",
    )
    if any(token in lowered for token in auth_tokens):
        return PathClassification("auth_security", ALL_GATES)

    payment_tokens = (
        "payment",
        "webhook",
        "wallet",
        "marketplace",
        "bazaar",
        "balance",
        "entitlement",
        "voucher",
        "refund",
        "chargeback",
    )
    if any(token in lowered for token in payment_tokens):
        return PathClassification("payment", ALL_GATES)

    if _matches(
        path,
        (
            "resources/views/**",
            "resources/js/**",
            "resources/css/**",
            "resources/lang/**",
            "public/**",
            "vite.config.*",
        ),
    ):
        return PathClassification("frontend", frozenset(("ci",)))

    if _matches(path, ("app/**", "tests/**")):
        return PathClassification("backend", frozenset(("ci", "phase7")))

    if _matches(
        path,
        (
            "bootstrap/**",
            "config/**",
            "routes/**",
            ".env.example",
            "artisan",
            "scripts/acceptance/**",
        ),
    ):
        return PathClassification("shared", ALL_GATES)

    return PathClassification("shared", ALL_GATES)


def classify_paths(paths: Iterable[str], *, force_all: bool = False) -> dict[str, object]:
    normalized = sorted(
        {
            path.strip().replace("\\", "/").removeprefix("./")
            for path in paths
            if path.strip()
        }
    )

    if force_all or not normalized:
        classes = ["shared"]
        gates = {gate: True for gate in GATES}
    else:
        classifications = [classify_path(path) for path in normalized]
        classes = sorted({item.change_class for item in classifications})
        affected = set().union(*(item.gates for item in classifications))
        gates = {gate: gate in affected for gate in GATES}

    return {"classes": classes, "paths": normalized, "gates": gates}


def validate_policy_contract(path: Path = POLICY_FIXTURES) -> int:
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
        cases = raw["cases"]
    except (OSError, json.JSONDecodeError, KeyError, TypeError) as exc:
        raise ValueError(f"invalid CI routing fixture contract: {exc}") from exc

    if not isinstance(cases, list) or not cases:
        raise ValueError("CI routing fixture contract must contain non-empty cases")

    names: set[str] = set()
    for index, case in enumerate(cases, start=1):
        if not isinstance(case, dict):
            raise ValueError(f"fixture case {index} must be an object")
        name = str(case.get("name", "")).strip()
        paths = case.get("paths")
        expected_classes = case.get("classes")
        expected_gates = case.get("gates")
        if not name or name in names:
            raise ValueError(f"fixture case {index} has an empty or duplicate name")
        names.add(name)
        if not isinstance(paths, list) or not all(isinstance(item, str) for item in paths):
            raise ValueError(f"fixture {name} paths must be a list of strings")
        if not isinstance(expected_classes, list) or not all(
            isinstance(item, str) for item in expected_classes
        ):
            raise ValueError(f"fixture {name} classes must be a list of strings")
        if not isinstance(expected_gates, list) or not all(
            isinstance(item, str) for item in expected_gates
        ):
            raise ValueError(f"fixture {name} gates must be a list of strings")
        unknown_gates = sorted(set(expected_gates) - set(GATES))
        if unknown_gates:
            raise ValueError(f"fixture {name} contains unknown gates: {unknown_gates}")

        result = classify_paths(paths)
        actual_gates = sorted(gate for gate, enabled in result["gates"].items() if enabled)
        if result["classes"] != expected_classes or actual_gates != sorted(expected_gates):
            raise ValueError(
                f"fixture {name} mismatch: classes={result['classes']} gates={actual_gates}"
            )
    return len(cases)


def changed_paths(base: str, head: str) -> list[str]:
    if not base or not head:
        raise ValueError("both --base and --head are required for git diff classification")
    output = subprocess.check_output(
        ["git", "diff", "--name-only", "--diff-filter=ACMRD", base, head],
        text=True,
    )
    return [line for line in output.splitlines() if line.strip()]


def write_github_output(path: Path, result: dict[str, object]) -> None:
    gates = result["gates"]
    assert isinstance(gates, dict)
    classes = result["classes"]
    paths = result["paths"]
    with path.open("a", encoding="utf-8") as handle:
        for gate in GATES:
            handle.write(f"{gate}={'true' if gates[gate] else 'false'}\n")
        handle.write(f"classes={','.join(str(item) for item in classes)}\n")
        handle.write(f"paths_json={json.dumps(paths, separators=(',', ':'))}\n")


def write_summary(path: Path, result: dict[str, object], fixture_count: int) -> None:
    gates = result["gates"]
    assert isinstance(gates, dict)
    classes = result["classes"]
    paths = result["paths"]
    lines = [
        "### Pull-request change classification",
        "",
        f"- classes: `{', '.join(str(item) for item in classes)}`",
        f"- changed paths: `{len(paths)}`",
        f"- policy fixtures validated: `{fixture_count}`",
        "- classification is routing evidence only; skipped jobs are not product-validation evidence.",
        "",
        "| Gate | Heavy internals |",
        "|---|---|",
    ]
    for gate in GATES:
        lines.append(f"| `{gate}` | `{'RUN' if gates[gate] else 'SKIP'}` |")
    lines.append("")
    with path.open("a", encoding="utf-8") as handle:
        handle.write("\n".join(lines))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Classify changed paths for Oteryn CI routing.")
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--all", action="store_true", help="Mark every heavy gate affected.")
    source.add_argument("--paths", nargs="*", help="Classify explicit repository paths.")
    source.add_argument("--base", help="Base commit for git diff classification.")
    parser.add_argument("--head", help="Head commit for git diff classification.")
    parser.add_argument("--github-output", type=Path)
    parser.add_argument("--summary", type=Path)
    parser.add_argument("--json", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    fixture_count = validate_policy_contract()
    if args.all:
        paths: list[str] = []
        force_all = True
    elif args.paths is not None:
        paths = list(args.paths)
        force_all = False
    else:
        paths = changed_paths(args.base, args.head)
        force_all = False

    result = classify_paths(paths, force_all=force_all)
    if args.github_output:
        write_github_output(args.github_output, result)
    if args.summary:
        write_summary(args.summary, result, fixture_count)
    if args.json or (not args.github_output and not args.summary):
        print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
