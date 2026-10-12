#!/usr/bin/env python3
"""Platform PostgreSQL backup and PITR contract (docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md P5).

Static checks of the Compose sidecar, the WAL archive configuration and the deploy wiring, plus
behaviour tests of the archive, restore and retention scripts against a temporary spool. No Docker and
no database: the live restore drill is deploy/synology/scripts/restore-postgres-pitr.sh, run on the NAS
(docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md).
"""

from __future__ import annotations

import fcntl
import gzip
import hashlib
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
COMPOSE = SYNOLOGY / "compose.yml"
POSTGRES_CONF = SYNOLOGY / "postgres/postgresql.conf"
BACKUP_DIR = SYNOLOGY / "postgres/backup"
ARCHIVE = BACKUP_DIR / "archive-wal.sh"
RESTORE = BACKUP_DIR / "restore-wal.sh"
JOB = BACKUP_DIR / "backup-job.sh"
SCRIPTS = SYNOLOGY / "scripts"
BACKUP_POSTGRES = SCRIPTS / "backup-postgres.sh"
DRILL = SCRIPTS / "restore-postgres-pitr.sh"
DEPLOY = SCRIPTS / "deploy.sh"
HEALTH = SCRIPTS / "health-check.sh"
TLS_INIT = SYNOLOGY / "tls/init.sh"
RECOVER_WORKFLOW = ROOT / ".github/workflows/recover-synology-staging-schema.yml"
RUNBOOK = ROOT / "docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md"
CLASSIFIER = ROOT / "scripts/ci/classify_synology_builds.py"
BUILD_WORKFLOW = ROOT / ".github/workflows/build-synology-staging-images.yml"
AUTOSTART_WORKFLOW = ROOT / ".github/workflows/repair-synology-autostart.yml"

SEGMENT = "000000010000000000000003"


def service_block(compose: str, name: str) -> str:
    match = re.search(rf"(?ms)^  {re.escape(name)}:\n(.*?)(?=^  [a-z][a-z0-9_-]*:\n|^[a-z])", compose)
    if match is None:
        raise AssertionError(f"service {name} not found")
    return match.group(1)


def run(args: list[str], *, env: dict[str, str] | None = None, cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args, cwd=cwd, env={**os.environ, **(env or {})}, text=True, capture_output=True, check=False, timeout=60
    )


class ComposeSidecarTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.compose = COMPOSE.read_text()
        cls.postgres = service_block(cls.compose, "postgres")
        cls.sidecar = service_block(cls.compose, "postgres-backup")

    def test_sidecar_uses_the_pinned_postgres_image_and_profile(self) -> None:
        image = re.search(r"(?m)^    image: (postgres:17\.\d+@sha256:[0-9a-f]{64})$", self.postgres)
        self.assertIsNotNone(image)
        self.assertIn(f"    image: {image.group(1)}\n", self.sidecar)
        self.assertIn('    profiles: ["postgres"]\n', self.sidecar)
        self.assertIn("    restart: always\n", self.sidecar)
        self.assertIn('    entrypoint: ["bash", "/bootstrap/postgres-backup/backup-job.sh"]\n', self.sidecar)
        self.assertIn('    command: ["service"]\n', self.sidecar)

    def test_sidecar_is_private_unpublished_and_carries_no_secret(self) -> None:
        self.assertNotRegex(self.sidecar, r"(?m)^    (ports|expose|network_mode|secrets):")
        self.assertIn("    networks:\n      - private\n", self.sidecar)
        self.assertNotIn("PASSWORD", self.sidecar)
        self.assertNotIn("PLATFORM_POSTGRES_", self.sidecar)
        self.assertIn("      postgres:\n        condition: service_healthy\n", self.sidecar)

    def test_spool_is_a_named_volume_outside_the_data_directory(self) -> None:
        mount = "      - postgres_backup:/var/lib/oteryn-postgres-backup\n"
        self.assertIn(mount, self.postgres)
        self.assertIn(mount, self.sidecar)
        self.assertIn("      - postgres_client_ca:/run/oteryn-postgres-ca:ro\n", self.sidecar)
        self.assertIn("      - bootstrap_files:/bootstrap:ro\n", self.sidecar)
        self.assertNotIn("postgres_data", self.sidecar)
        volumes = self.compose.split("\nvolumes:\n", 1)[1].split("\nnetworks:\n", 1)[0]
        self.assertRegex(volumes, r"(?m)^  postgres_backup:$")

    def test_space_floors_are_explicit(self) -> None:
        self.assertIn('      OTERYN_BACKUP_MIN_FREE_MB: "2048"\n', self.sidecar)
        self.assertIn('      OTERYN_BACKUP_WAL_RESERVE_MB: "1024"\n', self.sidecar)
        self.assertIn("status/heartbeat -mmin -15", self.sidecar)


class ArchiveConfigurationTest(unittest.TestCase):
    def test_wal_archive_is_on_with_the_checked_archive_command(self) -> None:
        conf = POSTGRES_CONF.read_text()
        self.assertRegex(conf, r"(?m)^archive_mode = on\b")
        self.assertRegex(conf, r"(?m)^archive_timeout = 60s$")
        self.assertIn(
            "archive_command = '/bin/bash /bootstrap/postgres-backup/archive-wal.sh \"%p\" \"%f\" "
            "&& test -s \"/var/lib/oteryn-postgres-backup/wal/%f.gz\"'\n",
            conf,
        )
        self.assertRegex(conf, r"(?m)^wal_recycle = off$")
        self.assertRegex(conf, r"(?m)^wal_level = replica\b")

    def test_scripts_are_executable_and_parse(self) -> None:
        for script in (ARCHIVE, RESTORE, JOB, BACKUP_POSTGRES, DRILL):
            self.assertTrue(os.access(script, os.X_OK), script.name)
            result = run(["bash", "-n", str(script)])
            self.assertEqual(result.returncode, 0, f"{script.name}: {result.stderr}")


