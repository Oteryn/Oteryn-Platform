#!/usr/bin/env python3

from __future__ import annotations

import json
import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PACKAGE_JSON = ROOT / "scripts/acceptance/package.json"
PACKAGE_LOCK = ROOT / "scripts/acceptance/package-lock.json"
EXACT_SEMVER = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$")


class AcceptanceLockfileContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.package = json.loads(PACKAGE_JSON.read_text(encoding="utf-8"))
        cls.lock = json.loads(PACKAGE_LOCK.read_text(encoding="utf-8"))

    def test_lockfile_v3_root_matches_package_manifest(self) -> None:
        self.assertEqual(3, self.lock.get("lockfileVersion"))
        self.assertTrue(self.lock.get("requires"))
        root = self.lock["packages"][""]
        self.assertEqual(self.package.get("name"), root.get("name"))
        self.assertEqual(self.package.get("version"), root.get("version"))
        self.assertEqual(self.package.get("devDependencies", {}), root.get("devDependencies", {}))

    def test_exact_direct_dev_dependencies_have_matching_locked_package(self) -> None:
        packages = self.lock["packages"]
        for name, requested in self.package.get("devDependencies", {}).items():
            if not EXACT_SEMVER.fullmatch(requested):
                self.fail(f"acceptance devDependency {name} must be exact semver, got {requested}")
            locked = packages.get(f"node_modules/{name}")
            self.assertIsInstance(locked, dict, f"missing lock entry for {name}")
            self.assertEqual(requested, locked.get("version"), f"lock version mismatch for {name}")
            self.assertRegex(locked.get("integrity", ""), r"^sha512-[A-Za-z0-9+/]+={0,2}$")

    def test_every_non_root_lock_entry_has_integrity(self) -> None:
        for path, package in self.lock["packages"].items():
            if path == "":
                continue
            self.assertRegex(
                package.get("integrity", ""),
                r"^sha512-[A-Za-z0-9+/]+={0,2}$",
                f"missing or invalid integrity for {path}",
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
