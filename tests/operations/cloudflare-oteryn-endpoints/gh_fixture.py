#!/usr/bin/env python3
"""Writes fixtures for fake_gh.py. Usage: gh_fixture.py <dir> <command> <args...>

run       <id> <path> <event> <branch> <status> <conclusion|null> <attempt>
artifact  <run_id> <name> <artifact_id> <expired true|false> <member> <json-content>
jobs      <run_id> <name>=<conclusion> ...
runs      <workflow-file> <status> [<id>:<title> ...]
"""
from __future__ import annotations

import json
import sys
import zipfile
from pathlib import Path

REPO = "repos_Oteryn_Oteryn-Platform_actions"


def write(path: Path, payload: object) -> None:
    path.write_text(json.dumps(payload), encoding="utf-8")


def main() -> None:
    base = Path(sys.argv[1])
    command = sys.argv[2]
    args = sys.argv[3:]
    base.mkdir(parents=True, exist_ok=True)
    if command == "run":
        run_id, path, event, branch, status, conclusion, attempt = args
        write(
            base / f"{REPO}_runs_{run_id}.json",
            {
                "id": int(run_id),
                "path": path,
                "event": event,
                "head_branch": branch,
                "status": status,
                "conclusion": None if conclusion == "null" else conclusion,
                "run_attempt": int(attempt),
            },
        )
    elif command == "artifact":
        run_id, name, artifact_id, expired, member, content = args
        listing = base / f"{REPO}_runs_{run_id}_artifacts.json"
        data = json.loads(listing.read_text()) if listing.exists() else {"artifacts": []}
        data["artifacts"].append({"id": int(artifact_id), "name": name, "expired": expired == "true"})
        write(listing, data)
        with zipfile.ZipFile(base / f"{REPO}_artifacts_{artifact_id}_zip.zip", "w") as archive:
            archive.writestr(member, content)
    elif command == "jobs":
        run_id = args[0]
        jobs = []
        for item in args[1:]:
            name, conclusion = item.split("=", 1)
            jobs.append({"name": name, "conclusion": None if conclusion == "null" else conclusion})
        write(base / f"{REPO}_runs_{run_id}_jobs.json", {"jobs": jobs})
    elif command == "runs":
        workflow, status = args[0], args[1]
        runs = []
        for item in args[2:]:
            run_id, title = item.split(":", 1)
            runs.append({"id": int(run_id), "display_title": title})
        write(base / f"runs_{workflow}_{status}.json", {"workflow_runs": runs})
    else:
        raise SystemExit(f"unknown command {command}")


if __name__ == "__main__":
    main()
