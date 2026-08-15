#!/usr/bin/env python3
from __future__ import annotations

import copy
import datetime as dt
import importlib.util
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).with_name("historical_work_reconciliation.py")
spec = importlib.util.spec_from_file_location("historical_work_reconciliation", SCRIPT)
assert spec and spec.loader
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
ValidationError = m.ValidationError


def fail(fn, needle: str) -> None:
    try: fn()
    except ValidationError as exc:
        assert needle in str(exc), (needle, str(exc)); return
    raise AssertionError(f"expected failure containing {needle}")


def registry(disposition: str = "DELETE") -> dict:
    item = {
        "branch": "history/example", "source_head_sha": "a" * 40, "previous_disposition": "RETAIN",
        "terminal_disposition": disposition, "reason": "TEST_REASON",
        "evidence": ["registry:self"],
        "reviewed_relation": {"main_sha": "b" * 40, "merge_base": "b" * 40, "ahead": 1, "behind": 1, "unique_commit_count": 1, "unique_merge_commit_count": 0, "changed_path_count": 1},
        "source_ref_state": "present_reviewed",
    }
    if disposition == "PR_PROVENANCE_DELETE":
        item["exact_pr"] = {"number": 1, "state": "closed", "merged": True, "head_sha": "a" * 40, "title": "done"}
    if disposition == "MANAGED_RECOVERY":
        item["managed_recovery"] = {"owner": "gov", "purpose": "recovery", "restore_procedure": "restore", "review_trigger": "review", "retention_reason": "incident", "source_object_sha": "a" * 40, "retention_ref": "refs/oteryn-recovery/example", "review_due_at": "2099-01-01T00:00:00Z"}
    return {"schema_version": 1, "issue": 1072, "registry_phase": "reviewed_for_deletion", "reviewed_main_sha": "b" * 40, "reviewed_pr": 1074, "inventory_workflow_run_id": 1, "inventory_artifact_id": 1, "inventory_artifact_digest": "sha256:" + "c" * 64, "workflow_inventory_count": 1, "managed_recovery": ([{"branch": "history/example"}] if disposition == "MANAGED_RECOVERY" else []), "managed_recovery_decision": "TEST", "managed_recovery_reason": "test", "reason_catalog": {"TEST_REASON": "reviewed test disposition"}, "terminal_disposition_counts": {disposition: 1}, "entries": [item]}


def validate(value: dict, *, safe: bool = False, reachable=True) -> None:
    with tempfile.TemporaryDirectory() as td:
        root = Path(td); (root / "docs/agents").mkdir(parents=True); (root / "docs/agents/HISTORICAL_WORK_RECONCILIATION_REGISTRY.json").write_text("{}")
        if safe: value["managed_recovery_workflow_safety_verdict"] = "SAFE"
        m.validate_registry(value, root=root, workflows={"workflow_count": 1}, now=dt.datetime(2026, 8, 15, tzinfo=dt.timezone.utc), reachable=lambda s, r: reachable)


def test_terminal_only():
    for disp in ("DELETE", "DOCUMENT_ARCHIVE", "CANONICALIZE_TO_MAIN", "PR_PROVENANCE_DELETE"): validate(registry(disp))
    bad = registry(); bad["entries"][0]["terminal_disposition"] = "RETAIN"; bad["terminal_disposition_counts"] = {"RETAIN": 1}; fail(lambda: validate(bad), "non-terminal")


def test_duplicate_and_sha_drift():
    bad = registry(); bad["entries"].append(copy.deepcopy(bad["entries"][0])); bad["terminal_disposition_counts"] = {"DELETE": 2}; fail(lambda: validate(bad), "duplicate")
    bad = registry(); bad["entries"][0]["source_head_sha"] = "short"; fail(lambda: validate(bad), "source_head_sha")


def test_pr_provenance_mismatch():
    bad = registry("PR_PROVENANCE_DELETE"); bad["entries"][0]["exact_pr"]["head_sha"] = "d" * 40; fail(lambda: validate(bad), "PR provenance")


def test_managed_recovery_gates():
    value = registry("MANAGED_RECOVERY"); fail(lambda: validate(value), "automation safety")
    validate(registry("MANAGED_RECOVERY"), safe=True)
    fail(lambda: validate(registry("MANAGED_RECOVERY"), safe=True, reachable=False), "not reachable")
    expired = registry("MANAGED_RECOVERY"); expired["entries"][0]["managed_recovery"]["review_due_at"] = "2026-08-14T00:00:00Z"; fail(lambda: validate(expired, safe=True), "due/expired")


def test_candidate_negative_paths():
    item = registry()["entries"][0]
    with tempfile.TemporaryDirectory() as td:
        root = Path(td); (root / "docs/agents").mkdir(parents=True); (root / "docs/agents/HISTORICAL_WORK_RECONCILIATION_REGISTRY.json").write_text("{}")
        row = {"active_claims": [], "pull_history": []}
        ok = {"commit": {"sha": "a" * 40}, "protected": False}
        m.candidate_guard(root, item, row, ok, [])
        drift = {"commit": {"sha": "d" * 40}, "protected": False}; fail(lambda: m.candidate_guard(root, item, row, drift, []), "SHA drift")
        protected = {"commit": {"sha": "a" * 40}, "protected": True}; fail(lambda: m.candidate_guard(root, item, row, protected, []), "protected")
        fail(lambda: m.candidate_guard(root, item, row, ok, [{"number": 1}]), "open PR")
        claimed = {"active_claims": ["task.md"], "pull_history": []}; fail(lambda: m.candidate_guard(root, item, claimed, ok, []), "active claim")
    class Client:
        def __init__(self, sha="a"*40, protected=True): self.sha, self.protected = sha, protected
        def get_branch(self, branch): return {"commit": {"sha": self.sha}, "protected": self.protected}
    fail(lambda: m.assert_main(Client(sha="e"*40), "main", "a"*40), "drifted")
    fail(lambda: m.assert_main(Client(protected=False), "main", "a"*40), "drifted")


def test_unreviewed_and_noncandidate_disappearance():
    assert m.unexplained_unregistered({"protected": False, "open_pr_numbers": [], "active_claims": []})
    assert not m.unexplained_unregistered({"protected": False, "open_pr_numbers": [1], "active_claims": []})
    fail(lambda: m.verify_non_candidates({"main": "a", "live": "b", "candidate": "c"}, {"candidate"}, {"main": "a"}), "non-candidate")
    fail(lambda: m.verify_non_candidates({"main": "a", "live": "b"}, set(), {"main": "a", "live": "z"}), "drifted")


def test_applied_phase_state():
    value = registry(); value["registry_phase"] = "applied"; fail(lambda: validate(value), "source_ref_state")
    value["entries"][0]["source_ref_state"] = "deleted_verified"; validate(value)


def main() -> int:
    tests = [v for k,v in sorted(globals().items()) if k.startswith("test_") and callable(v)]
    for test in tests: test(); print("PASS", test.__name__)
    print(f"historical work reconciliation tests PASS: {len(tests)}"); return 0


if __name__ == "__main__": raise SystemExit(main())
