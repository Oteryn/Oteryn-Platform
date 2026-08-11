from __future__ import annotations

import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class IdentityTemplateTests(unittest.TestCase):
    def test_template_contains_no_preapproved_identity_values(self) -> None:
        document = json.loads((ROOT / "identity.template.json").read_text(encoding="utf-8"))
        for field in (
            "client_version",
            "package_sha256",
            "executable_sha256",
            "elf_build_id",
            "package_source",
        ):
            self.assertTrue(str(document[field]).startswith("REPLACE_WITH_"), field)


if __name__ == "__main__":
    unittest.main()
