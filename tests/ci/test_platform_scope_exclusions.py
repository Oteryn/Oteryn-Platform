#!/usr/bin/env python3

from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]


class PlatformScopeExclusionTest(unittest.TestCase):
    def test_out_of_scope_operational_assets_are_absent(self) -> None:
        forbidden = (
            ROOT / ".github/workflows/liquid20-synology-control.yml",
            ROOT / "deploy/liquid20",
        )
        existing = [path.relative_to(ROOT).as_posix() for path in forbidden if path.exists()]
        self.assertEqual(existing, [], f"out-of-scope operational assets present: {existing}")


if __name__ == "__main__":
    unittest.main(verbosity=2)
