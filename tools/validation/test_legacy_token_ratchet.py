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

    def test_pattern_does_not_match_itself(self) -> None:
        source = Path(ratchet.__file__).read_text(encoding="utf-8")
        self.assertIsNone(ratchet.TOKEN_RE.search(source))

    def test_baseline_counts_lines_per_group_and_paths(self) -> None:
        self.seed()
        groups = self.baseline()["groups"]
        self.assertEqual(groups["runtime"]["count"], 2)
        self.assertEqual(groups["docs"]["count"], 1)
        self.assertEqual(groups["tests"]["count"], 0)
        self.assertEqual(groups["paths"]["count"], 1)
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
