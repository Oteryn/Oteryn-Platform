#!/usr/bin/env python3
"""Offline stand-in for `gh api <endpoint>`, selected through OTERYN_GH_BIN.

Fixtures live in $FAKE_GH_DIR. An endpoint (query string removed) maps to a file by
replacing every character outside [A-Za-z0-9._-] with '_', for example
repos/o/r/actions/runs/7 -> repos_o_r_actions_runs_7.json. The workflow-run listing
repos/o/r/actions/workflows/<file>/runs?status=<s> maps to
runs_<file>_<s>.json. A missing fixture exits 1 like a gh 404.
"""
from __future__ import annotations

import os
import re
import sys
from pathlib import Path


def main() -> int:
    args = sys.argv[1:]
    if not args or args[0] != "api" or len(args) < 2:
        return 2
    endpoint = args[1]
    base = Path(os.environ["FAKE_GH_DIR"])
    match = re.match(r"^repos/[^/]+/[^/]+/actions/workflows/([^/]+)/runs\?status=([a-z_]+)", endpoint)
    if match:
        name = f"runs_{match.group(1)}_{match.group(2)}.json"
    else:
        name = re.sub(r"[^A-Za-z0-9._-]", "_", endpoint.split("?", 1)[0])
        name += ".zip" if endpoint.endswith("/zip") else ".json"
    target = base / name
    if not target.is_file():
        print(f"gh: Not Found ({name})", file=sys.stderr)
        return 1
    sys.stdout.buffer.write(target.read_bytes())
    return 0


if __name__ == "__main__":
    sys.exit(main())
