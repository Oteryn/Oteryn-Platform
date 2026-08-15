#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

from tibia_linux_reference import HarnessError
from tibia_linux_reference.official_host import run_official_host_preflight


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--evidence-dir", type=Path, required=True)
    parser.add_argument("--expected-user", default="oteryn-tibia-ref")
    args = parser.parse_args()

    result = run_official_host_preflight(
        repo_root=args.repo_root.resolve(strict=True),
        evidence_root=args.evidence_dir,
        expected_user=args.expected_user,
    )
    result.pop("network_prefix", None)
    print(
        json.dumps(
            {
                "result": "PASS",
                "host": result["official_host"],
                "graphics": result["official_graphics"],
                "preflight": result,
                "official_service_contacted": False,
                "binary_execution_performed": False,
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except HarnessError as error:
        print(json.dumps({"result": "FAIL", "error_class": type(error).__name__}, sort_keys=True))
        raise SystemExit(2) from error
