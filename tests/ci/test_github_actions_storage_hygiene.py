#!/usr/bin/env python3
import datetime as dt
import importlib.util
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts" / "ci" / "github_actions_storage_hygiene.py"
SPEC = importlib.util.spec_from_file_location("github_actions_storage_hygiene", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)

Candidate = MODULE.Candidate

assert MODULE.pr_number_from_cache_ref("refs/pull/495/merge") == 495
assert MODULE.pr_number_from_cache_ref("refs/heads/main") is None
assert MODULE.pr_number_from_cache_ref("refs/pull/495/head") is None
assert MODULE.pr_number_from_cache_ref("refs/pull/0/merge") is None

caches = [
    {"id": 1, "ref": "refs/pull/495/merge", "size_in_bytes": 10},
    {"id": 2, "ref": "refs/pull/496/merge", "size_in_bytes": 20},
    {"id": 3, "ref": "refs/heads/main", "size_in_bytes": 30},
    {"id": 4, "ref": "refs/heads/feature", "size_in_bytes": 40},
]
closed = MODULE.closed_pr_cache_candidates(caches, {496})
assert [(item.resource_id, item.ref) for item in closed] == [
    (1, "refs/pull/495/merge")
]
exact = MODULE.exact_pr_cache_candidates(caches, 496)
assert [item.resource_id for item in exact] == [2]

now = dt.datetime(2026, 8, 11, 7, 0, tzinfo=dt.timezone.utc)
artifacts = [
    {
        "id": 10,
        "created_at": "2026-07-27T06:59:59Z",
        "size_in_bytes": 100,
        "workflow_run": {"id": 20},
    },
    {
        "id": 11,
        "created_at": "2026-07-28T07:00:00Z",
        "size_in_bytes": 200,
        "workflow_run": {"id": 21},
    },
]
old_artifacts = MODULE.old_artifact_candidates(artifacts, now, 14)
assert [(item.resource_id, item.parent_run_id) for item in old_artifacts] == [
    (10, 20)
]
assert MODULE.artifact_bytes_by_run(artifacts) == {20: 100, 21: 200}

runs = [
    {"id": 20, "status": "completed", "created_at": "2026-07-11T06:59:59Z"},
    {"id": 21, "status": "completed", "created_at": "2026-07-12T07:00:00Z"},
    {"id": 22, "status": "in_progress", "created_at": "2026-07-01T00:00:00Z"},
]
old_runs = MODULE.old_completed_run_candidates(runs, now, 30, {20: 100})
assert [(item.resource_id, item.size_in_bytes) for item in old_runs] == [(20, 100)]

choices = MODULE.choose_candidates(
    [
        Candidate("cache", 30, 10, "test"),
        Candidate("artifact", 31, 500, "test"),
        Candidate("cache", 32, 100, "test"),
        Candidate("run", 33, 0, "test"),
    ],
    2,
)
assert [item.resource_id for item in choices] == [31, 32]
assert MODULE.choose_candidates(choices, 0) == []

parent_aware_choices = MODULE.choose_candidates(
    [
        Candidate("run", 50, 600, "old run"),
        Candidate("artifact", 51, 500, "old artifact", parent_run_id=50),
        Candidate("cache", 52, 100, "closed PR cache"),
    ],
    2,
)
assert [(item.kind, item.resource_id) for item in parent_aware_choices] == [
    ("run", 50),
    ("cache", 52),
], "a selected run must cover its child artifact without consuming another delete slot"


class FakeRunSearchApi:
    """Force the first filtered window to hit GitHub's 1,000-result ceiling."""

    def __init__(self) -> None:
        self.probe_calls = 0
        self.paginate_calls = 0

    def request(self, method, path, *, params=None, **_kwargs):
        assert method == "GET"
        assert path == "actions/runs"
        assert params is not None
        assert params["status"] == "completed"
        assert ".." in str(params["created"])
        self.probe_calls += 1
        return {"total_count": 1000 if self.probe_calls == 1 else 1}

    def paginate(self, path, key, *, params=None):
        assert path == "actions/runs"
        assert key == "workflow_runs"
        assert params is not None
        self.paginate_calls += 1
        return [
            {
                "id": 100 + self.paginate_calls,
                "status": "completed",
                "created_at": "2026-06-01T00:00:00Z",
            }
        ]


fake_api = FakeRunSearchApi()
partitioned_runs = MODULE.list_completed_runs_window(
    fake_api,
    dt.datetime(2026, 6, 1, tzinfo=dt.timezone.utc),
    dt.datetime(2026, 6, 3, tzinfo=dt.timezone.utc),
)
assert fake_api.probe_calls == 3
assert fake_api.paginate_calls == 2
assert {run["id"] for run in partitioned_runs} == {101, 102}

assert MODULE.deletion_path(Candidate("cache", 40, 0, "test")) == "actions/caches/40"
assert MODULE.deletion_path(Candidate("artifact", 41, 0, "test")) == "actions/artifacts/41"
assert MODULE.deletion_path(Candidate("run", 42, 0, "test")) == "actions/runs/42"

try:
    MODULE.deletion_path(Candidate("package", 43, 0, "test"))
except ValueError:
    pass
else:
    raise AssertionError("unsupported resource kind did not fail closed")

source = MODULE_PATH.read_text(encoding="utf-8")
for required in (
    "RUN_SEARCH_RESULT_CEILING",
    "list_completed_runs_window(api, start, midpoint)",
    "list_completed_runs_window(api, midpoint, end)",
    "parent_run_id in selected_run_ids",
):
    assert required in source, f"missing bounded-cleanup contract marker: {required}"

for forbidden in (
    "/packages/",
    "/releases/",
    "/environments/",
    "/git/refs/",
    "docker system prune",
    "gh cache delete --all",
):
    assert forbidden not in source, f"unexpected broad/destructive marker: {forbidden}"

print("github actions storage hygiene contract: PASS")
