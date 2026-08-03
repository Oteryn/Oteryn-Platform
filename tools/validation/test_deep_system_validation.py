import json
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
        visual_dir = self.root / "artifacts/deep/visual"
        visual_dir.mkdir(parents=True)
        self.visual = visual_dir / "visual-acceptance-results.json"
        self.visual.write_text(
            json.dumps(
                {
                    "classification": "VISUAL_UX_EVIDENCE_COLLECTED",
                    "validationSha": "abc123",
                    "screenshotCount": 12,
                    "problematic": {
                        "statusMismatch": [],
                        "horizontalOverflow": [],
                        "unlabeledControls": [],
                        "lowContrast": [],
                        "focusNotObserved": [],
                        "rawTechnicalMessages": [],
                        "browserErrors": [],
                    },
                }
            ),
            encoding="utf-8",
        )
        deep_dir = self.root / "artifacts/deep"
        self.soak = deep_dir / "soak-runtime-metrics.json"
        self.soak.write_text(
            json.dumps(
                {
                    "exact_tested_sha": "abc123",
                    "target_duration_seconds": 300,
                    "measured_duration_seconds": 300,
                    "server_rss_start_kb": 100,
                    "server_rss_end_kb": 110,
                    "server_rss_max_kb": 120,
                    "redis_keys_before": 2,
                    "redis_keys_after": 2,
                    "threshold_policy": "calibration-only-no-production-capacity-claim",
                }
            ),
            encoding="utf-8",
        )
        (deep_dir / "soak-rss-samples.tsv").write_text(
            "1\t100\n2\t110\n", encoding="utf-8"
        )

    def tearDown(self):
        self.temp.cleanup()

    def contract(self):
        lanes = []
        junit_lanes = {
            "php-tests",
            "php-game-auth-concurrency",
            "browser-full-chromium",
            "account-lifecycle",
            "community-data",
            "content-scale-contract",
            "downloads",
            "downloads-portability",
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
            "portability-chromium",
            "portability-firefox",
            "portability-webkit",
        ]
        next(x for x in lanes if x["name"] == "responsive")["projects"] = [
            "responsive-desktop",
            "responsive-tablet",
            "responsive-mobile",
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

    def test_clean_contract_with_blocker_is_truthful(self):
        result = validate_contract(self.contract(), "abc123", self.root)
        self.assertEqual(
            result["global_verdict"],
            "DEEP_VALIDATION_PASS_WITH_EXTERNAL_BLOCKERS",
        )
        self.assertEqual(result["external_blocker_count"], 1)
        self.assertEqual(result["external_blockers"][0]["owner_issue"], 490)
        self.assertEqual(result["visual_summary"]["screenshot_count"], 12)
        self.assertEqual(result["soak_metrics"]["measured_duration_seconds"], 300)
        self.assertGreater(result["junit_totals"]["tests"], 0)

    def test_clean_contract_without_blocker_passes(self):
        contract = self.contract()
        contract["lanes"] = [
            lane for lane in contract["lanes"] if lane["name"] != "production-smoke"
        ]
        result = validate_contract(contract, "abc123", self.root)
        self.assertEqual(result["global_verdict"], "DEEP_VALIDATION_PASS")
        self.assertEqual(result["external_blocker_count"], 0)

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
            lane
            for lane in contract["lanes"]
            if lane["name"] != "php-game-auth-concurrency"
        ]
        with self.assertRaisesRegex(ValidationError, "required lanes are missing"):
            validate_contract(contract, "abc123", self.root)

    def test_required_junit_lane_cannot_claim_command_pass(self):
        contract = self.contract()
        lane = next(x for x in contract["lanes"] if x["name"] == "php-tests")
        lane.clear()
        lane.update(
            {
                "name": "php-tests",
                "kind": "command",
                "status": "PASS",
                "required": True,
                "exit_code": 0,
            }
        )
        with self.assertRaisesRegex(ValidationError, "required JUnit lane php-tests"):
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
        lane["projects"] = ["portability-chromium", "portability-firefox"]
        with self.assertRaisesRegex(ValidationError, "portability-webkit"):
            validate_contract(contract, "abc123", self.root)

    def test_missing_responsive_project_fails(self):
        contract = self.contract()
        lane = next(x for x in contract["lanes"] if x["name"] == "responsive")
        lane["projects"] = ["responsive-desktop", "responsive-mobile"]
        with self.assertRaisesRegex(ValidationError, "responsive-tablet"):
            validate_contract(contract, "abc123", self.root)

    def test_blocked_external_lane_requires_owner(self):
        contract = self.contract()
        lane = next(x for x in contract["lanes"] if x["name"] == "production-smoke")
        del lane["owner_issue"]
        with self.assertRaisesRegex(ValidationError, "owner_issue"):
            validate_contract(contract, "abc123", self.root)

    def test_optional_failed_lane_fails_closed(self):
        contract = self.contract()
        lane = next(x for x in contract["lanes"] if x["name"] == "production-smoke")
        lane["status"] = "FAIL"
        with self.assertRaisesRegex(ValidationError, "production-smoke reported FAIL"):
            validate_contract(contract, "abc123", self.root)

    def test_visual_finding_fails(self):
        payload = json.loads(self.visual.read_text(encoding="utf-8"))
        payload["problematic"]["horizontalOverflow"] = ["home/mobile"]
        self.visual.write_text(json.dumps(payload), encoding="utf-8")
        with self.assertRaisesRegex(ValidationError, "horizontalOverflow=1"):
            validate_contract(self.contract(), "abc123", self.root)

    def test_visual_sha_mismatch_fails(self):
        payload = json.loads(self.visual.read_text(encoding="utf-8"))
        payload["validationSha"] = "other"
        self.visual.write_text(json.dumps(payload), encoding="utf-8")
        with self.assertRaisesRegex(ValidationError, "visual evidence SHA"):
            validate_contract(self.contract(), "abc123", self.root)

    def test_short_soak_fails(self):
        payload = json.loads(self.soak.read_text(encoding="utf-8"))
        payload["measured_duration_seconds"] = 120
        self.soak.write_text(json.dumps(payload), encoding="utf-8")
        with self.assertRaisesRegex(ValidationError, "soak duration is incomplete"):
            validate_contract(self.contract(), "abc123", self.root)

    def test_parse_invalid_root_fails(self):
        self.junit.write_text("<root/>", encoding="utf-8")
        with self.assertRaisesRegex(ValidationError, "unsupported JUnit root"):
            parse_junit(self.junit)


if __name__ == "__main__":
    unittest.main()