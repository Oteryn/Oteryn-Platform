#!/usr/bin/env python3

from __future__ import annotations

import os
import re
import subprocess
import tempfile
import textwrap
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
        expected = ("mariadb", "platform", "internal-proxy", "gateway", "marketplace-scheduler")
        self.assertIn("persistent_services=(platform internal-proxy gateway marketplace-scheduler)", self.workflow)
        self.assertIn('persistent_services=(mariadb "${persistent_services[@]}")', self.workflow)
        self.assertIn('for service in "${persistent_services[@]}"; do', self.workflow)
        for service in expected:
            block = self.staging_compose.split(f"  {service}:\n", 1)[1]
            next_service = re.search(r"\n  (?! )", block)
            if next_service:
                block = block[: next_service.start()]
            self.assertIn("    restart: always", block)
        self.assertNotIn("resolve_one oteryn-staging tls-init", self.workflow)
        self.assertNotIn("resolve_one oteryn-organization-runners atlas", self.workflow)
        self.assertNotIn("resolve_one oteryn-organization-runners game", self.workflow)

    def test_missing_or_duplicate_container_remains_fail_closed(self) -> None:
        self.assertIn('if [[ "${#matches[@]}" -ne 1 ]]; then', self.workflow)
        self.assertIn('docker update --restart=always "${containers[@]}"', self.workflow)
        self.assertIn('running|restarting', self.workflow)

    def _cutover_prelude(self) -> str:
        script = self.workflow.split("        run: |\n", 1)[1]
        script = textwrap.dedent(script)
        return script.split("resolve_one() {", 1)[0]

    def _run_prelude(self, steps: list[str], lock: bool = False) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as tmp:
            state_dir = Path(tmp) / "postgres-cutover"
            if steps or lock:
                state_dir.mkdir()
            if steps:
                (state_dir / "cutover-state.env").write_text(
                    "".join(f"STEP_{step}=2026-10-14T00:00:00Z\n" for step in steps), encoding="utf-8"
                )
            if lock:
                (state_dir / "run.lock").mkdir()
            script = self._cutover_prelude() + 'printf "services=%s\\n" "${persistent_services[*]}"\n'
            return subprocess.run(
                ["bash", "-c", script],
                env={"PATH": os.environ["PATH"], "OTERYN_STATE_DIR_INPUT": tmp},
                capture_output=True,
                text=True,
                check=False,
            )

    def test_repair_keeps_mariadb_without_cutover_state(self) -> None:
        result = self._run_prelude([])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("services=mariadb platform internal-proxy gateway marketplace-scheduler", result.stdout)

    def test_repair_leaves_frozen_cutover_stack_alone(self) -> None:
        for steps, lock in (
            (["PREFLIGHT", "FREEZE"], False),
            (["PREFLIGHT", "FREEZE", "BOOTSTRAP", "SMOKE", "REENABLE_STARTED"], False),
            (["PREFLIGHT", "FREEZE", "BOOTSTRAP", "DECLARED_DONE", "RETIRE_STARTED", "MARIADB_RETIRED"], False),
            (["PREFLIGHT"], True),
        ):
            with self.subTest(steps=steps, lock=lock):
                result = self._run_prelude(steps, lock)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("::notice::", result.stdout)
                self.assertNotIn("services=", result.stdout)

    def test_repair_skips_retired_mariadb_after_unfreeze(self) -> None:
        steps = ["PREFLIGHT", "FREEZE", "BOOTSTRAP", "DECLARED_DONE", "RETIRE_STARTED", "MARIADB_RETIRED", "UNFROZEN"]
        result = self._run_prelude(steps)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("services=platform internal-proxy gateway marketplace-scheduler", result.stdout)

    def test_repair_keeps_mariadb_after_rolled_back_cutover(self) -> None:
        steps = ["PREFLIGHT", "FREEZE", "ROLLBACK_STARTED", "ROLLBACK_VERIFIED", "UNFROZEN"]
        result = self._run_prelude(steps)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("services=mariadb platform", result.stdout)


if __name__ == "__main__":
    unittest.main()
