#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("adr_registry.py")
SPEC = importlib.util.spec_from_file_location("adr_registry", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
adr_registry = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(adr_registry)


class AdrRegistryValidatorTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        self.adr_dir = self.root / "docs/architecture/adr"
        self.adr_dir.mkdir(parents=True)

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def write_adr(
        self,
        filename: str,
        *,
        status: str = "Accepted",
        extra: str = "",
    ) -> None:
        content = (
            f"# ADR {filename[:4]}: Fixture\n\n"
            f"- Status: {status}\n"
            "- Date: 2026-08-05\n"
        )
        if extra:
            content += f"{extra.rstrip()}\n"
        self.adr_dir.joinpath(filename).write_text(content, encoding="utf-8")

    def write_inventory(self, filenames: list[str]) -> None:
        lines = ["# Architecture Decision Records", "", "## Inventory", ""]
        lines.extend(f"- `{filename}`" for filename in filenames)
        lines.extend(["", "## Known registry debt", "", "Fixture."])
        self.adr_dir.joinpath("README.md").write_text(
            "\n".join(lines) + "\n", encoding="utf-8"
        )

    def assert_error_contains(self, errors: list[str], text: str) -> None:
        self.assertTrue(
            any(text in error for error in errors),
            f"expected error containing {text!r}, got {errors!r}",
        )

    def test_accepts_exact_legacy_duplicate_allowlist(self) -> None:
        files = ["0001-alpha.md", "0001-beta.md", "0002-gamma.md"]
        for filename in files:
            self.write_adr(filename)
        self.write_inventory(files)

        errors = adr_registry.validate_repository(
            self.root,
            legacy_duplicates={"0001": ("0001-alpha.md", "0001-beta.md")},
        )

        self.assertEqual([], errors)

    def test_rejects_new_duplicate_prefix(self) -> None:
        files = ["0001-alpha.md", "0001-beta.md"]
        for filename in files:
            self.write_adr(filename)
        self.write_inventory(files)

        errors = adr_registry.validate_repository(
            self.root, legacy_duplicates={}
        )

        self.assert_error_contains(errors, "unapproved duplicate ADR prefix 0001")

    def test_rejects_legacy_allowlist_drift(self) -> None:
        files = ["0001-alpha.md", "0001-charlie.md"]
        for filename in files:
            self.write_adr(filename)
        self.write_inventory(files)

        errors = adr_registry.validate_repository(
            self.root,
            legacy_duplicates={"0001": ("0001-alpha.md", "0001-beta.md")},
        )

        self.assert_error_contains(errors, "legacy duplicate allowlist drift for 0001")

    def test_rejects_missing_lifecycle(self) -> None:
        filename = "0001-alpha.md"
        self.adr_dir.joinpath(filename).write_text(
            "# ADR 0001: Fixture\n", encoding="utf-8"
        )
        self.write_inventory([filename])

        errors = adr_registry.validate_repository(
            self.root, legacy_duplicates={}
        )

        self.assert_error_contains(errors, "expected exactly one lifecycle line")

    def test_rejects_inventory_drift(self) -> None:
        self.write_adr("0001-alpha.md")
        self.write_adr("0002-beta.md")
        self.write_inventory(["0001-alpha.md"])

        errors = adr_registry.validate_repository(
            self.root, legacy_duplicates={}
        )

        self.assert_error_contains(errors, "README inventory is missing ADR files")

    def test_rejects_broken_supersession_target(self) -> None:
        filename = "0001-alpha.md"
        self.write_adr(
            filename,
            status="Superseded",
            extra="- Superseded by: `0002-missing.md`",
        )
        self.write_inventory([filename])

        errors = adr_registry.validate_repository(
            self.root, legacy_duplicates={}
        )

        self.assert_error_contains(errors, "supersession target does not exist")

    def test_accepts_resolved_supersession_target(self) -> None:
        self.write_adr(
            "0001-alpha.md",
            status="Superseded",
            extra="- Superseded by: `docs/architecture/adr/0002-beta.md`",
        )
        self.write_adr("0002-beta.md")
        self.write_inventory(["0001-alpha.md", "0002-beta.md"])

        errors = adr_registry.validate_repository(
            self.root, legacy_duplicates={}
        )

        self.assertEqual([], errors)

    def test_rejects_invalid_filename(self) -> None:
        filename = "ADR-1-invalid.md"
        self.write_adr(filename)
        self.write_inventory([filename])

        errors = adr_registry.validate_repository(
            self.root, legacy_duplicates={}
        )

        self.assert_error_contains(errors, "invalid ADR filename")


if __name__ == "__main__":
    unittest.main()
