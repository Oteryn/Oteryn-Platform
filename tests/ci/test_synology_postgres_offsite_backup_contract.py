#!/usr/bin/env python3
"""Off-NAS copy of the Platform PostgreSQL backup spool (ADR 0046 §7, review 2026-10-12 F-02).

Static checks of deploy/synology/scripts/offsite-postgres-backup.sh and its configuration template, plus
behaviour tests of its modes against a fake `docker` (no Docker, no network, no restic repository). The
live run against a real repository is the monthly drill of docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md.
"""

from __future__ import annotations

import importlib.util
import os
import re
import shutil
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SYNOLOGY = ROOT / "deploy/synology"
SCRIPTS = SYNOLOGY / "scripts"
OFFSITE = SCRIPTS / "offsite-postgres-backup.sh"
LIB = SCRIPTS / "lib.sh"
TEMPLATE = SYNOLOGY / "offsite-backup.env.example"
RUNBOOK = ROOT / "docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md"
CLASSIFIER = ROOT / "scripts/ci/classify_synology_builds.py"
BUILD_WORKFLOW = ROOT / ".github/workflows/build-synology-staging-images.yml"

PASSWORD = "offsite-test-password-0123456789abcdef"
SNAPSHOT = "331a4e01f0d811218914232d27a99925eaac0212d633a1ce0cc6ccf040c9ec6e"

FAKE_DOCKER = r"""#!/bin/bash
printf '%s\n' "$*" >> "$FAKE_LOG"
case "$1" in
  ps) echo sidecarid ;;
  inspect)
    case "$3" in
      *'"/restore"'*) echo anonvol ;;
      *oteryn-postgres-backup*) echo oteryn-staging_postgres_backup ;;
    esac
    ;;
  run)
    case "$*" in
      *" backup --json "*)
        n=$(( $(cat "$FAKE_COUNT" 2>/dev/null || echo 0) + 1 )); echo "$n" > "$FAKE_COUNT"
        read -r -a rcs <<< "$FAKE_BACKUP_RCS"
        rc="${rcs[$((n - 1))]:-0}"
        (( rc != 0 )) || printf '{"message_type":"summary","files_new":3,"snapshot_id":"%s"}\n' "$FAKE_SNAPSHOT"
        exit "$rc"
        ;;
      *" snapshots --json "*)
        [[ -n "$FAKE_SNAPSHOT_TIME" ]] || { echo '[]'; exit 0; }
        printf '[{"time":"%s","tree":"ab","paths":["/spool/base"],"hostname":"oteryn-staging","id":"%s"}]\n' "$FAKE_SNAPSHOT_TIME" "$FAKE_SNAPSHOT"
        ;;
      *" check "*) exit "${FAKE_CHECK_RC:-0}" ;;
      *" restore "*) exit "${FAKE_RESTORE_RC:-0}" ;;
      *" cat config"*) exit "${FAKE_CAT_RC:-0}" ;;
    esac
    ;;
  # The drill holder and its anonymous volume are gone once the holder stops (--rm).
  container) exit 1 ;;
  volume) exit 1 ;;
esac
exit 0
"""

FAKE_PITR = r"""#!/bin/bash
printf 'pitr %s project=%s\n' "$*" "$COMPOSE_PROJECT_NAME" >> "$FAKE_LOG"
exit "${FAKE_PITR_RC:-0}"
"""


