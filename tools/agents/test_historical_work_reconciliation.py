#!/usr/bin/env python3
from __future__ import annotations

import copy
import datetime as dt
import importlib.util
import subprocess
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).with_name("historical_work_reconciliation.py")
spec = importlib.util.spec_from_file_location("historical_work_reconciliation", SCRIPT)
assert spec and spec.loader
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
ValidationError = m.ValidationError


def fail(fn, needle: str) -> None:
    try:
        fn()
    except ValidationError as exc:
        assert needle in str(exc), (needle, str(exc))
        return
    raise AssertionError(f"expected failure containing {needle}")


def blob_binding() -> dict[str, str]:
    return {
        ".github/workflows/historical-branch-audit.yml": "1" * 40,
        "tools/agents/historical_work_reconciliation.py": "2" * 40,
    }


def registry(disposition: str = "DELETE") -> dict:
    item = {
        "branch": "history/example",
        "source_head_sha": "a" * 40,
        "terminal_disposition": disposition,
        "reason": "TEST_REASON",
        "evidence": ["registry:self"],
        "source_ref_state": "present_reviewed",
    }
    if disposition == "PR_PROVENANCE_DELETE":
        item["exact_pr"] = {"number": 1, "state": "closed", "head_sha": "a" * 40}
    if disposition == "MANAGED_RECOVERY":
        item["managed_recovery"] = {
            "owner": "gov",
            "purpose": "recovery",
            "restore_procedure": "restore",
            "review_trigger": "review",
            "retention_reason": "incident",
            "source_object_sha": "a" * 40,
            "retention_ref": "refs/oteryn-recovery/example",
            "review_due_at": "2099-01-01T00:00:00Z",
        }
    return {
        "schema_version": 1,
        "issue": 1072,
        "registry_phase": "reviewed_for_deletion",
        "reviewed_main_sha": "b" * 40,
        "reviewed_pr": 1074,
        "workflow_inventory_count": 1,
        "managed_recovery": ([{"branch": "history/example"}] if disposition == "MANAGED_RECOVERY" else []),
        "managed_recovery_reason": "test",
        "reason_catalog": {"TEST_REASON": "reviewed test disposition"},
        "terminal_disposition_counts": {disposition: 1},
        "reviewed_apply_implementation_blobs": blob_binding(),
        "entries": [item],
    }


def validate(value: dict, *, safe: bool = False, reachable=True) -> None:
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        (root / "docs/agents").mkdir(parents=True)
        (root / "docs/agents/HISTORICAL_WORK_RECONCILIATION_REGISTRY.json").write_text("{}")
        if safe:
            value["managed_recovery_workflow_safety_verdict"] = "SAFE"
        m.validate_registry(
            value,
            root=root,
            workflows={"workflow_count": 1},
            now=dt.datetime(2026, 8, 15, tzinfo=dt.timezone.utc),
            reachable=lambda source, ref: reachable,
            blob_resolver=lambda path: value["reviewed_apply_implementation_blobs"][path.as_posix()],
        )


def test_repository_scope_is_hard_bound():
    m.validate_repository_scope("Oteryn/Oteryn-Platform", "main")
    m.validate_repository_scope("OTERYN/OTERYN-PLATFORM", "main")
    fail(lambda: m.validate_repository_scope("blakinio/Oteryn-v2", "main"), "may mutate only")
    fail(lambda: m.validate_repository_scope("Oteryn/Oteryn-Platform", "release"), "protected main")


def test_reviewed_implementation_binding():
    value = registry()
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        m.validate_reviewed_implementation(
            root,
            value,
            blob_resolver=lambda path: value["reviewed_apply_implementation_blobs"][path.as_posix()],
        )
        fail(
            lambda: m.validate_reviewed_implementation(
                root,
                value,
                blob_resolver=lambda path: "f" * 40,
            ),
            "reviewed implementation drift",
        )
        bad = copy.deepcopy(value)
        bad["reviewed_apply_implementation_blobs"].pop(
            ".github/workflows/historical-branch-audit.yml"
        )
        fail(
            lambda: m.validate_reviewed_implementation(
                root,
                bad,
                blob_resolver=lambda path: "f" * 40,
            ),
            "cover exactly",
        )


def test_terminal_only():
    for disposition in (
        "DELETE",
        "DOCUMENT_ARCHIVE",
        "CANONICALIZE_TO_MAIN",
        "PR_PROVENANCE_DELETE",
    ):
        validate(registry(disposition))
    bad = registry()
    bad["entries"][0]["terminal_disposition"] = "RETAIN"
    bad["terminal_disposition_counts"] = {"RETAIN": 1}
    fail(lambda: validate(bad), "non-terminal")


def test_duplicate_and_sha_drift():
    bad = registry()
    bad["entries"].append(copy.deepcopy(bad["entries"][0]))
    bad["terminal_disposition_counts"] = {"DELETE": 2}
    fail(lambda: validate(bad), "duplicate")
    bad = registry()
    bad["entries"][0]["source_head_sha"] = "short"
    fail(lambda: validate(bad), "source_head_sha")