class DeployWiringTest(unittest.TestCase):
    def test_deploy_stages_the_scripts_and_reconciles_after_postgres(self) -> None:
        deploy = DEPLOY.read_text()
        self.assertIn('docker cp "$DEPLOY_DIR/postgres/backup/." "$tls_container:/bootstrap/postgres-backup"', deploy)
        reconcile_postgres = deploy.index('oteryn_reconcile_postgres "${compose[@]}" || exit 1')
        reconcile_backup = deploy.index('bash "$SCRIPT_DIR/backup-postgres.sh" reconcile || exit 1')
        self.assertLess(reconcile_postgres, reconcile_backup)
        self.assertIn('chmod -R a+rX,go-w "$BOOTSTRAP_DIR/postgres-backup"', TLS_INIT.read_text())

    def test_health_check_fails_on_structure_and_warns_on_degraded_backups(self) -> None:
        health = HEALTH.read_text()
        self.assertIn('bash "$SCRIPT_DIR/backup-postgres.sh" check || backup_status=$?', health)
        self.assertIn("        3) echo \"WARNING: PostgreSQL backups are degraded", health)
        self.assertIn("        *) exit 1 ;;", health)
        script = BACKUP_POSTGRES.read_text()
        self.assertIn("        exit 3\n", script)
        # The deploy reconcile is strict: it ends with a check whose non-zero exit fails the script.
        self.assertRegex(script, r"sidecar_job ensure \|\| fail [^\n]*\n(?:\s*#[^\n]*\n)*\s*check\n\}")

    def test_fast_path_requires_reconciled_backups(self) -> None:
        deploy = DEPLOY.read_text()
        candidate = deploy.split("\n_oteryn_fast_candidate() {\n", 1)[1].split("\n}\n", 1)[0]
        probe = candidate.index('bash "$SCRIPT_DIR/backup-postgres.sh" check > /dev/null 2>&1 || {')
        self.assertLess(candidate.index("oteryn_postgres_certificate_is_current"), probe)
        self.assertIn("FAST_DEPLOY_REASON='PostgreSQL backups need reconciliation'", candidate[probe:])
        # Only inside the configured branch: without PostgreSQL secrets nothing is checked.
        self.assertLess(candidate.index('if [[ "$postgres_state" == configured ]]; then'), probe)

    def test_passwords_reach_the_sidecar_on_standard_input_only(self) -> None:
        script = BACKUP_POSTGRES.read_text()
        self.assertIn(
            "printf '%s\\n%s\\n' \"$PLATFORM_POSTGRES_BACKUP_PASSWORD\" \"$PLATFORM_POSTGRES_DUMP_PASSWORD\" \\\n"
            "        | sidecar_job credentials",
            script,
        )
        job = JOB.read_text()
        self.assertIn("sslmode=verify-full", job)
        self.assertNotIn("PGPASSWORD", job)

    def test_restore_drill_never_removes_a_named_volume(self) -> None:
        drill = DRILL.read_text()
        self.assertIn("--rm --label oteryn.pitr-drill=true --network none", drill)
        self.assertIn('--mount "type=volume,source=$spool_volume,target=$SPOOL,readonly"', drill)
        self.assertIn('--mount "type=volume,target=$DATA"', drill)
        self.assertNotRegex(drill, r"docker\s+volume\s+(rm|remove|prune)\b")
        self.assertNotRegex(drill, r"docker\s+(container\s+)?rm\b")

    def test_recover_workflow_offers_a_secret_free_drill_mode(self) -> None:
        workflow = RECOVER_WORKFLOW.read_text()
        self.assertIn("          - schema-recovery\n          - pitr-drill\n", workflow)
        self.assertIn("    if: github.ref == 'refs/heads/main' && inputs.mode == 'schema-recovery'\n", workflow)
        drill = workflow.split("\n  pitr-drill:\n", 1)[1]
        self.assertIn("    if: github.ref == 'refs/heads/main' && inputs.mode == 'pitr-drill'\n", drill)
        self.assertIn("    environment: synology-staging\n", drill)
        self.assertNotIn("secrets.", drill)
        # The schema-recovery job carries the PostgreSQL secrets (P4b); the drill job reads none.
        self.assertNotIn("PLATFORM_POSTGRES_", drill)
        self.assertIn("bash deploy/synology/scripts/restore-postgres-pitr.sh", drill)
        self.assertNotIn("--skip-platform-checks", drill)

    def test_drill_job_timeout_exceeds_the_lock_wait_rto_and_cleanup(self) -> None:
        workflow = RECOVER_WORKFLOW.read_text()
        drill_job = workflow.split("\n  pitr-drill:\n", 1)[1]
        timeout = int(re.search(r"^    timeout-minutes: (\d+)$", drill_job, re.M).group(1))
        script = DRILL.read_text()
        lock_wait = int(re.search(r'^LOCK_WAIT_SECONDS="\$\{OTERYN_DRILL_LOCK_WAIT_SECONDS:-(\d+)\}"$', script, re.M).group(1))
        rto = int(re.search(r"^RTO_SECONDS=(\d+)$", script, re.M).group(1))
        archive_wait = int(re.search(r'^ARCHIVE_WAIT_SECONDS="\$\{OTERYN_ARCHIVE_WAIT_SECONDS:-(\d+)\}"$', script, re.M).group(1))
        # The lock wait (plus the holder's start margin), the RTO from base backup choice to promotion, the
        # probe's archive wait, and at least 30 minutes for the checks after promotion and the cleanup.
        self.assertGreaterEqual(timeout * 60, (lock_wait + 60) + rto + archive_wait + 30 * 60)
        # pg_ctl waits only for what is left of the RTO, so the restore phase cannot outlast it.
        self.assertIn("-w -t $remaining start", script)
        self.assertNotIn("-t $RTO_SECONDS", script)
        runbook = RUNBOOK.read_text()
        self.assertIn(f"The job times out after {timeout} minutes.", runbook)

    def test_build_routing_and_syntax_check_cover_the_backup_scripts(self) -> None:
        spec = importlib.util.spec_from_file_location("classify_synology_builds", CLASSIFIER)
        assert spec is not None and spec.loader is not None
        classifier = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(classifier)
        # deploy.sh runs backup-postgres.sh, so a change to it needs a staging reconcile; the drill runs
        # from the workflow checkout and is validated like the other operator scripts.
        self.assertIn("deploy/synology/scripts/backup-postgres.sh", classifier.DEPLOYMENT_RUNTIME_EXACT)
        self.assertIn("deploy/synology/scripts/restore-postgres-pitr.sh", classifier.DEPLOYMENT_OPERATOR_EXACT)
        self.assertTrue(any("deploy/synology/postgres/backup/archive-wal.sh".startswith(prefix)
                            for prefix in classifier.DEPLOYMENT_RUNTIME_PREFIXES))
        workflow = BUILD_WORKFLOW.read_text()
        syntax = workflow.split("      - name: Validate shell syntax\n", 1)[1].split("          sh -n", 1)[0]
        for script in (BACKUP_POSTGRES, DRILL, ARCHIVE, RESTORE, JOB):
            relative = script.relative_to(ROOT).as_posix()
            self.assertIn(f"            {relative} \\\n", syntax)
        for script in (BACKUP_POSTGRES, DRILL):
            self.assertEqual(workflow.count(f"      - {script.relative_to(ROOT).as_posix()}\n"), 2, script.name)

    def test_runbook_exists_and_is_referenced(self) -> None:
        self.assertTrue(RUNBOOK.is_file())
        for script in (JOB, BACKUP_POSTGRES, DRILL, ARCHIVE):
            self.assertIn("docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md", script.read_text(), script.name)


