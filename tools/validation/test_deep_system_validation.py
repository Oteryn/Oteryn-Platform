import tempfile
import unittest
from pathlib import Path

from deep_system_validation import (
    REQUIRED_LANES,
    ValidationError,
    parse_junit,
    validate_contract,
)


class DeepSystemValidationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.junit = self.root / "clean.xml"
        self.junit.write_text(
            '<testsuite tests="2" failures="0" errors="0" skipped="0">'
            '<testcase name="a"/><testcase name="b"/></testsuite>',
            encoding="utf-8",
        )

    def tearDown(self):
        self.temp.cleanup()

    def contract(self):
        lanes = []
        junit_lanes = {
            "browser-full-chromium",
            "account-lifecycle",
            "community-data",
            "downloads",
            "portability",
            "responsive",
            "resilience",
            "accessibility",
            "soak",
        }
        for name in sorted(REQUIRED_LANES):
            if name in junit_lanes:
                lane = {
                    "name": name,
                    "kind": "junit",
                    "status": "PASS",
                    "required": True,
                    "junit_files": ["clean.xml"],
                }
            else:
                lane = {
                    "name": name,
                    "kind": "command",
                    "status": "PASS",
                    "required": True,
                    "exit_code": 0,
                }
            lanes.append(lane)
        next(x for x in lanes if x["name"] == "portability")["projects"] = [
            "chromium-primary",
            "firefox-portability",
            "webkit-portability",
        ]
        next(x for x in lanes if x["name"] == "responsive")["viewports"] = [
            "desktop",
            "tablet",
            "mobile",
        ]
        lanes.append(
            {
                "name": "production-smoke",
                "kind": "external",
                "status": "BLOCKED",
                "required": False,
                "reason": "No authorized production target or credentials.",
                "owner_issue": 490,
            }
        )
        return {
            "schema_version": 1,
            "exact_sha": "abc123",
            "retries": 0,
            "lanes": lanes,
            "nonclaims": ["Repository execution is not production proof."],
        }

    def test_clean_contract_passes(self):
        result = validate_contract(self.contract(), "abc123", self.root)
        self.assertEqual(result["global_verdict"], "DEEP_VALIDATION_PASS")
        self.assertGreater(result["junit_totals"]["tests"], 0)

    def test_wrong_sha_fails(self):
        with self.assertRaisesRegex(ValidationError, "does not match"):
            validate_contract(self.contract(), "other", self.root)

    def test_retries_fail(self):
        contract = self.contract()
        contract["retries"] = 1
        with self.assertRaisesRegex(ValidationError, "retries"):
            validate_contract(contract, "abc123", self.root)

    def test_missing_required_lane_fails(self):
        contract = self.contract()
        contract["lanes"] = [
            lane for lane in contract["lanes"] if lane["name"] != "soak"
        ]
        with self.assertRaisesRegex(ValidationError, "required lanes are missing"):
            validate_contract(contract, "abc123", self.root)

    def test_zero_test_junit_fails(self):
        self.junit.write_text(
            '<testsuite tests="0" failures="0" errors="0" skipped="0"/>',
            encoding="utf-8",
        )
        with self.assertRaisesRegex(ValidationError, "zero tests"):
            validate_contract(self.contract(), "abc123", self.root)

    def test_skipped_test_fails(self):
        self.junit.write_text(
            '<testsuite tests="2" failures="0" errors="0" skipped="1"/>',
            encoding="utf-8",
        )
        with self.assertRaisesRegex(ValidationError, "not clean"):
            validate_contract(self.contract(), "abc123", self.root)

    def test_missing_browser_project_fails(self):
        contract = self.contract()
        lane = next(x for x in contract["lanes"] if x["name"] == "portability")
        lane["projects"] = ["chromium-primary", "firefox-portability"]
        with self.assertRaisesRegex(ValidationError, "webkit-portability"):
            validate_contract(contract, "abc123", self.root)

    def test_blocked_external_lane_requires_owner(self):
        contract = self.contract()
        lane = next(x for x in contract["lanes"] if x["name"] == "production-smoke")
        del lane["owner_issue"]
        with self.assertRaisesRegex(ValidationError, "owner_issue"):
            validate_contract(contract, "abc123", self.root)

    def test_parse_invalid_root_fails(self):
        self.junit.write_text("<root/>", encoding="utf-8")
        with self.assertRaisesRegex(ValidationError, "unsupported JUnit root"):
            parse_junit(self.junit)


if __name__ == "__main__":
    unittest.main()