def test_pr_provenance_mismatch():
    bad = registry("PR_PROVENANCE_DELETE")
    bad["entries"][0]["exact_pr"]["head_sha"] = "d" * 40
    fail(lambda: validate(bad), "PR provenance")


def test_managed_recovery_gates():
    value = registry("MANAGED_RECOVERY")
    fail(lambda: validate(value), "automation safety")
    validate(registry("MANAGED_RECOVERY"), safe=True)
    fail(
        lambda: validate(registry("MANAGED_RECOVERY"), safe=True, reachable=False),
        "not reachable",
    )
    expired = registry("MANAGED_RECOVERY")
    expired["entries"][0]["managed_recovery"]["review_due_at"] = "2026-08-14T00:00:00Z"
    fail(lambda: validate(expired, safe=True), "due/expired")


def test_candidate_negative_paths():
    item = registry()["entries"][0]
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        (root / "docs/agents").mkdir(parents=True)
        (root / "docs/agents/HISTORICAL_WORK_RECONCILIATION_REGISTRY.json").write_text("{}")
        row = {"active_claims": [], "pull_history": []}
        ok = {"commit": {"sha": "a" * 40}, "protected": False}
        m.candidate_guard(root, item, row, ok, [])
        drift = {"commit": {"sha": "d" * 40}, "protected": False}
        fail(lambda: m.candidate_guard(root, item, row, drift, []), "SHA drift")
        protected = {"commit": {"sha": "a" * 40}, "protected": True}
        fail(lambda: m.candidate_guard(root, item, row, protected, []), "protected")
        fail(lambda: m.candidate_guard(root, item, row, ok, [{"number": 1}]), "open PR")
        claimed = {"active_claims": ["task.md"], "pull_history": []}
        fail(lambda: m.candidate_guard(root, item, claimed, ok, []), "active claim")

    class Client:
        def __init__(self, sha="a" * 40, protected=True):
            self.sha = sha
            self.protected = protected

        def get_branch(self, branch):
            return {"commit": {"sha": self.sha}, "protected": self.protected}

    fail(lambda: m.assert_main(Client(sha="e" * 40), "main", "a" * 40), "drifted")
    fail(lambda: m.assert_main(Client(protected=False), "main", "a" * 40), "drifted")


def test_atomic_delete_uses_exact_leases():
    entries = [
        {"branch": "history/a", "source_head_sha": "a" * 40},
        {"branch": "history/b", "source_head_sha": "b" * 40},
    ]
    seen: list[list[str]] = []
    original = m.audit.run_git

    class Client:
        def _validated_git_remote(self):
            return "origin"

    def fake_run_git(root, args, **kwargs):
        seen.append(list(args))
        return subprocess.CompletedProcess(["git", *args], 0, "ok", "")

    try:
        m.audit.run_git = fake_run_git
        deleted = m.atomic_delete_reviewed_refs(Client(), Path("."), entries)
    finally:
        m.audit.run_git = original
    assert deleted == [
        {"branch": "history/a", "head_sha": "a" * 40},
        {"branch": "history/b", "head_sha": "b" * 40},
    ]
    command = seen[0]
    assert "--atomic" in command
    assert f"--force-with-lease=refs/heads/history/a:{'a' * 40}" in command
    assert f"--force-with-lease=refs/heads/history/b:{'b' * 40}" in command
    assert ":refs/heads/history/a" in command
    assert ":refs/heads/history/b" in command


def test_deleted_ref_restore_is_exact():
    live: dict[str, str] = {}
    original = m.remote_ref_sha

    class Client:
        def create_branch(self, branch, sha):
            live[branch] = sha

    try:
        m.remote_ref_sha = lambda root, branch: live.get(branch)
        m.restore_deleted_refs(
            Client(),
            Path("."),
            [
                {"branch": "history/a", "head_sha": "a" * 40},
                {"branch": "history/b", "head_sha": "b" * 40},
            ],
        )
    finally:
        m.remote_ref_sha = original
    assert live == {"history/a": "a" * 40, "history/b": "b" * 40}


def test_unreviewed_and_noncandidate_disappearance():
    assert m.unexplained_unregistered(
        {"protected": False, "open_pr_numbers": [], "active_claims": []}
    )
    assert not m.unexplained_unregistered(
        {"protected": False, "open_pr_numbers": [1], "active_claims": []}
    )
    fail(
        lambda: m.verify_non_candidates(
            {"main": "a", "live": "b", "candidate": "c"},
            {"candidate"},
            {"main": "a"},
        ),
        "non-candidate",
    )
    fail(
        lambda: m.verify_non_candidates(
            {"main": "a", "live": "b"}, set(), {"main": "a", "live": "z"}
        ),
        "drifted",
    )


def test_applied_phase_state():
    value = registry()
    value["registry_phase"] = "applied"
    fail(lambda: validate(value), "source_ref_state")
    value["entries"][0]["source_ref_state"] = "deleted_verified"
    validate(value)


def main() -> int:
    tests = [
        value
        for key, value in sorted(globals().items())
        if key.startswith("test_") and callable(value)
    ]
    for test in tests:
        test()
        print("PASS", test.__name__)
    print(f"historical work reconciliation tests PASS: {len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