FAKE_DOCKER = r"""#!/bin/bash
# Containers: $FAKE_CONTAINERS lists service=id (all exist); $FAKE_RUNNING lists the running services.
log="$FAKE_LOG"
cmd="$1"; shift
case "$cmd" in
    ps)
        all=false service=''
        for arg in "$@"; do
            [[ "$arg" == -aq ]] && all=true
            [[ "$arg" == label=com.docker.compose.service=* ]] && service="${arg#label=com.docker.compose.service=}"
        done
        for entry in $FAKE_CONTAINERS; do
            [[ "${entry%%=*}" == "$service" ]] || continue
            if [[ "$all" == true || " $FAKE_RUNNING " == *" $service "* ]]; then echo "${entry#*=}"; fi
        done
        ;;
    update|start) echo "$cmd $*" >> "$log" ;;
    inspect)
        case "$2" in
            *RestartPolicy*) echo always ;;
            *State.Status*) echo running ;;
            *) echo "/${4:-x}" ;;
        esac
        ;;
esac
"""


class AutostartRepairTest(unittest.TestCase):
    """The autostart repair keeps the backup sidecar with the configured PostgreSQL service."""

    BASE = "platform=org-platform mariadb=c-mariadb internal-proxy=c-proxy gateway=c-gateway"

    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="oteryn-p5-autostart-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        docker = self.tmp / "bin/docker"
        docker.parent.mkdir()
        docker.write_text(FAKE_DOCKER)
        docker.chmod(0o755)
        workflow = AUTOSTART_WORKFLOW.read_text()
        lines = []
        for line in workflow.split("        run: |\n", 1)[1].splitlines():
            if line.strip() and not line.startswith(" " * 10):
                break
            lines.append(line[10:])
        script = "\n".join(lines) + "\n"
        # The repair waits for the containers to settle; the fake ones settle at once.
        script = script.replace("\nsleep 3\n", "\n")
        self.script = self.tmp / "repair.sh"
        self.script.write_text(script)
        self.log = self.tmp / "docker.log"

    def repair(self, containers: str, running: str, secrets: bool) -> subprocess.CompletedProcess[str]:
        env = {
            "PATH": f"{self.tmp / 'bin'}:/usr/bin:/bin",
            "FAKE_CONTAINERS": f"{self.BASE} {containers}",
            "FAKE_RUNNING": running,
            "FAKE_LOG": str(self.log),
        }
        for name in ("SUPERUSER", "OWNER", "APP", "BACKUP", "DUMP"):
            env[f"PLATFORM_POSTGRES_{name}_PASSWORD"] = "x" * 24 if secrets else ""
        return run(["bash", str(self.script)], env=env)

    def started(self) -> str:
        return self.log.read_text() if self.log.exists() else ""

    def test_configured_postgres_repairs_the_backup_sidecar_too(self) -> None:
        result = self.repair("postgres=c-pg postgres-backup=c-pgb", "", secrets=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("start c-pg", self.started())
        self.assertIn("start c-pgb", self.started())
        self.assertRegex(self.started(), r"update --restart=always .*c-pg .*c-pgb")

    def test_a_stack_without_the_sidecar_yet_is_left_to_the_next_deployment(self) -> None:
        result = self.repair("postgres=c-pg", "", secrets=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("postgres-backup is not created yet", result.stdout)
        self.assertIn("start c-pg", self.started())

    def test_a_duplicate_sidecar_fails_closed(self) -> None:
        result = self.repair("postgres=c-pg postgres-backup=c-pgb postgres-backup=c-pgb2", "", secrets=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("start", self.started())

    def test_a_running_sidecar_without_secrets_is_drift(self) -> None:
        result = self.repair("postgres=c-pg postgres-backup=c-pgb", "postgres-backup", secrets=False)
        self.assertEqual(result.returncode, 1)
        self.assertIn("A postgres-backup container is running", result.stderr)
        self.assertNotIn("start", self.started())

    def test_stopped_postgres_containers_without_secrets_stay_stopped(self) -> None:
        result = self.repair("postgres=c-pg postgres-backup=c-pgb", "", secrets=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("c-pg", self.started())


class SpoolTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="oteryn-p5-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.spool = self.tmp / "spool"
        for name in ("wal", "base", "dump", "status"):
            (self.spool / name).mkdir(parents=True)
        self.env = {"OTERYN_BACKUP_SPOOL": str(self.spool), "OTERYN_ARCHIVE_MIN_FREE_MB": "0"}

    def segment(self, content: bytes) -> Path:
        path = self.tmp / "pg_wal" / SEGMENT
        path.parent.mkdir(exist_ok=True)
        path.write_bytes(content)
        return path


class ArchiveWalTest(SpoolTestCase):
    def test_archives_compressed_with_checksum_and_accepts_an_identical_retry(self) -> None:
        source = self.segment(b"wal" * 1000)
        result = run(["bash", str(ARCHIVE), str(source), SEGMENT], env=self.env)
        self.assertEqual(result.returncode, 0, result.stderr)
        target = self.spool / "wal" / f"{SEGMENT}.gz"
        self.assertEqual(gzip.decompress(target.read_bytes()), source.read_bytes())
        digest = (self.spool / "wal" / f"{SEGMENT}.gz.sha256").read_text().split()[0]
        self.assertEqual(digest, hashlib.sha256(target.read_bytes()).hexdigest())
        self.assertEqual(run(["bash", str(ARCHIVE), str(source), SEGMENT], env=self.env).returncode, 0)
        self.assertEqual(sorted(p.name for p in (self.spool / "wal").iterdir()), [f"{SEGMENT}.gz", f"{SEGMENT}.gz.sha256"])

    def test_an_identical_retry_syncs_the_files_and_directory_before_success(self) -> None:
        source = self.segment(b"wal" * 100)
        self.assertEqual(run(["bash", str(ARCHIVE), str(source), SEGMENT], env=self.env).returncode, 0)
        fake_bin = self.tmp / "bin"
        fake_bin.mkdir()
        sync_log = self.tmp / "sync.log"
        (fake_bin / "sync").write_text(f'#!/bin/bash\nprintf "%s\\n" "$@" >> {sync_log}\n')
        (fake_bin / "sync").chmod(0o755)
        env = {**self.env, "PATH": f"{fake_bin}:{os.environ['PATH']}"}
        result = run(["bash", str(ARCHIVE), str(source), SEGMENT], env=env)
        self.assertEqual(result.returncode, 0, result.stderr)
        synced = sync_log.read_text().split("\n")
        wal = self.spool / "wal"
        for path in (wal / f"{SEGMENT}.gz", wal / f"{SEGMENT}.gz.sha256", wal):
            self.assertIn(str(path), synced)
        # A failed sync is no success: PostgreSQL keeps the segment and retries.
        (fake_bin / "sync").write_text("#!/bin/bash\nexit 1\n")
        result = run(["bash", str(ARCHIVE), str(source), SEGMENT], env=env)
        self.assertEqual(result.returncode, 1)
        self.assertIn("cannot sync", result.stderr)

    def test_an_identical_retry_succeeds_on_a_spool_below_the_free_space_floor(self) -> None:
        source = self.segment(b"wal" * 100)
        self.assertEqual(run(["bash", str(ARCHIVE), str(source), SEGMENT], env=self.env).returncode, 0)
        full = {**self.env, "OTERYN_ARCHIVE_MIN_FREE_MB": "9999999"}
        result = run(["bash", str(ARCHIVE), str(source), SEGMENT], env=full)
        self.assertEqual(result.returncode, 0, result.stderr)
        # A new segment still meets the floor, and different content is still refused.
        other = self.tmp / "pg_wal" / "000000010000000000000099"
        other.write_bytes(b"next")
        result = run(["bash", str(ARCHIVE), str(other), other.name], env=full)
        self.assertEqual(result.returncode, 1)
        self.assertIn("below", result.stderr)
        source.write_bytes(b"changed")
        result = run(["bash", str(ARCHIVE), str(source), SEGMENT], env=full)
        self.assertEqual(result.returncode, 1)
        self.assertIn("different content", result.stderr)

    def test_refuses_to_overwrite_different_content(self) -> None:
        source = self.segment(b"first")
        self.assertEqual(run(["bash", str(ARCHIVE), str(source), SEGMENT], env=self.env).returncode, 0)
        source.write_bytes(b"second")
        result = run(["bash", str(ARCHIVE), str(source), SEGMENT], env=self.env)
        self.assertEqual(result.returncode, 1)
        self.assertIn("different content", result.stderr)

    def test_a_truncated_copy_never_reports_success(self) -> None:
        source = self.segment(b"x")
        text = ARCHIVE.read_text()
        body = text.index("main() {")
        for cut in range(body + 10, len(text) - len('main "$@"\n'), 97):
            truncated = self.tmp / "archive-wal.sh"
            truncated.write_text(text[:cut])
            result = run(["bash", str(truncated), str(source), SEGMENT], env=self.env)
            self.assertNotEqual(result.returncode, 0, f"cut at {cut}")
        self.assertEqual(list((self.spool / "wal").iterdir()), [])
        # A copy cut before main runs exits 0, so archive_command also requires the archived file.
        truncated.write_text(text[:body])
        command = f'/bin/bash {truncated} "{source}" "{SEGMENT}" && test -s "{self.spool}/wal/{SEGMENT}.gz"'
        self.assertNotEqual(run(["sh", "-c", command], env=self.env).returncode, 0)

    def test_refuses_bad_names_missing_spool_and_low_space(self) -> None:
        source = self.segment(b"x")
        self.assertEqual(run(["bash", str(ARCHIVE), str(source), "../etc/passwd"], env=self.env).returncode, 1)
        missing = {**self.env, "OTERYN_BACKUP_SPOOL": str(self.tmp / "absent")}
        self.assertEqual(run(["bash", str(ARCHIVE), str(source), SEGMENT], env=missing).returncode, 1)
        full = {**self.env, "OTERYN_ARCHIVE_MIN_FREE_MB": "9999999"}
        result = run(["bash", str(ARCHIVE), str(source), SEGMENT], env=full)
        self.assertEqual(result.returncode, 1)
        self.assertIn("below", result.stderr)
        self.assertEqual(run(["bash", str(ARCHIVE), str(source), SEGMENT], env={**self.env, "OTERYN_ARCHIVE_MIN_FREE_MB": "1e3"}).returncode, 1)
        self.assertEqual(list((self.spool / "wal").iterdir()), [])


class RestoreWalTest(SpoolTestCase):
    def archive(self, content: bytes) -> None:
        source = self.segment(content)
        self.assertEqual(run(["bash", str(ARCHIVE), str(source), SEGMENT], env=self.env).returncode, 0)

    def test_restores_an_archived_segment(self) -> None:
        self.archive(b"segment-bytes")
        destination = self.tmp / "RECOVERYXLOG"
        result = run(["bash", str(RESTORE), SEGMENT, str(destination)], env=self.env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(destination.read_bytes(), b"segment-bytes")

    def test_absent_file_is_the_end_of_the_archive(self) -> None:
        destination = self.tmp / "RECOVERYXLOG"
        self.assertEqual(run(["bash", str(RESTORE), SEGMENT, str(destination)], env=self.env).returncode, 1)
        self.assertEqual(run(["bash", str(RESTORE), "00000002.history", str(destination)], env=self.env).returncode, 1)
        self.assertEqual(run(["bash", str(RESTORE), "bad/name", str(destination)], env=self.env).returncode, 1)

    def test_damage_aborts_recovery(self) -> None:
        self.archive(b"segment-bytes")
        destination = self.tmp / "RECOVERYXLOG"
        target = self.spool / "wal" / f"{SEGMENT}.gz"
        target.write_bytes(target.read_bytes() + b"tampered")
        self.assertEqual(run(["bash", str(RESTORE), SEGMENT, str(destination)], env=self.env).returncode, 199)
        (self.spool / "wal" / f"{SEGMENT}.gz.sha256").unlink()
        self.assertEqual(run(["bash", str(RESTORE), SEGMENT, str(destination)], env=self.env).returncode, 199)
        self.assertFalse(destination.exists())
        missing = {**self.env, "OTERYN_BACKUP_SPOOL": str(self.tmp / "absent")}
        self.assertEqual(run(["bash", str(RESTORE), SEGMENT, str(destination)], env=missing).returncode, 199)


class RetentionTest(SpoolTestCase):
    def make_base(self, stamp: str, start_wal: str) -> None:
        directory = self.spool / "base" / stamp
        directory.mkdir()
        (directory / "backup.info").write_text(f"format=1\nkind=base\nstamp={stamp}\nstart_wal={start_wal}\n")

    def job(self, *args: str, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
        return run(["bash", str(JOB), *args], env={**self.env, **(env or {})})

    def test_keeps_seven_daily_and_four_weekly_and_prunes_wal_before_the_oldest_daily(self) -> None:
        # One backup a day for 40 days, ending Sunday 2026-10-11; start WAL segment = day number.
        stamps = []
        for day in range(40):
            stamp = time.strftime("%Y%m%dT020000Z", time.gmtime(time.mktime((2026, 9, 2, 12, 0, 0, 0, 0, 0)) + day * 86400))
            stamps.append(stamp)
            self.make_base(stamp, f"0000000100000000{day + 1:08X}")
        (self.spool / "base" / ".20261011T030000Z.partial").mkdir()
        for day in range(1, 42):
            name = f"0000000100000000{day:08X}"
            (self.spool / "wal" / f"{name}.gz").write_bytes(b"x")
            (self.spool / "wal" / f"{name}.gz.sha256").write_text("0" * 64 + "\n")
        (self.spool / "wal" / "00000002.history.gz").write_bytes(b"h")
        stale = self.spool / "wal" / f".{SEGMENT}.gz.tmp.123"
        stale.write_bytes(b"t")
        os.utime(stale, (time.time() - 7200, time.time() - 7200))
        fresh = self.spool / "wal" / f".{SEGMENT}.gz.tmp.456"
        fresh.write_bytes(b"t")

        newest_first = sorted(stamps, reverse=True)
        retained = self.job("retained", "base")
        self.assertEqual(retained.returncode, 0, retained.stderr)
        kept = retained.stdout.split()
        self.assertEqual(kept[:7], newest_first[:7])
        self.assertEqual(len(kept), 11)
        weeks = [time.strftime("%G-W%V", time.strptime(stamp, "%Y%m%dT%H%M%SZ")) for stamp in kept[7:]]
        self.assertEqual(len(set(weeks)), 4)

        result = self.job("prune")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(sorted(p.name for p in (self.spool / "base").iterdir()), sorted(kept))
        # The oldest retained daily backup started at day 34 (40 - 7 + 1): WAL before it is gone.
        names = sorted(p.name for p in (self.spool / "wal").iterdir())
        self.assertNotIn("000000010000000000000021.gz", names)
        self.assertIn("000000010000000000000022.gz", names)
        self.assertIn("000000010000000000000022.gz.sha256", names)
        self.assertIn("000000010000000000000029.gz", names)
        self.assertIn("00000002.history.gz", names)
        self.assertFalse(stale.exists())
        self.assertTrue(fresh.exists())
        self.assertIn("result=ok", (self.spool / "status" / "prune.last").read_text())

    def test_settings_are_bounded_decimal_integers(self) -> None:
        self.make_base("20261011T020000Z", "000000010000000000000001")
        self.assertEqual(self.job("retained", "base", env={"OTERYN_BACKUP_HOUR_UTC": "08"}).returncode, 0)
        for name, value in (
            ("OTERYN_BACKUP_HOUR_UTC", "24"),
            ("OTERYN_BACKUP_HOUR_UTC", "2 "),
            ("OTERYN_BACKUP_KEEP_DAILY", "0"),
            ("OTERYN_BACKUP_INTERVAL_SECONDS", "99999999"),
            ("OTERYN_BACKUP_MIN_FREE_MB", "-1"),
        ):
            result = self.job("retained", "base", env={name: value})
            self.assertEqual(result.returncode, 1, f"{name}={value!r}")
        self.assertEqual(self.job("retained", "../base").returncode, 1)
        self.assertEqual(self.job("unknown").returncode, 2)

    def test_credentials_refuse_malformed_passwords(self) -> None:
        pgpass = self.tmp / "pgpass"
        env = {"OTERYN_BACKUP_PGPASS_FILE": str(pgpass)}
        result = subprocess.run(
            ["bash", str(JOB), "credentials"], input="short\nalso-short\n", text=True, capture_output=True,
            env={**os.environ, **self.env, **env}, check=False, timeout=60,
        )
        self.assertEqual(result.returncode, 1)
        self.assertFalse(pgpass.exists())
        backup, dump = "b" * 32, "d" * 32
        result = subprocess.run(
            ["bash", str(JOB), "credentials"], input=f"{backup}\n{dump}\n", text=True, capture_output=True,
            env={**os.environ, **self.env, **env}, check=False, timeout=60,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(pgpass.stat().st_mode & 0o777, 0o600)
        self.assertEqual(
            pgpass.read_text(),
            f"postgres:5432:replication:oteryn_platform_backup:{backup}\n"
            f"postgres:5432:oteryn_platform:oteryn_platform_dump:{dump}\n",
        )


class JobLockTest(SpoolTestCase):
    """The sidecar loop and a deploy's ensure decide again under the job lock (backup-job.sh job_still_needed)."""

    def setUp(self) -> None:
        super().setUp()
        self.env = {**self.env, "OTERYN_BACKUP_PGPASS_FILE": str(self.tmp / "absent.pgpass")}
        self.lock_path = self.spool / "status" / "job.lock"

    def complete(self, kind: str) -> None:
        stamp = time.strftime("%Y%m%dT%H%M%SZ", time.gmtime())
        directory = self.spool / kind / stamp
        directory.mkdir()
        info = "backup.info" if kind == "base" else "dump.info"
        (directory / info).write_text(f"format=1\nkind={kind}\nstamp={stamp}\nstart_wal=000000010000000000000001\n")

    def test_a_job_that_waited_for_the_lock_skips_a_backup_another_job_took(self) -> None:
        for kind, condition in (("base", "stale"), ("dump", "due")):
            with self.subTest(kind=kind, condition=condition), open(self.lock_path, "w") as holder:
                fcntl.flock(holder, fcntl.LOCK_EX)
                job = subprocess.Popen(
                    ["bash", str(JOB), kind, condition], env={**os.environ, **self.env},
                    text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                )
                try:
                    time.sleep(1.5)
                    self.assertIsNone(job.poll(), "the job must wait for the lock")
                    # The other job finishes its backup while this one waits.
                    self.complete(kind)
                    fcntl.flock(holder, fcntl.LOCK_UN)
                    _, stderr = job.communicate(timeout=60)
                finally:
                    if job.poll() is None:
                        job.kill()
                self.assertEqual(job.returncode, 0, stderr)
                self.assertIn("no longer needed", stderr)
                self.assertFalse((self.spool / "status" / f"{kind}.last").exists())

    def test_a_backup_still_needed_under_the_lock_runs_and_an_operator_run_always_does(self) -> None:
        # Without credentials the job stops at its preflight: it got past the decision.
        result = run(["bash", str(JOB), "base", "stale"], env=self.env)
        self.assertEqual(result.returncode, 1)
        self.assertIn("result=refused", (self.spool / "status" / "base.last").read_text())
        self.complete("dump")
        result = run(["bash", str(JOB), "dump"], env=self.env)
        self.assertEqual(result.returncode, 1)
        self.assertIn("detail=credentials_missing", (self.spool / "status" / "dump.last").read_text())
        result = run(["bash", str(JOB), "base", "later"], env=self.env)
        self.assertEqual(result.returncode, 1)
        self.assertIn("expects no condition, due or stale", result.stderr)

    def test_loop_and_ensure_pass_their_condition(self) -> None:
        job = JOB.read_text()
        self.assertIn('bash "$self" "$kind" due', job)
        self.assertIn('bash "$self" "$kind" stale', job)
        for kind in ("base", "dump"):
            body = job.split(f"run_{kind}() {{\n", 1)[1]
            self.assertLess(body.index("take_lock"), body.index(f"job_still_needed {kind}"))
            self.assertLess(body.index(f"job_still_needed {kind}"), body.index(f"preflight_or_refuse {kind}"))


class JobPreflightTest(SpoolTestCase):
    """A job reserves only its own backup; the deploy and check reserve the next base backup and dump together."""

    DATABASE_MB = 1000  # pair: 2 x 1064 + 1024 = 3152 MiB; one backup: 1064 + 1024 = 2088 MiB

    def setUp(self) -> None:
        super().setUp()
        self.bin = self.tmp / "bin"
        self.bin.mkdir()
        pgpass = self.tmp / "pgpass"
        pgpass.write_text("postgres:5432:oteryn_platform:oteryn_platform_dump:" + "d" * 32 + "\n")
        pgpass.chmod(0o600)
        tools = {
            "psql": f"echo {self.DATABASE_MB * 1048576}",
            "df": 'echo "Filesystem 1M-blocks Used Available Capacity Mounted"; echo "spool 99999 0 $FAKE_FREE_MB 1% /spool"',
            "pg_dump": 'while (( $# )); do if [[ "$1" == -f ]]; then echo dump > "$2"; fi; shift; done',
            "pg_restore": "echo ';  toc'",
        }
        for name, body in tools.items():
            (self.bin / name).write_text(f"#!/bin/bash\n{body}\n")
            (self.bin / name).chmod(0o755)
        self.env = {
            **self.env, "OTERYN_BACKUP_PGPASS_FILE": str(pgpass), "OTERYN_BACKUP_MIN_FREE_MB": "0",
            "PATH": f"{self.bin}:{os.environ['PATH']}", "FAKE_FREE_MB": "2500",
        }

    def test_a_dump_after_the_base_backup_needs_room_for_itself_only(self) -> None:
        pair = run(["bash", str(JOB), "preflight"], env=self.env)
        self.assertEqual(pair.returncode, 1)
        self.assertIn("required_free_mb=3152", pair.stdout)
        self.assertIn("result=refused", pair.stdout)
        for kind in ("base", "dump"):
            single = run(["bash", str(JOB), "preflight", kind], env=self.env)
            self.assertEqual(single.returncode, 0, single.stderr)
            self.assertIn("required_free_mb=2088", single.stdout)
        result = run(["bash", str(JOB), "dump", "stale"], env=self.env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("result=ok", (self.spool / "status" / "dump.last").read_text())
        short = run(["bash", str(JOB), "dump"], env={**self.env, "FAKE_FREE_MB": "2000"})
        self.assertEqual(short.returncode, 1)
        self.assertIn("detail=refused", (self.spool / "status" / "dump.last").read_text())
        self.assertEqual(run(["bash", str(JOB), "preflight", "wal"], env=self.env).returncode, 1)

    def test_jobs_pass_their_kind_and_status_keeps_the_pair_reserve(self) -> None:
        job = JOB.read_text()
        self.assertIn('output="$(preflight "$1")"', job)
        self.assertIn("preflight_or_refuse base", job)
        self.assertIn("preflight_or_refuse dump", job)
        self.assertIn("    preflight || true\n", job.split("print_status() {", 1)[1])


class DrillEvidenceTest(unittest.TestCase):
    """A drill whose scratch instance outlives it is reported failed in its evidence, not only by its exit."""

    FUNCTIONS = ("log", "record", "late_failure", "cleanup", "evidence_lines", "evidence_table", "write_evidence")

    def harness(
        self, mode: str, *, failed_before: bool = False, evidence_blocked: bool = False, summary_blocked: bool = False,
    ) -> tuple[subprocess.CompletedProcess[str], str, str]:
        tmp = Path(tempfile.mkdtemp(prefix="oteryn-p5-evidence-"))
        self.addCleanup(shutil.rmtree, tmp, True)
        if evidence_blocked:
            # A file where the evidence directory belongs: mkdir -p fails.
            (tmp / "evidence").write_text("")
        summary = tmp / "summary.md"
        if summary_blocked:
            summary.mkdir()
        drill = DRILL.read_text()
        functions = []
        for name in self.FUNCTIONS:
            match = re.search(rf"^{name}\(\) \{{(\n.*?^\}}| [^\n]*\}})\n", drill, re.M | re.S)
            self.assertIsNotNone(match, name)
            functions.append(match.group(0))
        (tmp / "bin").mkdir()
        (tmp / "bin" / "docker").write_text(
            "#!/bin/bash\n"
            'case "$1 $2:$FAKE_MODE" in\n'
            '  "container inspect:container") exit 0 ;;\n'
            '  "volume inspect:volume") exit 0 ;;\n'
            '  "container inspect:"* | "volume inspect:"*) exit 1 ;;\n'
            "esac\nexit 0\n"
        )
        (tmp / "bin" / "docker").chmod(0o755)
        outcome = "record result failed; record failure 'checks failed'; exit 1" if failed_before else "record result passed; exit 0"
        script = "\n".join([
            "set -euo pipefail",
            "declare -A evidence=(); evidence_order=()",
            f"scratch=oteryn-pitr-drill-x; scratch_volume=scratch-volume; scratch_created=true; DRILL=/tmp/d; stamp=S; evidence_dir={tmp}/evidence",
            # The 60-second wait for the container advances the clock instead of sleeping.
            "sleep() { SECONDS=$((SECONDS + 30)); }",
            *functions,
            "trap cleanup EXIT",
            outcome,
        ])
        result = run(["bash", "-c", script], env={
            "PATH": f"{tmp / 'bin'}:{os.environ['PATH']}", "FAKE_MODE": mode, "GITHUB_STEP_SUMMARY": str(summary),
        })
        evidence_file = tmp / "evidence" / "S" / "evidence.env"
        evidence = evidence_file.read_text() if not evidence_blocked and evidence_file.exists() else ""
        return result, evidence, "" if summary_blocked else summary.read_text()

    def test_a_scratch_instance_left_behind_fails_the_recorded_result(self) -> None:
        for mode, what in (("container", "scratch container"), ("volume", "scratch data volume")):
            with self.subTest(mode=mode):
                result, evidence, summary = self.harness(mode)
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertIn("RESULT=failed\n", evidence)
                self.assertIn(f"FAILURE=cleanup: {what}", evidence)
                self.assertNotIn("RESULT=passed", evidence)
                self.assertIn("| result | failed |", summary)

    def test_a_clean_drill_stays_passed_and_an_earlier_failure_stays_first(self) -> None:
        result, evidence, _ = self.harness("clean")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("RESULT=passed\n", evidence)
        result, evidence, _ = self.harness("volume", failed_before=True)
        self.assertEqual(result.returncode, 1)
        self.assertIn("FAILURE=checks failed\n", evidence)
        self.assertIn("CLEANUP_FAILURE=scratch data volume", evidence)

    def test_evidence_that_cannot_be_written_fails_the_drill(self) -> None:
        result, _, summary = self.harness("clean", evidence_blocked=True)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("could not be written", result.stderr)
        self.assertIn("RESULT=failed\n", result.stdout)
        self.assertIn("FAILURE=evidence: cannot write", result.stdout)
        self.assertIn("| result | failed |", summary)
        self.assertNotIn("| result | passed |", summary)
        # A job summary that cannot be written fails it too, and evidence.env does not keep "passed".
        result, evidence, _ = self.harness("clean", summary_blocked=True)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("RESULT=failed\n", evidence)
        self.assertIn("FAILURE=evidence: cannot write the job summary", evidence)
        self.assertNotIn("RESULT=passed", evidence)
        # An earlier failure stays the reported one.
        result, _, _ = self.harness("clean", failed_before=True, evidence_blocked=True)
        self.assertEqual(result.returncode, 1)
        self.assertIn("FAILURE=checks failed\n", result.stdout)
        self.assertIn("EVIDENCE_FAILURE=cannot write", result.stdout)


class DrillLockTest(unittest.TestCase):
    """The drill's scratch instance holds the spool job lock while it chooses and replays a base backup."""

    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="oteryn-p5-lock-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        (self.tmp / "status").mkdir()
        self.lock_path = self.tmp / "status" / "job.lock"
        drill = DRILL.read_text()
        start = drill.index("SCRATCH_MAIN='")
        end = drill.index("\ndone'\n", start) + len("\ndone'\n")
        printed = run(["bash", "-c", drill[start:end] + 'printf %s "$SCRATCH_MAIN"'])
        self.assertEqual(printed.returncode, 0, printed.stderr)
        self.main = printed.stdout

    def holder(self, wait_seconds: int) -> subprocess.Popen[str]:
        return subprocess.Popen(
            ["bash", "-c", self.main, "oteryn-pitr-drill", str(self.tmp), str(wait_seconds), str(self.tmp / "locked")],
            text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )

    def wait_for_marker(self, process: subprocess.Popen[str]) -> None:
        deadline = time.time() + 30
        while not (self.tmp / "locked").exists():
            if process.poll() is not None:
                self.fail(f"the lock holder exited {process.returncode}: {process.communicate()[1]}")
            self.assertLess(time.time(), deadline)
            time.sleep(0.1)

    def test_the_scratch_instance_blocks_backup_jobs_until_it_stops(self) -> None:
        self.lock_path.write_text("")
        process = self.holder(5)
        try:
            self.wait_for_marker(process)
            with open(self.lock_path, "w") as job:
                with self.assertRaises(BlockingIOError):
                    fcntl.flock(job, fcntl.LOCK_EX | fcntl.LOCK_NB)
            process.terminate()
            self.assertEqual(process.wait(timeout=30), 0)
            with open(self.lock_path, "w") as job:
                fcntl.flock(job, fcntl.LOCK_EX | fcntl.LOCK_NB)
        finally:
            if process.poll() is None:
                process.kill()

    def test_the_scratch_instance_waits_for_a_running_job_and_gives_up(self) -> None:
        self.assertEqual(self.holder(1).wait(timeout=30), 3)
        self.lock_path.write_text("")
        with open(self.lock_path, "w") as job:
            fcntl.flock(job, fcntl.LOCK_EX)
            self.assertEqual(self.holder(1).wait(timeout=30), 4)
        self.assertFalse((self.tmp / "locked").exists())

    def test_the_lock_is_held_before_the_backup_is_chosen(self) -> None:
        drill = DRILL.read_text()
        self.assertIn('-c "$SCRATCH_MAIN" oteryn-pitr-drill "$SPOOL" "$LOCK_WAIT_SECONDS" "$LOCKED_MARKER"', drill)
        locked = drill.index('until drill_sh "test -f $LOCKED_MARKER"')
        self.assertLess(drill.index('docker start "$scratch"'), locked)
        self.assertLess(locked, drill.index('restore_started="$(date +%s)"'))
        self.assertLess(locked, drill.index('backups="$(drill_sh'))
        self.assertLess(drill.index('backups="$(drill_sh'), drill.index("pg_ctl -D $DATA -o"))
        self.assertLess(drill.index("pg_ctl -D $DATA -o"), drill.index("dump_result="))
        result = run(["bash", str(DRILL)], env={"OTERYN_DRILL_LOCK_WAIT_SECONDS": "0", "PATH": "/usr/bin:/bin"})
        self.assertEqual(result.returncode, 2)


class DrillArgumentTest(unittest.TestCase):
    def test_rejects_malformed_targets_before_touching_docker(self) -> None:
        for args in (
            ["--target-time"],
            ["--target-time", "2026-10-12T02:00:00"],
            ["--target-time", "2026-10-12 02:00:00; rm"],
            ["--target", "latest"],
            ["--target", "immediate"],
            ["--target", "immediate", "--backup", "20261012"],
            ["--backup", "20261012T020000Z"],
            ["--target-time", "2026-10-12 02:00:00", "--target", "immediate", "--backup", "20261012T020000Z"],
            ["--target", "immediate", "--target-time", "2026-10-12 02:00:00", "--backup", "20261012T020000Z"],
            ["--target", "immediate", "--target-time", "2026-10-12 02:00:00"],
            ["--evidence-dir", ""],
            ["--unknown"],
        ):
            result = run(["bash", str(DRILL), *args], env={"PATH": "/usr/bin:/bin"})
            self.assertEqual(result.returncode, 2, f"{args}: {result.stderr}")
        result = run(["bash", str(DRILL)], env={"OTERYN_DRILL_WAL_MARGIN_MB": "08x"})
        self.assertEqual(result.returncode, 2)
        for project in ("Staging", "a,b", "x" * 64, "-x"):
            result = run(["bash", str(DRILL)], env={"COMPOSE_PROJECT_NAME": project, "PATH": "/usr/bin:/bin"})
            self.assertEqual(result.returncode, 2, project)

    def test_spool_values_never_reach_options_and_live_settings_never_reach_the_scratch(self) -> None:
        drill = DRILL.read_text()
        self.assertIn('[[ "$spool_volume" =~ ^[A-Za-z0-9][A-Za-z0-9_.-]{0,254}$ ]]', drill)
        self.assertLess(drill.index("unexpected backup spool volume name"), drill.index("write_marker A\n"))
        self.assertIn('[[ "$dump_result" =~ ^[0-9]{8}T[0-9]{6}Z$ ]]', drill)
        verify = drill.index("pg_verifybackup -q -m")
        self.assertLess(verify, drill.index(": > $DATA/postgresql.auto.conf"))
        self.assertLess(drill.index(": > $DATA/postgresql.auto.conf"), drill.index("touch $DATA/recovery.signal"))
        job = JOB.read_text()
        service = job.split("    service)\n", 1)[1]
        self.assertLess(service.index('[[ ! -L "$dir" ]]'), service.index("install -d -o postgres"))


if __name__ == "__main__":
    unittest.main(verbosity=2)
