#!/usr/bin/env python3

from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/repair-synology-autostart.yml"
ORG_COMPOSE = ROOT / "deploy/synology/runner/compose.organization.example.yml"
STAGING_COMPOSE = ROOT / "deploy/synology/compose.yml"


class SynologyAutostartContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")
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
        expected = ("mariadb", "redis", "platform", "canary", "internal-proxy", "gateway")
        loop = 'for service in mariadb redis platform canary internal-proxy gateway; do'
        self.assertIn(loop, self.workflow)
        for service in expected:
            block = self.staging_compose.split(f"  {service}:\n", 1)[1]
            next_service = block.find("\n  ")
            if next_service >= 0:
                block = block[:next_service]
            self.assertIn("    restart: always", block)
        self.assertNotIn("resolve_one oteryn-staging tls-init", self.workflow)
        self.assertNotIn("resolve_one oteryn-organization-runners atlas", self.workflow)
        self.assertNotIn("resolve_one oteryn-organization-runners game", self.workflow)

    def test_missing_or_duplicate_container_remains_fail_closed(self) -> None:
        self.assertIn('if [[ "${#matches[@]}" -ne 1 ]]; then', self.workflow)
        self.assertIn('docker update --restart=always "${containers[@]}"', self.workflow)
        self.assertIn('running|restarting', self.workflow)


if __name__ == "__main__":
    unittest.main()
