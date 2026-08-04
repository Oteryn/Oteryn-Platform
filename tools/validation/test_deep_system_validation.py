import json
import tempfile
import unittest
from pathlib import Path

from deep_system_validation import (
    EXPECTED_PROJECTS_BY_LANE,
    REQUIRED_JUNIT_LANES,
    REQUIRED_LANES,
    ValidationError,
    parse_junit,
    validate_contract,
)


class DeepSystemValidationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        visual_dir = self.root / "artifacts/deep/visual"
        visual_dir.mkdir(parents=True)
        self.visual = visual_dir / "visual-acceptance-results.json"
        self.visual.write_text(
            json.dumps(
                {
                    "classification": "VISUAL_UX_EVIDENCE_COLLECTED",
                    "validationSha": "abc123",
                    "screenshotCount": 12,
                    "results": [],
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

    def write_junit(
        self,
        path: Path,
        *,
        projects: set[str] | None = None,
        failures: int = 0,
        errors: int = 0,
        skipped: int = 0,
    ) -> None:
        names = (
            [f"[{project}] testcase" for project in sorted(projects)]
            if projects
            else ["testcase-a", "testcase-b"]
        )
        cases = "".join(f'<testcase name="{name}"/>' for name in names)
        path.write_text(
            f'<testsuite tests="{len(names)}" failures="{failures}" '
            f'errors="{errors}" skipped="{skipped}">{cases}</testsuite>',
            encoding="utf-8",
        )

    def contract(self):
        lanes = []
        for name in sorted(REQUIRED_LANES):
            if name in REQUIRED_JUNIT_LANES:
                path = self.root / f"clean-{name}.xml"
                projects = EXPECTED_PROJECTS_BY_LANE.get(name)
                self.write_junit(path, projects=projects)
                lane = {
                    "name": name,
                    "kind": "junit",
                    "status": "PASS",
                    "required": True,
                    "junit_files": [path.name],
                }
                if projects is not None:
                    lane["projects"] = sorted(projects)
            else:
                lane = {
                    "name": name,
                    "kind": "command",
                    "status": "PASS",
                    "required": True,
                    "exit_code": 0,
                }
            lanes.append(lane)
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

    @staticmethod
    def lane(contract, name):
        return next(item for item in contract["lanes"] if item["name"] == name)

    def visual_payload(self):
        return json.loads(self.visual.read_text(encoding="utf-8"))

    def write_visual_payload(self, payload):
        self.visual.write_text(json.dumps(payload), encoding="utf-8")

    def test_clean_contract_with_blocker_is_truthful(self):
        result = validate_contract(self.contract(), "abc123", self.root)
        self.assertEqual(
            result["global_verdict"],
            "DEEP_VALIDATION_PASS_WITH_EXTERNAL_BLOCKERS",
        )
        self.assertEqual(result["external_blocker_count"], 1)
        self.assertEqual(result["visual_summary"]["screenshot_count"], 12)
        self.assertEqual(
            result["visual_summary"]["expected_navigation_console_error_count"], 0
        )
        self.assertEqual(result["soak_metrics"]["measured_duration_seconds"], 300)
        self.assertEqual(
            set(result["junit_totals"]["projects"]),
            set().union(*EXPECTED_PROJECTS_BY_LANE.values()),
        )

    def test_clean_contract_without_blocker_passes(self):
        contract = self.contract()
        contract["lanes"] = [
            item for item in contract["lanes"] if item["name"] != "production-smoke"
        ]
        result = validate_contract(contract, "abc123", self.root)
        self.assertEqual(result["global_verdict"], "DEEP_VALIDATION_PASS")

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
            item
            for item in contract["lanes"]
            if item["name"] != "php-game-auth-concurrency"
        ]
        with self.assertRaisesRegex(ValidationError, "required lanes are missing"):
            validate_contract(contract, "abc123", self.root)

    def test_required_junit_lane_cannot_claim_command_pass(self):
        contract = self.contract()
        lane = self.lane(contract, "php-tests")
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

    def test_required_flag_must_be_boolean(self):
        contract = self.contract()
        self.lane(contract, "php-tests")["required"] = "true"
        with self.assertRaisesRegex(ValidationError, "required must be a boolean"):
            validate_contract(contract, "abc123", self.root)

    def test_zero_test_junit_fails(self):
        contract = self.contract()
        path = self.root / self.lane(contract, "php-tests")["junit_files"][0]
        path.write_text(
            '<testsuite tests="0" failures="0" errors="0" skipped="0"/>',
            encoding="utf-8",
        )
        with self.assertRaisesRegex(ValidationError, "zero tests"):
            validate_contract(contract, "abc123", self.root)

    def test_claimed_junit_count_must_match_testcases(self):
        contract = self.contract()
        path = self.root / self.lane(contract, "php-tests")["junit_files"][0]
        path.write_text(
            '<testsuite tests="2" failures="0" errors="0" skipped="0">'
            '<testcase name="a"/></testsuite>',
            encoding="utf-8",
        )
        with self.assertRaisesRegex(ValidationError, "declared 2 tests"):
            validate_contract(contract, "abc123", self.root)

    def test_skipped_test_fails(self):
        contract = self.contract()
        path = self.root / self.lane(contract, "php-tests")["junit_files"][0]
        self.write_junit(path, skipped=1)
        with self.assertRaisesRegex(ValidationError, "not clean"):
            validate_contract(contract, "abc123", self.root)

    def test_reused_junit_evidence_fails(self):
        contract = self.contract()
        first = self.lane(contract, "php-tests")
        second = self.lane(contract, "php-game-auth-concurrency")
        second["junit_files"] = list(first["junit_files"])
        with self.assertRaisesRegex(ValidationError, "is reused by lanes"):
            validate_contract(contract, "abc123", self.root)

    def test_junit_path_cannot_escape_evidence_base(self):
        contract = self.contract()
        self.lane(contract, "php-tests")["junit_files"] = ["../outside.xml"]
        with self.assertRaisesRegex(ValidationError, "escapes the evidence base"):
            validate_contract(contract, "abc123", self.root)

    def test_declared_project_contract_must_be_complete(self):
        contract = self.contract()
        lane = self.lane(contract, "portability")
        lane["projects"].remove("portability-webkit")
        with self.assertRaisesRegex(ValidationError, "project contract mismatch"):
            validate_contract(contract, "abc123", self.root)

    def test_junit_must_prove_every_declared_project_executed(self):
        contract = self.contract()
        lane = self.lane(contract, "portability")
        path = self.root / lane["junit_files"][0]
        self.write_junit(
            path,
            projects={"portability-chromium", "portability-firefox"},
        )
        with self.assertRaisesRegex(ValidationError, "JUnit project mismatch"):
            validate_contract(contract, "abc123", self.root)

    def test_junit_cannot_report_unexpected_project(self):
        contract = self.contract()
        lane = self.lane(contract, "responsive")
        path = self.root / lane["junit_files"][0]
        self.write_junit(
            path,
            projects=set(EXPECTED_PROJECTS_BY_LANE["responsive"]) | {"other"},
        )
        with self.assertRaisesRegex(ValidationError, "JUnit project mismatch"):
            validate_contract(contract, "abc123", self.root)

    def test_blocked_external_lane_requires_owner(self):
        contract = self.contract()
        del self.lane(contract, "production-smoke")["owner_issue"]
        with self.assertRaisesRegex(ValidationError, "positive owner_issue"):
            validate_contract(contract, "abc123", self.root)

    def test_external_owner_must_be_positive_integer(self):
        contract = self.contract()
        self.lane(contract, "production-smoke")["owner_issue"] = "490"
        with self.assertRaisesRegex(ValidationError, "positive owner_issue"):
            validate_contract(contract, "abc123", self.root)

    def test_non_external_lane_cannot_be_blocked(self):
        contract = self.contract()
        lane = self.lane(contract, "production-smoke")
        lane.update({"kind": "command", "status": "BLOCKED", "exit_code": 0})
        with self.assertRaisesRegex(ValidationError, "non-external lane"):
            validate_contract(contract, "abc123", self.root)

    def test_external_pass_requires_evidence_identity(self):
        contract = self.contract()
        self.lane(contract, "production-smoke")["status"] = "PASS"
        with self.assertRaisesRegex(ValidationError, "requires evidence_identity"):
            validate_contract(contract, "abc123", self.root)

    def test_optional_failed_lane_fails_closed(self):
        contract = self.contract()
        self.lane(contract, "production-smoke")["status"] = "FAIL"
        with self.assertRaisesRegex(ValidationError, "production-smoke reported FAIL"):
            validate_contract(contract, "abc123", self.root)

    def test_nonclaims_must_be_non_empty_strings(self):
        contract = self.contract()
        contract["nonclaims"] = [""]
        with self.assertRaisesRegex(ValidationError, "non-empty string nonclaims"):
            validate_contract(contract, "abc123", self.root)

    def test_visual_finding_fails(self):
        payload = self.visual_payload()
        payload["problematic"]["horizontalOverflow"] = ["home/mobile"]
        self.write_visual_payload(payload)
        with self.assertRaisesRegex(ValidationError, "horizontalOverflow=1"):
            validate_contract(self.contract(), "abc123", self.root)

    def test_expected_error_page_navigation_console_status_is_retained_not_blocking(self):
        payload = self.visual_payload()
        payload["results"] = [
            {
                "name": "not-found-404",
                "viewport": "desktop",
                "expectedStatus": 404,
                "actualStatus": 404,
                "statusMatches": True,
            }
        ]
        payload["problematic"]["browserErrors"] = [
            {
                "surface": "not-found-404/desktop",
                "consoleErrors": [
                    "Failed to load resource: the server responded with a status of 404 (Not Found)"
                ],
                "pageErrors": [],
            }
        ]
        self.write_visual_payload(payload)
        result = validate_contract(self.contract(), "abc123", self.root)
        self.assertEqual(
            result["visual_summary"]["expected_navigation_console_error_count"], 1
        )
        self.assertEqual(result["visual_summary"]["problem_counts"]["browserErrors"], 0)

    def test_unexpected_browser_console_error_fails(self):
        payload = self.visual_payload()
        payload["results"] = [
            {
                "name": "home",
                "viewport": "desktop",
                "expectedStatus": 200,
                "actualStatus": 200,
                "statusMatches": True,
            }
        ]
        payload["problematic"]["browserErrors"] = [
            {
                "surface": "home/desktop",
                "consoleErrors": ["ReferenceError: portal is not defined"],
                "pageErrors": [],
            }
        ]
        self.write_visual_payload(payload)
        with self.assertRaisesRegex(ValidationError, "browserErrors=1"):
            validate_contract(self.contract(), "abc123", self.root)

    def test_expected_status_error_with_page_error_still_fails(self):
        payload = self.visual_payload()
        payload["results"] = [
            {
                "name": "authorization-denied-403",
                "viewport": "desktop",
                "expectedStatus": 403,
                "actualStatus": 403,
                "statusMatches": True,
            }
        ]
        payload["problematic"]["browserErrors"] = [
            {
                "surface": "authorization-denied-403/desktop",
                "consoleErrors": [
                    "Failed to load resource: the server responded with a status of 403 (Forbidden)"
                ],
                "pageErrors": ["ReferenceError: deniedView is not defined"],
            }
        ]
        self.write_visual_payload(payload)
        with self.assertRaisesRegex(ValidationError, "browserErrors=1"):
            validate_contract(self.contract(), "abc123", self.root)

    def test_visual_sha_mismatch_fails(self):
        payload = self.visual_payload()
        payload["validationSha"] = "other"
        self.write_visual_payload(payload)
        with self.assertRaisesRegex(ValidationError, "visual evidence SHA"):
            validate_contract(self.contract(), "abc123", self.root)

    def test_short_soak_fails(self):
        payload = json.loads(self.soak.read_text(encoding="utf-8"))
        payload["measured_duration_seconds"] = 299
        self.soak.write_text(json.dumps(payload), encoding="utf-8")
        with self.assertRaisesRegex(ValidationError, "soak duration is incomplete"):
            validate_contract(self.contract(), "abc123", self.root)

    def test_parse_invalid_root_fails(self):
        path = self.root / "invalid.xml"
        path.write_text("<root/>", encoding="utf-8")
        with self.assertRaisesRegex(ValidationError, "unsupported JUnit root"):
            parse_junit(path)


if __name__ == "__main__":
    unittest.main()
