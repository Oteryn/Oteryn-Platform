#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess
from pathlib import Path

import classify_changes

ZERO_SHA = "0" * 40


def classify_push_range(base: str, head: str) -> tuple[dict[str, object], bool]:
    normalized_base = base.strip()
    normalized_head = head.strip()
    fail_closed = (
        not normalized_base
        or not normalized_head
        or normalized_base == ZERO_SHA
        or normalized_head == ZERO_SHA
    )

    if not fail_closed:
        try:
            paths = classify_changes.changed_paths(normalized_base, normalized_head)
        except (ValueError, OSError, subprocess.SubprocessError):
            fail_closed = True
        else:
            if paths:
                return classify_changes.classify_paths(paths), False
            fail_closed = True

    return classify_changes.classify_paths([], force_all=True), True


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Classify a GitHub push range while failing closed on unusable SHAs."
    )
    parser.add_argument("--base", default="")
    parser.add_argument("--head", default="")
    parser.add_argument("--github-output", type=Path)
    parser.add_argument("--summary", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    fixture_count = classify_changes.validate_policy_contract()
    result, fail_closed = classify_push_range(args.base, args.head)

    if args.github_output:
        classify_changes.write_github_output(args.github_output, result)
    if args.summary:
        classify_changes.write_summary(args.summary, result, fixture_count)
        if fail_closed:
            with args.summary.open("a", encoding="utf-8") as handle:
                handle.write(
                    "\n> Push range was missing, zero, empty or unusable; all heavy gates were enabled fail-closed.\n"
                )
    if not args.github_output and not args.summary:
        print(result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
