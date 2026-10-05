#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts/ci/container_supply_chain.py"

spec = importlib.util.spec_from_file_location("container_supply_chain", MODULE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"cannot load {MODULE_PATH}")
supply = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = supply
spec.loader.exec_module(supply)


class ContainerSupplyChainTest(unittest.TestCase):
    def test_repository_policy_accepts_current_pinned_dockerfiles(self) -> None:
        result = supply.validate_repository()
        self.assertEqual(1, result["schema_version"])
        self.assertEqual(
            {"platform", "game-gateway", "deploy-runner"},
            {item["component"] for item in result["dockerfiles"]},
        )

    def test_mutable_external_base_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            (root / "Dockerfile").write_text(
                "FROM php:8.5-cli-alpine\nUSER www-data\n", encoding="utf-8"
            )
            (root / "exceptions.json").write_text(
                json.dumps({"schema_version": 1, "exceptions": []}), encoding="utf-8"
            )
            policy = root / "policy.json"
            policy.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "dockerfiles": [
                            {
                                "component": "platform",
                                "path": "Dockerfile",
                                "expected_user": "www-data",
                            }
                        ],
                        "vulnerability_policy": {
                            "severities": ["HIGH", "CRITICAL"],
                            "ignore_unfixed": True,
                            "exception_file": "exceptions.json",
                            "maximum_exception_days": 30,
                        },
                    }
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(supply.SupplyChainError, "mutable external image"):
                supply.validate_repository(policy, root)

    def test_non_root_runtime_identity_is_enforced(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            (root / "Dockerfile").write_text(
                "FROM alpine:3.22@sha256:" + "a" * 64 + "\nUSER root\n",
                encoding="utf-8",
            )
            (root / "exceptions.json").write_text(
                json.dumps({"schema_version": 1, "exceptions": []}), encoding="utf-8"
            )
            policy = root / "policy.json"
            policy.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "dockerfiles": [
                            {
                                "component": "gateway",
                                "path": "Dockerfile",
                                "expected_user": "oteryn",
                            }
                        ],
                        "vulnerability_policy": {
                            "severities": ["HIGH", "CRITICAL"],
                            "ignore_unfixed": True,
                            "exception_file": "exceptions.json",
                            "maximum_exception_days": 30,
                        },
                    }
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(supply.SupplyChainError, "final USER"):
                supply.validate_repository(policy, root)

    def write_scan_fixture(
        self,
        root: Path,
        *,
        fixed: str = "1.2.4",
        severity: str = "HIGH",
        vuln_id: str = "CVE-TEST-1",
        package: str = "demo",
    ) -> Path:
        report = root / "trivy.json"
        report.write_text(
            json.dumps(
                {
                    "Results": [
                        {
                            "Vulnerabilities": [
                                {
                                    "VulnerabilityID": vuln_id,
                                    "PkgName": package,
                                    "Severity": severity,
                                    "FixedVersion": fixed,
                                }
                            ]
                        }
                    ]
                }
            ),
            encoding="utf-8",
        )
        return report

    def write_scan_policy(self, root: Path, exceptions: list[dict[str, str]]) -> Path:
        (root / "exceptions.json").write_text(
            json.dumps({"schema_version": 1, "exceptions": exceptions}), encoding="utf-8"
        )
        policy = root / "policy.json"
        policy.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "dockerfiles": [],
                    "vulnerability_policy": {
                        "severities": ["HIGH", "CRITICAL"],
                        "ignore_unfixed": True,
                        "exception_file": "exceptions.json",
                        "maximum_exception_days": 30,
                    },
                }
            ),
            encoding="utf-8",
        )
        return policy

    def test_patchable_high_finding_blocks_without_exception(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            report = self.write_scan_fixture(root)
            policy = self.write_scan_policy(root, [])
            with self.assertRaisesRegex(supply.SupplyChainError, "blocked findings"):
                supply.evaluate_trivy(report, policy, "platform", root)

    def test_unfixed_high_is_ignored_by_explicit_policy(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            report = self.write_scan_fixture(root, fixed="")
            policy = self.write_scan_policy(root, [])
            result = supply.evaluate_trivy(report, policy, "platform", root)
            self.assertEqual("PASS", result["status"])

    def test_bounded_exception_allows_exact_finding(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            report = self.write_scan_fixture(root)
            policy = self.write_scan_policy(
                root,
                [
                    {
                        "component": "platform",
                        "vulnerability_id": "CVE-TEST-1",
                        "package_name": "demo",
                        "created_on": "2026-10-01",
                        "expires_on": "2026-10-20",
                        "evidence": "https://github.com/Oteryn/Oteryn-Platform/issues/1011",
                    }
                ],
            )
            result = supply.evaluate_trivy(
                report, policy, "platform", root, today=date(2026, 10, 5)
            )
            self.assertEqual(1, len(result["excepted"]))

    def test_expired_exception_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            report = self.write_scan_fixture(root)
            policy = self.write_scan_policy(
                root,
                [
                    {
                        "component": "platform",
                        "vulnerability_id": "CVE-TEST-1",
                        "package_name": "demo",
                        "created_on": "2026-09-01",
                        "expires_on": "2026-09-20",
                        "evidence": "https://github.com/Oteryn/Oteryn-Platform/issues/1011",
                    }
                ],
            )
            with self.assertRaisesRegex(supply.SupplyChainError, "expired"):
                supply.evaluate_trivy(
                    report, policy, "platform", root, today=date(2026, 10, 5)
                )

    def test_provenance_is_deterministic_and_binds_all_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            dockerfile = root / "Dockerfile"
            dockerignore = root / "Dockerfile.dockerignore"
            sbom = root / "sbom.json"
            trivy = root / "trivy.json"
            policy = root / "policy.json"
            dockerfile.write_text(
                "FROM alpine:3.22@sha256:" + "a" * 64 + "\nUSER 10001\n",
                encoding="utf-8",
            )
            dockerignore.write_text("**\n", encoding="utf-8")
            sbom.write_text('{"spdxVersion":"SPDX-2.3"}\n', encoding="utf-8")
            trivy.write_text('{"Results":[]}\n', encoding="utf-8")
            policy.write_text('{"schema_version":1}\n', encoding="utf-8")

            kwargs = dict(
                source_sha="b" * 40,
                dockerfile=dockerfile,
                dockerignore=dockerignore,
                image="ghcr.io/oteryn/demo",
                image_digest="sha256:" + "c" * 64,
                sbom=sbom,
                trivy=trivy,
                policy=policy,
            )
            first = supply.provenance_manifest(**kwargs)
            second = supply.provenance_manifest(**kwargs)
            self.assertEqual(first, second)
            self.assertEqual(
                "ghcr.io/oteryn/demo@sha256:" + "c" * 64, first["image_ref"]
            )
            self.assertRegex(first["provenance_digest"], r"^sha256:[0-9a-f]{64}$")

    def test_supply_chain_policy_changes_force_all_synology_image_validation(self) -> None:
        classifier_path = ROOT / "scripts/ci/classify_synology_builds.py"
        classifier_spec = importlib.util.spec_from_file_location(
            "classify_synology_builds_supply_chain", classifier_path
        )
        if classifier_spec is None or classifier_spec.loader is None:
            raise RuntimeError(f"cannot load {classifier_path}")
        classifier = importlib.util.module_from_spec(classifier_spec)
        sys.modules[classifier_spec.name] = classifier
        classifier_spec.loader.exec_module(classifier)
        for changed_path in (
            "scripts/ci/container_supply_chain.py",
            "docs/security/CONTAINER_SUPPLY_CHAIN_POLICY.json",
            "docs/security/CONTAINER_VULNERABILITY_EXCEPTIONS.json",
        ):
            with self.subTest(changed_path=changed_path):
                result = classifier.classify(
                    [changed_path],
                    event_name="pull_request",
                    head="a" * 40,
                )
                modes = {
                    item["name"]: item["mode"]
                    for item in result["matrix"]["include"]
                }
                self.assertEqual(
                    {
                        "platform": "full",
                        "game-gateway": "full",
                        "deploy-runner": "full",
                    },
                    modes,
                )

    def test_workflow_and_dependabot_keep_enforcement_wired(self) -> None:
        workflow = (ROOT / ".github/workflows/build-synology-staging-images.yml").read_text(
            encoding="utf-8"
        )
        dependabot = (ROOT / ".github/dependabot.yml").read_text(encoding="utf-8")
        for marker in (
            "anchore/sbom-action@66cbf4bc1f1c0d2edc94016e65bc221b6bb0ad6c",
            "aquasecurity/trivy-action@ed142fd0673e97e23eac54620cfb913e5ce36c25",
            "scripts/ci/container_supply_chain.py evaluate-trivy",
            "scripts/ci/container_supply_chain.py provenance",
            "synology-supply-chain-",
        ):
            self.assertIn(marker, workflow)
        self.assertGreaterEqual(dependabot.count("package-ecosystem: docker"), 2)
        self.assertIn('directory: "/deploy/synology/docker"', dependabot)
        self.assertIn('directory: "/deploy/synology/runner"', dependabot)


if __name__ == "__main__":
    unittest.main(verbosity=2)
