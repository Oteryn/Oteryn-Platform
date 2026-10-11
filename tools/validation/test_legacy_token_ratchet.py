#!/usr/bin/env python3

from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

import legacy_token_ratchet as ratchet

TOKEN = "c" + "anary"


def run(root: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True)


class RatchetTest(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        run(self.root, "init", "-q")
        (self.root / "docs/agents").mkdir(parents=True)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def add(self, rel: str, text: str) -> None:
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        run(self.root, "add", rel)

    def main(self, *args: str) -> int:
        return ratchet.main(["--root", str(self.root), *args])

    def baseline(self) -> dict:
        return json.loads(
            (self.root / ratchet.DEFAULT_BASELINE).read_text(encoding="utf-8")
        )

    def seed(self) -> None:
        self.add("app/A.php", f"one {TOKEN.upper()}\nclean\ntwo {TOKEN}\n")
        self.add("docs/note.md", f"{TOKEN}\n")
        self.add(f"docs/agents/tasks/archive/{TOKEN}-old.md", "history\n")
        self.add("tests/t.php", "clean\n")
        self.assertEqual(self.main("--write"), 0)

    def test_task_record_branch_lines_are_exempt(self) -> None:
        def record(branch: str, notes: str) -> str:
            return (
                "---\ntask_id: T\n---\n\n## Context checkpoint\n\n```yaml\n"
                f"head: UNKNOWN\nbranch: {branch}\nstatus: validating\n```\n\n"
                f"## Notes\n{notes}"
            )

        self.add("docs/agents/tasks/active/T.md", record(f"claude/{TOKEN}-x-1", "clean\n"))
        self.add("docs/agents/tasks/active/U.md", record(f"claude/{TOKEN}-y", f"branch: claude/{TOKEN}-y\n"))
        self.add("docs/agents/tasks/active/V.md", record(f"{TOKEN} is prose", f"source_branch_reason: {TOKEN}\n"))
        self.add("docs/agents/tasks/active/W.md", f"branch: claude/{TOKEN}-w\n")
        self.add("docs/other.md", record(f"claude/{TOKEN}-z", ""))
        self.assertEqual(self.main("--write"), 0)
        docs = self.baseline()["groups"]["docs"]
        # U notes line, V branch prose + notes line, W (no checkpoint block) and
        # docs/other.md (not a task record) count; the T and U checkpoint branch lines do not.
        self.assertEqual(docs["count"], 5)
        self.assertEqual(docs["files"], 4)

    def test_pattern_does_not_match_itself(self) -> None:
        source = Path(ratchet.__file__).read_text(encoding="utf-8")
        self.assertIsNone(ratchet.TOKEN_RE.search(source))

    def test_baseline_counts_lines_per_group_and_paths(self) -> None:
        self.seed()
        groups = self.baseline()["groups"]
        self.assertEqual(groups["runtime"]["count"], 2)
        self.assertEqual(groups["docs"]["count"], 1)
        self.assertEqual(groups["tests"]["count"], 0)
        self.assertEqual(groups["paths"]["count"], 0)
        self.assertEqual(groups["history_paths"]["count"], 1)
        self.assertEqual(groups["history"]["count"], 0)

    def test_unchanged_passes(self) -> None:
        self.seed()
        self.assertEqual(self.main(), 0)

    def test_rise_in_content_fails(self) -> None:
        self.seed()
        self.add("tests/t.php", f"{TOKEN}\n")
        self.assertEqual(self.main(), 1)

    def test_rise_in_path_names_fails(self) -> None:
        self.seed()
        self.add(f"docs/{TOKEN}-guide.txt", "clean\n")
        self.assertEqual(self.main(), 1)

    def test_fall_passes_and_write_lowers_baseline(self) -> None:
        self.seed()
        self.add("app/A.php", "clean\n")
        self.assertEqual(self.main(), 0)
        self.assertEqual(self.main("--write"), 0)
        self.assertEqual(self.baseline()["groups"]["runtime"]["count"], 0)

    def test_write_refuses_to_raise(self) -> None:
        self.seed()
        self.add("tests/t.php", f"{TOKEN}\n")
        self.assertEqual(self.main("--write"), 1)
        self.assertEqual(self.baseline()["groups"]["tests"]["count"], 0)

    def test_report_only_group_does_not_fail(self) -> None:
        self.seed()
        data = self.baseline()
        data["groups"]["docs"]["enforce"] = False
        (self.root / ratchet.DEFAULT_BASELINE).write_text(json.dumps(data))
        self.add("docs/note.md", f"{TOKEN}\n{TOKEN}\n")
        self.assertEqual(self.main(), 0)

    def test_write_keeps_notes_and_enforce_flags(self) -> None:
        self.seed()
        data = self.baseline()
        data["notes"] = ["keep me"]
        data["groups"]["docs"]["enforce"] = False
        (self.root / ratchet.DEFAULT_BASELINE).write_text(json.dumps(data))
        self.assertEqual(self.main("--write"), 0)
        rewritten = self.baseline()
        self.assertEqual(rewritten["notes"], ["keep me"])
        self.assertFalse(rewritten["groups"]["docs"]["enforce"])

    def test_missing_baseline_is_error(self) -> None:
        self.assertEqual(self.main(), 2)

    def test_group_of_first_match_wins(self) -> None:
        rules = ratchet.DEFAULT_RULES
        self.assertEqual(ratchet.group_of("tests/ci/x.py", rules), "ci_deploy")
        self.assertEqual(ratchet.group_of("tests/Feature/x.php", rules), "tests")
        self.assertEqual(ratchet.group_of("docs/agents/reports/x.md", rules), "history")
        self.assertEqual(ratchet.group_of("composer.json", rules), "other")


if __name__ == "__main__":
    unittest.main()

    def test_history_prefixes_are_excluded_from_paths_and_enforced_groups(self) -> None:
        self.add("docs/agents/tasks/archive/OLD.md", f"{TOKEN}\n")
        self.add(f"docs/agents/tasks/archive/{TOKEN}-old.md", "clean\n")
        self.add(f"docs/evidence/{TOKEN}-run.md", f"{TOKEN}\n")
        self.add("docs/live.md", "clean\n")
        self.assertEqual(self.main("--write"), 0)
        data = self.baseline()
        self.assertEqual(data["exclusions"], ratchet.DEFAULT_EXCLUSIONS)
        self.assertEqual(data["groups"]["paths"]["count"], 0)
        self.assertEqual(data["groups"]["history_paths"]["count"], 2)
        self.assertEqual(data["groups"]["history"]["count"], 2)
        self.assertEqual(data["groups"]["docs"]["count"], 0)
        self.assertFalse(data["groups"]["history"]["enforce"])
        self.assertFalse(data["groups"]["history_paths"]["enforce"])
        self.assertEqual(self.main(), 0)

    def test_new_excluded_history_path_does_not_fail_but_live_path_does(self) -> None:
        self.add("docs/live.md", "clean\n")
        self.assertEqual(self.main("--write"), 0)
        self.add(f"docs/agents/reports/{TOKEN}.md", f"{TOKEN}\n")
        self.assertEqual(self.main(), 0)
        self.add(f"docs/{TOKEN}-live.md", "clean\n")
        self.assertEqual(self.main(), 1)

    def test_exclusion_applies_only_to_listed_prefixes(self) -> None:
        self.assertTrue(ratchet.is_excluded("CHANGELOG.md", ["CHANGELOG.md"]))
        self.assertFalse(ratchet.is_excluded("CHANGELOG.md.bak", ["CHANGELOG.md"]))
        self.assertFalse(ratchet.is_excluded("docs/agents/tasks/active/T.md", ratchet.DEFAULT_EXCLUSIONS))
        self.assertTrue(ratchet.is_excluded("docs/agents/tasks/archive/T.md", ratchet.DEFAULT_EXCLUSIONS))
