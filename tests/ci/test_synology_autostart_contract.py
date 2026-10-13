#!/usr/bin/env python3

from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/repair-synology-autostart.yml"
DIAGNOSTICS = ROOT / ".github/workflows/synology-diagnostics.yml"
ORG_COMPOSE = ROOT / "deploy/synology/runner/compose.organization.example.yml"
STAGING_COMPOSE = ROOT / "deploy/synology/compose.yml"


class SynologyAutostartContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")
        cls.diagnostics = DIAGNOSTICS.read_text(encoding="utf-8")
        cls.org_compose = ORG_COMPOSE.read_text(encoding="utf-8")
        cls.staging_compose = STAGING_COMPOSE.read_text(encoding="utf-8")

    def test_repair_targets_current_platform_organization_runner(self) -> None:
        self.assertIn("resolve_one oteryn-organization-runners platform", self.workflow)
        self.assertNotIn("resolve_one oteryn-deploy-runner runner", self.workflow)
        self.assertIn("group: platform-runners", self.workflow)
        self.assertIn("labels: oteryn-platform", self.workflow)

    def test_workflow_tracks_authoritative_org_compose_input(self) -> None:
        push_block = self.workflow.split("  push:\n", 1)[1].split("  workflow_dispatch:\n", 1)[0]
        self.assertIn("      - deploy/synology/runner/compose.organization.example.yml", push_block)
        self.assertNotIn("      - deploy/synology/runner/compose.yml", push_block)

    def test_org_compose_contract_matches_repair_selector(self) -> None:
        self.assertIn("name: oteryn-organization-runners", self.org_compose)
        platform = self.org_compose.split("  platform:\n", 1)[1].split("\n  atlas:\n", 1)[0]
        self.assertIn("    restart: always", platform)
        self.assertIn("RUNNER_GROUP: platform-runners", platform)
        self.assertIn("RUNNER_NAME: oteryn-synology-platform", platform)
        self.assertIn("RUNNER_LABELS: oteryn-platform", platform)

    def test_only_persistent_platform_owned_host_services_are_repaired(self) -> None:
        expected = ("platform", "internal-proxy", "gateway", "marketplace-scheduler")
        loop = re.search(r"for service in ([a-z -]+); do", self.workflow)
        self.assertIsNotNone(loop)
        listed = loop.group(1).split()
        for service in expected:
            self.assertIn(service, listed)
        # MariaDB is retired (plan P7): neither compose.yml nor the repair workflow names it, and every listed
        # service is a restart-always compose service.
        self.assertNotIn("\n  mariadb:\n", self.staging_compose)
        self.assertNotIn("mariadb", listed)
        self.assertNotIn("service=mariadb", self.workflow)
        self.assertNotRegex(self.workflow, r"resolve_one oteryn-staging \"?mariadb")
        for service in listed:
            block = self.staging_compose.split(f"  {service}:\n", 1)[1]
            next_service = re.search(r"\n  (?! )", block)
            if next_service:
                block = block[: next_service.start()]
            self.assertIn("    restart: always", block)
        self.assertNotIn("resolve_one oteryn-staging tls-init", self.workflow)
        self.assertNotIn("resolve_one oteryn-organization-runners atlas", self.workflow)
        self.assertNotIn("resolve_one oteryn-organization-runners game", self.workflow)

    def test_postgres_is_required_with_all_five_secrets(self) -> None:
        # PostgreSQL is the Platform database (plan P7): no "not configured" branch may skip it.
        self.assertIn('            5)\n              containers+=("$(resolve_one oteryn-staging postgres)")', self.workflow)
        self.assertNotIn("            0)\n", self.workflow)
        self.assertIn("of its 5 secrets are configured", self.workflow)

    def test_diagnostics_require_postgres_and_never_mariadb(self) -> None:
        validate = self.diagnostics.split("      - name: Validate replacement route and diagnostic access\n", 1)[1]
        validate = validate.split("      - name: Collect read-only diagnostics\n", 1)[0]
        self.assertNotIn("service=mariadb", validate)
        self.assertNotIn("mariadb_id", validate)
        self.assertNotRegex(validate, r"expected='[^']*mariadb")
        self.assertIn("expected='platform gateway marketplace-scheduler postgres'", validate)
        self.assertIn("--filter 'label=com.docker.compose.service=postgres')\"", validate)
        self.assertIn('if [[ "$configured" -ne 5 ]]; then', validate)
        self.assertNotIn("            0)\n", validate)
        self.assertNotIn("MariaDB health", self.diagnostics)

    def test_missing_or_duplicate_container_remains_fail_closed(self) -> None:
        self.assertIn('if [[ "${#matches[@]}" -ne 1 ]]; then', self.workflow)
        self.assertIn('docker update --restart=always "${containers[@]}"', self.workflow)
        self.assertIn('running|restarting', self.workflow)


if __name__ == "__main__":
    unittest.main()
