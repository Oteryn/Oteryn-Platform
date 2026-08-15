#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
from pathlib import Path

SCRIPT = Path(__file__).with_name("branch_hygiene.py")
spec = importlib.util.spec_from_file_location("branch_hygiene", SCRIPT)
assert spec and spec.loader
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
ValidationError = m.ValidationError

MAIN_SHA = "a" * 40
BRANCH_SHA = "b" * 40


def row(branch: str, *, protected: bool = False, sha: str = BRANCH_SHA) -> dict:
    return {"branch": branch, "head_sha": sha, "protected": protected}


def good_settings() -> dict:
    return dict(m.EXPECTED_REPOSITORY_SETTINGS)


def report(
    branch: str,
    *,
    open_prs: list[int] | None = None,
    claims: list[str] | None = None,
    protected: bool = False,
    settings_findings: list[dict] | None = None,
) -> dict:
    return m.evaluate_snapshot(
        branches=[
            row("main", protected=True, sha=MAIN_SHA),
            row(branch, protected=protected),
        ],
        open_prs={branch: list(open_prs or [])},
        active_claims={branch: list(claims or [])},
        settings=good_settings(),
        settings_findings=list(settings_findings or []),
    )


def codes(value: dict) -> set[str]:
    return {item["code"] for item in value["findings"]}


def test_scope_is_hard_bound():
    m.validate_scope("blakinio/Oteryn-Platform", "main")
    m.validate_scope("BLAKINIO/OTERYN-PLATFORM", "main")
    try:
        m.validate_scope("blakinio/Oteryn-v2", "main")
    except ValidationError as exc:
        assert "scoped to" in str(exc)
    else:
        raise AssertionError("foreign repository must fail closed")


def test_valid_active_branch_and_hard_target():
    value = report(
        "repair/issue-1089-steady-state-branch-hygiene",
        open_prs=[1090],
        claims=["docs/agents/tasks/active/task.md"],
    )
    assert value["finding_count"] == 0
    assert value["new_unexplained_branch_count"] == 0
    assert value["hard_target"] == {"NEW_UNEXPLAINED_BRANCHES": 0}
    assert value["raw_branch_count_cap"] is None
    assert value["advisory_count"] == 0


def test_unexplained_remote_branch_fails():
    value = report("feat/issue-9999-orphan")
    assert "UNEXPLAINED_REMOTE_BRANCH" in codes(value)
    assert value["new_unexplained_branches"] == ["feat/issue-9999-orphan"]
    assert value["new_unexplained_branch_count"] == 1


def test_forbidden_active_namespace_fails():
    value = report("backup/issue-9999-copy", open_prs=[9999])
    assert "FORBIDDEN_ACTIVE_NAMESPACE" in codes(value)
    assert value["new_unexplained_branch_count"] == 0
    legacy_shape = report("tmp-do-not-use-again", open_prs=[9998])
    assert "FORBIDDEN_ACTIVE_NAMESPACE" in codes(legacy_shape)


def test_duplicate_branch_ownership_fails():
    value = report(
        "feat/issue-9999-example",
        open_prs=[41, 42],
        claims=["task-a.md", "task-b.md"],
    )
    assert "MULTIPLE_OPEN_PRS" in codes(value)
    assert "MULTIPLE_ACTIVE_CLAIMS" in codes(value)


def test_non_main_protected_branch_fails():
    value = report("release/issue-9999-long-lived", open_prs=[9999], protected=True)
    assert "NON_MAIN_PROTECTED_BRANCH" in codes(value)


def test_naming_is_advisory_and_bots_are_exempt():
    human = report("agent/legacy-worker", open_prs=[10])
    assert human["finding_count"] == 0
    assert human["advisory_count"] == 1
    assert human["advisories"][0]["code"] == "NON_PREFERRED_TASK_BRANCH_NAME"

    bot = report("dependabot/npm_and_yarn/example", open_prs=[11])
    assert bot["finding_count"] == 0
    assert bot["advisory_count"] == 0


def test_repository_setting_drift_fails():
    observed = good_settings()
    assert m.repository_setting_findings(observed) == []
    observed["delete_branch_on_merge"] = False
    findings = m.repository_setting_findings(observed)
    assert len(findings) == 1
    assert findings[0]["code"] == "REPOSITORY_SETTING_DRIFT"
    assert findings[0]["field"] == "delete_branch_on_merge"


def test_repository_settings_graphql_fallback():
    class Client:
        repo = "blakinio/Oteryn-Platform"

        def __init__(self):
            self.calls = []

        def request(self, method, path, data=None):
            self.calls.append((method, path))
            if method == "GET":
                return {"default_branch": "main"}, {}
            assert method == "POST" and path == "/graphql"
            return {
                "data": {
                    "repository": {
                        "defaultBranchRef": {"name": "main"},
                        "mergeCommitAllowed": False,
                        "rebaseMergeAllowed": False,
                        "squashMergeAllowed": True,
                        "deleteBranchOnMerge": True,
                    }
                }
            }, {}

    client = Client()
    observed, findings = m.repository_settings(client)
    assert observed == good_settings()
    assert findings == []
    assert client.calls == [
        ("GET", "/repos/blakinio/Oteryn-Platform"),
        ("POST", "/graphql"),
    ]


def main() -> int:
    tests = [
        value
        for key, value in sorted(globals().items())
        if key.startswith("test_") and callable(value)
    ]
    for test in tests:
        test()
        print("PASS", test.__name__)
    print(f"branch hygiene tests PASS: {len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