def run(args: list[str], *, env: dict[str, str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, env=env, text=True, capture_output=True, check=False, timeout=60)


class OffsiteTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="oteryn-offsite-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        # The script runs from a copy beside a fake restore-postgres-pitr.sh (the drill calls its sibling).
        scripts = self.tmp / "scripts"
        scripts.mkdir()
        shutil.copy(OFFSITE, scripts / OFFSITE.name)
        shutil.copy(LIB, scripts / "lib.sh")
        (scripts / "restore-postgres-pitr.sh").write_text(FAKE_PITR)
        self.script = scripts / OFFSITE.name
        bin_dir = self.tmp / "bin"
        bin_dir.mkdir()
        (bin_dir / "docker").write_text(FAKE_DOCKER)
        (bin_dir / "docker").chmod(0o755)
        secrets = self.tmp / "secrets"
        secrets.mkdir(mode=0o700)
        self.password = secrets / "restic-password"
        self.password.write_text(PASSWORD + "\n")
        self.password.chmod(0o600)
        self.credentials = secrets / "provider.env"
        self.credentials.write_text("AWS_ACCESS_KEY_ID=test\nAWS_SECRET_ACCESS_KEY=test\n")
        self.credentials.chmod(0o600)
        self.state = self.tmp / "state"
        self.log = self.tmp / "docker.log"
        self.config = self.tmp / "offsite.env"
        self.write_config()
        self.env = {
            "PATH": f"{bin_dir}:{os.environ['PATH']}", "OTERYN_OFFSITE_CONFIG": str(self.config),
            "FAKE_LOG": str(self.log), "FAKE_COUNT": str(self.tmp / "count"), "FAKE_SNAPSHOT": SNAPSHOT,
            "FAKE_BACKUP_RCS": "0", "FAKE_SNAPSHOT_TIME": "",
        }

    def write_config(self, **overrides: str) -> None:
        values = {
            "OTERYN_OFFSITE_REPOSITORY": "s3:https://s3.example.invalid/bucket/oteryn-platform-postgres",
            "OTERYN_OFFSITE_PASSWORD_FILE": str(self.password),
            "OTERYN_OFFSITE_CREDENTIALS_FILE": str(self.credentials),
            "OTERYN_OFFSITE_STATE_DIR": str(self.state),
            **overrides,
        }
        self.config.write_text("".join(f"{key}={value}\n" for key, value in values.items() if value is not None))

    def offsite(self, *args: str, **env: str) -> subprocess.CompletedProcess[str]:
        return run(["bash", str(self.script), *args], env={**self.env, **env})

    def calls(self) -> list[str]:
        return self.log.read_text().splitlines() if self.log.exists() else []

    def status(self) -> dict[str, str]:
        text = (self.state / "status/push.last").read_text()
        return dict(line.split("=", 1) for line in text.splitlines())


class ConfigurationTest(OffsiteTestCase):
    def test_refuses_a_missing_or_unsafe_configuration_before_docker(self) -> None:
        result = run(["bash", str(self.script), "status"], env={**self.env, "OTERYN_OFFSITE_CONFIG": ""})
        self.assertEqual(result.returncode, 2, result.stderr)
        for overrides, message in (
            ({"OTERYN_OFFSITE_REPOSITORY": "REQUIRED_RESTIC_REPOSITORY_URL"}, "restic repository URL"),
            ({"OTERYN_OFFSITE_REPOSITORY": "rest:https://user:secret@backup.example.invalid/repo"}, "carries a password"),
            ({"OTERYN_OFFSITE_REPOSITORY": "s3:https://x/bucket --no-lock"}, "restic repository URL"),
            ({"OTERYN_OFFSITE_PASSWORD_FILE": "relative/password"}, "absolute path"),
            ({"OTERYN_OFFSITE_STATE_DIR": "state"}, "absolute path"),
            ({"OTERYN_OFFSITE_TARGET_RPO_SECONDS": "5m"}, "positive integer"),
            ({"OTERYN_OFFSITE_KEEP_WITHIN_DAILY": "14d --keep-last 0"}, "restic durations"),
            ({"OTERYN_OFFSITE_CHECK_SUBSET": "500%"}, "percentage"),
            ({"OTERYN_OFFSITE_SPOOL_VOLUME": "a,readonly=false"}, "Docker volume name"),
        ):
            with self.subTest(overrides=overrides):
                self.write_config(**overrides)
                result = self.offsite("status")
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertIn(message, result.stderr)
        self.assertEqual(self.calls(), [])

    def test_the_secret_files_are_owner_only_and_the_password_is_long(self) -> None:
        self.password.chmod(0o640)
        result = self.offsite("status")
        self.assertEqual(result.returncode, 2)
        self.assertIn("expected 0600 or stricter", result.stderr)
        self.password.chmod(0o600)
        self.credentials.chmod(0o644)
        self.assertIn("expected 0600 or stricter", self.offsite("status").stderr)
        self.credentials.chmod(0o600)
        self.password.write_text("short\n")
        result = self.offsite("status")
        self.assertEqual(result.returncode, 2)
        self.assertIn("fewer than 32 characters", result.stderr)

    def test_rejects_unknown_modes_and_options(self) -> None:
        for args in (
            [], ["backup"], ["push", "--snapshot", "latest"], ["drill", "--snapshot", "latest; rm"],
            ["restore"], ["restore", "-v"], ["restore", "copy", "--keep-volume"], ["status", "--unknown"],
        ):
            with self.subTest(args=args):
                self.assertEqual(self.offsite(*args).returncode, 2)


class PushTest(OffsiteTestCase):
    def test_push_snapshots_the_spool_read_only_with_the_password_file_never_its_value(self) -> None:
        result = self.offsite("push")
        self.assertEqual(result.returncode, 0, result.stderr)
        backup = next(call for call in self.calls() if " backup --json " in call)
        self.assertIn("--cap-drop ALL --security-opt no-new-privileges", backup)
        self.assertIn("--cap-add DAC_READ_SEARCH", backup)
        self.assertIn("type=volume,source=oteryn-staging_postgres_backup,target=/spool,readonly", backup)
        self.assertIn(f"type=bind,source={self.password},target=/run/oteryn/restic-password,readonly", backup)
        self.assertIn("RESTIC_PASSWORD_FILE=/run/oteryn/restic-password", backup)
        self.assertIn(f"--env-file {self.credentials}", backup)
        self.assertIn("--host oteryn-staging --tag postgres-spool", backup)
        self.assertIn("--exclude .*.partial --exclude .*.tmp.*", backup)
        self.assertTrue(backup.endswith("/spool/base /spool/wal /spool/dump"), backup)
        self.assertNotIn(PASSWORD, self.log.read_text())
        self.assertNotIn("AWS_SECRET_ACCESS_KEY", self.log.read_text())
        status = self.status()
        self.assertEqual(status["result"], "ok")
        self.assertEqual(status["snapshot"], SNAPSHOT)
        self.assertEqual(status["spool_volume"], "oteryn-staging_postgres_backup")
        result = self.offsite("status")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("target_rpo_seconds=300", result.stdout)

    def test_an_incomplete_snapshot_is_retried_once_then_fails(self) -> None:
        result = self.offsite("push", FAKE_BACKUP_RCS="3 0")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(sum(" backup --json " in call for call in self.calls()), 2)
        self.assertIn("retrying once", result.stderr)
        (self.tmp / "count").unlink()
        result = self.offsite("push", FAKE_BACKUP_RCS="3 3")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(self.status()["result"], "failed")
        self.assertEqual(self.offsite("status").returncode, 3)
        (self.tmp / "count").unlink()
        self.log.unlink()
        result = self.offsite("push", FAKE_BACKUP_RCS="1")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(sum(" backup --json " in call for call in self.calls()), 1)

    def test_a_configured_spool_volume_needs_no_container(self) -> None:
        self.write_config(OTERYN_OFFSITE_SPOOL_VOLUME="restored_spool")
        self.assertEqual(self.offsite("push").returncode, 0)
        self.assertFalse(any(call.startswith("ps ") for call in self.calls()))
        self.assertIn("source=restored_spool,target=/spool,readonly", self.log.read_text())

    def test_the_push_takes_no_spool_job_lock_so_a_long_base_backup_never_holds_the_wal_back(self) -> None:
        script = OFFSITE.read_text()
        push = script.split("run_push() {", 1)[1].split("\n}\n", 1)[0]
        self.assertNotIn("job.lock", push)
        self.assertIn('flock -n 8', push)


class FreshnessTest(OffsiteTestCase):
    def test_status_is_degraded_without_a_push_or_with_a_stale_one(self) -> None:
        self.assertEqual(self.offsite("status").returncode, 3)
        (self.state / "status").mkdir(parents=True, exist_ok=True)
        old = int(time.time()) - 301
        (self.state / "status/push.last").write_text(f"result=ok\nepoch={old}\nsnapshot={SNAPSHOT}\n")
        result = self.offsite("status")
        self.assertEqual(result.returncode, 3)
        self.assertIn("above the target RPO of 300s", result.stderr)
        self.write_config(OTERYN_OFFSITE_TARGET_RPO_SECONDS="900")
        self.assertEqual(self.offsite("status").returncode, 0)
        self.assertEqual(self.calls(), [])

    def test_check_reads_the_newest_snapshot_at_the_destination_then_reads_data_back(self) -> None:
        fresh = time.strftime("%Y-%m-%dT%H:%M:%S+00:00", time.gmtime(time.time() - 30))
        stale = time.strftime("%Y-%m-%dT%H:%M:%S+00:00", time.gmtime(time.time() - 3600))
        result = self.offsite("check", FAKE_SNAPSHOT_TIME=fresh)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("check --retry-lock 5m --read-data-subset 5%", self.log.read_text())
        result = self.offsite("check", FAKE_SNAPSHOT_TIME=stale)
        self.assertEqual(result.returncode, 3)
        self.assertIn("above the target RPO", result.stderr)
        result = self.offsite("check", FAKE_SNAPSHOT_TIME=fresh, FAKE_CHECK_RC="1")
        self.assertEqual(result.returncode, 1)
        self.assertIn("damaged", result.stderr)
        result = self.offsite("check")
        self.assertEqual(result.returncode, 1)
        self.assertIn("no snapshot of oteryn-staging", result.stderr)


class RetentionTest(OffsiteTestCase):
    def test_forget_thins_this_hosts_snapshots_and_prunes(self) -> None:
        self.write_config(OTERYN_OFFSITE_KEEP_WITHIN_DAILY="30d")
        result = self.offsite("forget")
        self.assertEqual(result.returncode, 0, result.stderr)
        forget = next(call for call in self.calls() if " forget " in call)
        self.assertIn("--host oteryn-staging --tag postgres-spool --group-by host,tags", forget)
        self.assertIn("--keep-last 1 --keep-within-hourly 48h --keep-within-daily 30d --keep-within-weekly 56d --prune", forget)
        self.assertNotIn("/spool", forget)


class InitTest(OffsiteTestCase):
    def test_init_creates_a_repository_only_where_none_exists(self) -> None:
        self.assertEqual(self.offsite("init").returncode, 0)
        self.assertFalse(any(call.endswith(" init") for call in self.calls()))
        self.log.unlink()
        self.assertEqual(self.offsite("init", FAKE_CAT_RC="10").returncode, 0)
        self.assertTrue(any(call.endswith(" init") for call in self.calls()))
        self.log.unlink()
        # A wrong password (12) or an unreachable destination never ends in a second repository.
        result = self.offsite("init", FAKE_CAT_RC="12")
        self.assertEqual(result.returncode, 1)
        self.assertFalse(any(call.endswith(" init") for call in self.calls()))


class DrillTest(OffsiteTestCase):
    def test_the_drill_restores_into_a_holder_volume_runs_the_offline_pitr_and_records_evidence(self) -> None:
        fresh = time.strftime("%Y-%m-%dT%H:%M:%S+00:00", time.gmtime(time.time() - 30))
        result = self.offsite("drill", "--skip-platform-checks", FAKE_SNAPSHOT_TIME=fresh)
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.calls()
        create = next(call for call in calls if call.startswith("create "))
        self.assertIn("--rm", create)
        self.assertIn("--network none --cap-drop ALL", create)
        self.assertIn("--mount type=volume,target=/restore", create)
        restore = next(call for call in calls if " restore " in call and call.startswith("run "))
        self.assertIn(f"restore {SNAPSHOT}:/spool", restore)
        self.assertIn("--verify", restore)
        self.assertIn("type=volume,source=anonvol,target=/restore", restore)
        lock = next(call for call in calls if "job.lock" in call)
        self.assertIn("chown -R 999:999 /restore/status", lock)
        pitr = next(call for call in calls if call.startswith("pitr "))
        self.assertIn("--offline --spool-volume anonvol --target latest --evidence-dir", pitr)
        self.assertIn("--skip-platform-checks project=oteryn-staging", pitr)
        self.assertLess(calls.index(pitr), max(i for i, call in enumerate(calls) if call.startswith("stop ")))
        evidence = next((self.state / "drills").glob("*/offsite.env")).read_text()
        self.assertIn(f"SNAPSHOT={SNAPSHOT}\n", evidence)
        self.assertIn("REPOSITORY_SCHEME=s3\n", evidence)
        self.assertIn("RTO_SECONDS=14400\n", evidence)
        self.assertIn("RESULT=passed\n", evidence)
        self.assertNotIn("example.invalid", evidence)

    def test_a_failed_restore_or_pitr_fails_the_drill_and_still_discards_the_holder(self) -> None:
        fresh = time.strftime("%Y-%m-%dT%H:%M:%S+00:00", time.gmtime(time.time() - 30))
        for env in ({"FAKE_PITR_RC": "1"}, {"FAKE_RESTORE_RC": "1"}):
            with self.subTest(env=env):
                if self.log.exists():
                    self.log.unlink()
                shutil.rmtree(self.state / "drills", ignore_errors=True)
                result = self.offsite("drill", FAKE_SNAPSHOT_TIME=fresh, **env)
                self.assertEqual(result.returncode, 1)
                self.assertTrue(any(call.startswith("stop ") for call in self.calls()))
                evidence = next((self.state / "drills").glob("*/offsite.env")).read_text()
                self.assertIn("RESULT=failed\n", evidence)
        self.assertFalse(any(call.startswith("pitr ") for call in self.calls()))


class StaticContractTest(unittest.TestCase):
    def test_restic_is_pinned_by_digest_and_nothing_removes_a_volume(self) -> None:
        script = OFFSITE.read_text()
        self.assertRegex(script, r"(?m)^RESTIC_IMAGE='restic/restic:\d+\.\d+\.\d+@sha256:[0-9a-f]{64}'$")
        self.assertNotRegex(script, r"docker\s+volume\s+(rm|remove|prune)\b")
        self.assertNotRegex(script, r"docker\s+(container\s+)?rm\b")
        self.assertNotIn("RESTIC_PASSWORD=", script)
        self.assertNotIn("--privileged", script)

    def test_the_template_names_files_and_holds_no_secret(self) -> None:
        template = TEMPLATE.read_text()
        keys = re.findall(r"(?m)^([A-Z][A-Z0-9_]*)=", template)
        self.assertTrue(keys)
        for key in keys:
            self.assertTrue(key.startswith("OTERYN_OFFSITE_"), key)
            self.assertNotRegex(key, r"KEY|SECRET|TOKEN|PASSWORD$")
        self.assertIn("OTERYN_OFFSITE_REPOSITORY=REQUIRED_RESTIC_REPOSITORY_URL\n", template)
        self.assertIn("OTERYN_OFFSITE_TARGET_RPO_SECONDS=300\n", template)

    def test_routing_syntax_check_and_runbook_cover_the_off_nas_copy(self) -> None:
        spec = importlib.util.spec_from_file_location("classify_synology_builds", CLASSIFIER)
        assert spec is not None and spec.loader is not None
        classifier = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(classifier)
        for path in (OFFSITE, TEMPLATE):
            self.assertIn(path.relative_to(ROOT).as_posix(), classifier.DEPLOYMENT_OPERATOR_EXACT)
        workflow = BUILD_WORKFLOW.read_text()
        syntax = workflow.split("      - name: Validate shell syntax\n", 1)[1].split("          sh -n", 1)[0]
        self.assertIn(f"            {OFFSITE.relative_to(ROOT).as_posix()} \\\n", syntax)
        runbook = RUNBOOK.read_text()
        for needle in (
            "## Off-NAS copy", "offsite-postgres-backup.sh push", "offsite-postgres-backup.sh drill",
            "offsite-postgres-backup.sh forget", "offsite-postgres-backup.sh check", "--spool-volume",
        ):
            self.assertIn(needle, runbook)


if __name__ == "__main__":
    unittest.main(verbosity=2)
