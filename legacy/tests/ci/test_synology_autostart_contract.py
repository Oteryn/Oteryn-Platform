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
        loop = re.search(r"persistent_services=\(([a-z -]+)\)", self.workflow)
        self.assertIsNotNone(loop)
        listed = loop.group(1).split()
        self.assertEqual(self.workflow.count("persistent_services=("), 1)
        self.assertIn('for service in "${persistent_services[@]}"; do', self.workflow)
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

    def _cutover_prelude(self) -> str:
        script = self.workflow.split("        run: |\n", 1)[1]
        script = textwrap.dedent(script)
        return script.split("resolve_one() {", 1)[0]

    # The keys cutover-postgres.sh records after each step marker (lib.sh _oteryn_cutover_step_keys).
    STEP_KEYS = {
        "PREFLIGHT": ("PRESERVED_RULES_HASH",),
        "FREEZE": ("CLOSE_RUN_ID", "MARIADB_VOLUME", "FREEZE_PLATFORM_IMAGE", "FREEZE_GATEWAY_IMAGE"),
        "BOOTSTRAP": ("OAUTH_CLIENT_ID", "OAUTH_RUST_CLIENT_ID", "NATIVE_WORLD_ID", "BOOTSTRAP_PLATFORM_IMAGE"),
        "REENABLE_STARTED": ("REENABLE_CLOSE_RUN_ID",),
        "MARIADB_RETIRED": ("MARIADB_DELETED_AT",),
        "UNFROZEN": ("UNFREEZE_AUDIT_RUN_ID",),
    }
    FORWARD = ("PREFLIGHT", "FREEZE", "BOOTSTRAP", "SMOKE", "REENABLE_STARTED", "NATIVE_SMOKE", "DECLARED_DONE",
               "RETIRE_STARTED", "MARIADB_RETIRED")
    ROLLED_BACK = ("PREFLIGHT", "FREEZE", "ROLLBACK_STARTED", "ROLLBACK_VERIFIED", "UNFROZEN")

    @classmethod
    def state_text(cls, steps: tuple[str, ...]) -> str:
        """The state text cutover-postgres.sh appends for the steps, in order."""
        values = {
            "PRESERVED_RULES_HASH": "c" * 64,
            "FREEZE_PLATFORM_IMAGE": "ghcr.io/oteryn/platform@sha256:" + "a" * 64,
            "FREEZE_GATEWAY_IMAGE": "ghcr.io/oteryn/gateway@sha256:" + "d" * 64,
            "BOOTSTRAP_PLATFORM_IMAGE": "ghcr.io/oteryn/platform@sha256:" + "b" * 64,
        }
        text = "TASK_ID=OTERYN-20261013-test\n"
        for index, step in enumerate(steps):
            text += f"STEP_{step}=2026-10-13T00:00:{index:02d}Z\n"
            for key in cls.STEP_KEYS.get(step, ()):
                text += f"{key}={values.get(key, '1')}\n"
        return text

    def _run_prelude(
        self,
        steps: tuple[str, ...] | list[str] = (),
        lock: bool = False,
        *,
        state: str | None = None,
        bootstrap: tuple[str, ...] = (),
        arrange=None,
    ) -> subprocess.CompletedProcess[str]:
        """Runs the cutover prelude of the repair against a state root; arrange(root) may alter it first."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            state_dir = root / "postgres-cutover"
            if steps or lock or state is not None:
                state_dir.mkdir(mode=0o700)
            if steps or state is not None:
                text = state if state is not None else self.state_text(tuple(steps))
                (state_dir / "cutover-state.env").write_text(text, encoding="utf-8")
            if lock:
                (state_dir / "run.lock").mkdir()
            if bootstrap:
                (root / "postgres-bootstrap").mkdir(mode=0o700)
                for name in bootstrap:
                    (root / "postgres-bootstrap" / name).write_text("X=1\n", encoding="utf-8")
            if arrange is not None:
                arrange(root)
            script = self._cutover_prelude() + 'printf "services=%s\\n" "${persistent_services[*]}"\n'
            return subprocess.run(
                ["bash", "-c", script],
                cwd=ROOT,
                env={"PATH": os.environ["PATH"], "OTERYN_STATE_DIR_INPUT": tmp},
                capture_output=True,
                text=True,
                check=False,
            )

    def assert_refused(self, result: subprocess.CompletedProcess[str], reason: str) -> None:
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("Autostart repair refused:", result.stderr)
        self.assertIn(reason, result.stderr)
        self.assertNotIn("services=", result.stdout)

    def test_repair_reads_the_state_through_the_deploy_gate_check(self) -> None:
        self.assertIn("uses: actions/checkout@", self.workflow)
        self.assertIn("persist-credentials: false", self.workflow)
        prelude = self._cutover_prelude()
        self.assertIn("source deploy/synology/scripts/lib.sh", prelude)
        self.assertIn('oteryn_read_cutover_state "Autostart repair refused"', prelude)
        self.assertIn("oteryn_cutover_bootstrap_record", prelude)
        self.assertNotIn("grep", prelude)

    PERSISTENT = "services=platform internal-proxy gateway marketplace-scheduler\n"

    def test_repair_never_lists_retired_mariadb_without_cutover_state(self) -> None:
        # MariaDB is retired (plan P7): no cutover state brings it back into the repaired services.
        for steps in ((), ("PREFLIGHT",)):
            with self.subTest(steps=steps):
                result = self._run_prelude(steps)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn(self.PERSISTENT, result.stdout)
                self.assertNotIn("mariadb", result.stdout)

    def test_repair_leaves_frozen_cutover_stack_alone(self) -> None:
        for steps, lock in (
            (("PREFLIGHT", "FREEZE"), False),
            (("PREFLIGHT", "FREEZE", "BOOTSTRAP", "SMOKE", "REENABLE_STARTED"), False),
            (self.FORWARD, False),
            (("PREFLIGHT", "FREEZE", "ROLLBACK_STARTED", "ROLLBACK_VERIFIED"), False),
            (("PREFLIGHT",), True),
            # A running step may be mid-append: the lock is honoured before the state is read.
            ((), True),
        ):
            with self.subTest(steps=steps, lock=lock):
                result = self._run_prelude(steps, lock, bootstrap=("bootstrap-complete.env",) if "BOOTSTRAP" in steps else ())
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("::notice::", result.stdout)
                self.assertNotIn("services=", result.stdout)

    def test_repair_skips_retired_mariadb_after_unfreeze(self) -> None:
        # The finished forward cutover keeps the bootstrap records.
        result = self._run_prelude(
            self.FORWARD + ("UNFROZEN",), bootstrap=("witness-freeze.env", "bootstrap-complete.env")
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(self.PERSISTENT, result.stdout)
        self.assertNotIn("mariadb", result.stdout)

    def test_repair_never_lists_mariadb_after_rolled_back_cutover(self) -> None:
        result = self._run_prelude(self.ROLLED_BACK)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(self.PERSISTENT, result.stdout)
        self.assertNotIn("mariadb", result.stdout)

    def test_repair_honours_the_bootstrap_witness_of_an_interrupted_freeze(self) -> None:
        # Codex P1 on PR 1609: record-witness stopped the stack and wrote witness-freeze.env, then the freeze
        # failed before STEP_FREEZE and its exit trap removed run.lock. Any bootstrap record, in any form, or one
        # whose absence cannot be proven, refuses until a finished cutover or the verified rollback's move aside.
        def linked(name: str):
            def arrange(root: Path) -> None:
                (root / "postgres-bootstrap").mkdir(mode=0o700)
                (root / "postgres-bootstrap" / name).symlink_to(root / "missing")
            return arrange

        def directory(root: Path) -> None:
            (root / "postgres-bootstrap").mkdir(mode=0o700)
            (root / "postgres-bootstrap/witness-freeze.env").mkdir()

        cases = [
            ((), ("witness-freeze.env",), None),
            (("PREFLIGHT",), ("witness-freeze.env",), None),
            (("PREFLIGHT",), ("witness-moved-aside.env",), None),
            ((), ("bootstrap-complete.env",), None),
            # A rollback that recorded its verification but left the bootstrap state in place.
            (self.ROLLED_BACK, ("witness-freeze.env",), None),
            (("PREFLIGHT",), (), linked("witness-freeze.env")),
            ((), (), linked("bootstrap-complete.env")),
            (("PREFLIGHT",), (), directory),
        ]
        for steps, records, arrange in cases:
            with self.subTest(steps=steps, records=records, arrange=arrange):
                result = self._run_prelude(steps, bootstrap=records, arrange=arrange)
                self.assert_refused(result, "PostgreSQL cutover record (")

    def test_repair_refuses_malformed_or_uninspectable_cutover_state(self) -> None:
        # Codex P2 on PR 1609: a state the cutover cannot have written is never read as "no cutover".
        valid = self.state_text(("PREFLIGHT", "FREEZE"))

        def linked_state(root: Path) -> None:
            (root / "postgres-cutover").mkdir(mode=0o700)
            (root / "elsewhere.env").write_text(self.state_text(("PREFLIGHT",)))
            (root / "postgres-cutover/cutover-state.env").symlink_to(root / "elsewhere.env")

        def dangling_state(root: Path) -> None:
            (root / "postgres-cutover").mkdir(mode=0o700)
            (root / "postgres-cutover/cutover-state.env").symlink_to(root / "missing.env")

        def directory_state(root: Path) -> None:
            (root / "postgres-cutover").mkdir(mode=0o700)
            (root / "postgres-cutover/cutover-state.env").mkdir()

        def linked_state_dir(root: Path) -> None:
            (root / "target").mkdir(mode=0o700)
            (root / "postgres-cutover").symlink_to(root / "target", target_is_directory=True)

        def shared_state_dir(root: Path) -> None:
            (root / "postgres-cutover").mkdir()
            (root / "postgres-cutover").chmod(0o777)

        def file_state_dir(root: Path) -> None:
            (root / "postgres-cutover").write_text("")

        def file_bootstrap_dir(root: Path) -> None:
            # No bootstrap record can be ruled out below a bootstrap state path that is not a directory.
            (root / "postgres-bootstrap").write_text("")

        def linked_bootstrap_dir(root: Path) -> None:
            (root / "target").mkdir(mode=0o700)
            (root / "postgres-bootstrap").symlink_to(root / "target", target_is_directory=True)

        for label, state, arrange, reason in (
            ("empty", "", None, "empty, truncated or malformed"),
            ("truncated marker", "TASK_ID=OTERYN-20261013-test\nSTEP_PREF", None, "empty, truncated or malformed"),
            ("no final newline", valid[:-1], None, "empty, truncated or malformed"),
            ("freeze keys cut off", valid.split("MARIADB_VOLUME=")[0], None, "empty, truncated or malformed"),
            ("unfrozen without rollback or retirement", self.state_text(("PREFLIGHT", "FREEZE", "UNFROZEN")), None,
             "empty, truncated or malformed"),
            ("retirement without the forward cutover", self.state_text(("PREFLIGHT", "MARIADB_RETIRED")), None,
             "empty, truncated or malformed"),
            ("foreign", "STEP_FREEZE=2026-10-13T00:00:00Z\n", None, "empty, truncated or malformed"),
            ("linked state file", None, linked_state, "not a regular file"),
            ("dangling state link", None, dangling_state, "not a regular file"),
            ("state directory as file", None, directory_state, "not a regular file"),
            ("linked state directory", None, linked_state_dir, "state directory is a link"),
            ("linked bootstrap directory", None, linked_bootstrap_dir, "state directory is a link"),
            ("bootstrap directory is a file", None, file_bootstrap_dir, "must be an owner-only directory"),
            ("shared state directory", None, shared_state_dir, "must be an owner-only directory"),
            ("state directory is a file", None, file_state_dir, "cannot be inspected"),
        ):
            with self.subTest(label):
                result = self._run_prelude(state=state, arrange=arrange)
                self.assert_refused(result, reason)


if __name__ == "__main__":
    unittest.main()
