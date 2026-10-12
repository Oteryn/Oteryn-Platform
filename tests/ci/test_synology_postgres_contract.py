"""PostgreSQL service, TLS and role split on Synology (docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md P4).

Standard library only: ci.yml runs this module through test_synology_deploy_release_identity.py.
"""

from __future__ import annotations

import os
import re
import shutil
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SYNOLOGY = ROOT / "deploy/synology"
COMPOSE = SYNOLOGY / "compose.yml"
MARKETPLACE_COMPOSE = SYNOLOGY / "compose.marketplace.yml"
POSTGRES_CONF = SYNOLOGY / "postgres/postgresql.conf"
PG_HBA = SYNOLOGY / "postgres/pg_hba.conf"
ROLES = SYNOLOGY / "postgres/init/10-platform-roles.sh"
TLS_INIT = SYNOLOGY / "tls/init.sh"
CONTRACT = SYNOLOGY / "release-contract.env"
SCRIPTS = SYNOLOGY / "scripts"
LIB = SCRIPTS / "lib.sh"
DEPLOY = SCRIPTS / "deploy.sh"
HEALTH = SCRIPTS / "health-check.sh"
WORKFLOWS = ROOT / ".github/workflows"

SECRET_NAMES = (
    "PLATFORM_POSTGRES_SUPERUSER_PASSWORD",
    "PLATFORM_POSTGRES_OWNER_PASSWORD",
    "PLATFORM_POSTGRES_APP_PASSWORD",
    "PLATFORM_POSTGRES_BACKUP_PASSWORD",
    "PLATFORM_POSTGRES_DUMP_PASSWORD",
)
# Throwaway values for these tests only; never a deployed secret.
FAKE = {name: f"testonly{index}{'a' * 24}" for index, name in enumerate(SECRET_NAMES)}


def service_block(compose: str, name: str) -> str:
    match = re.search(rf"^  {re.escape(name)}:\n(.*?)(?=^  [a-z][a-z0-9_-]*:\n|^[a-z])", compose, re.M | re.S)
    assert match, f"compose service {name} not found"
    return match.group(1)


def run_lib(script: str, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    base = {"PATH": os.environ["PATH"], "HOME": os.environ.get("HOME", "/tmp")}
    base.update(env or {})
    return subprocess.run(
        ["bash", "-c", f'source "$1"; {script}', "bash", str(LIB)],
        text=True,
        capture_output=True,
        check=False,
        env=base,
    )


class VolumeGuardTest(unittest.TestCase):
    """Plan P4 step 6: no workflow or deploy script removes a data volume."""

    REMOVAL = (
        re.compile(r"docker\s+volume\s+(rm|remove|prune)\b"),
        re.compile(r"\bsystem\s+prune\b"),
        re.compile(r"\bcompose\b[^\n]*\bdown\b[^\n]*(\s-v\b|--volumes)"),
        re.compile(r"docker\s+(container\s+)?rm\b[^\n]*\s(-v|--volumes)\b"),
    )

    def test_hygiene_and_orphan_repair_never_remove_or_name_a_data_volume(self) -> None:
        for name in ("synology-container-hygiene.yml", "repair-synology-compose-orphans.yml"):
            text = (WORKFLOWS / name).read_text()
            for pattern in self.REMOVAL:
                self.assertIsNone(pattern.search(text), f"{name}: {pattern.pattern}")
            for volume in ("mariadb_data", "postgres_data", "platform_storage"):
                self.assertNotIn(volume, text, f"{name} names {volume}")

    def test_staging_workflows_and_deploy_scripts_never_remove_a_volume(self) -> None:
        sources = [
            WORKFLOWS / name
            for name in (
                "deploy-synology-staging.yml",
                "recover-synology-staging-schema.yml",
                "synology-diagnostics.yml",
                "repair-synology-autostart.yml",
            )
        ]
        sources += sorted(SCRIPTS.glob("*.sh"))
        for source in sources:
            text = source.read_text()
            for pattern in self.REMOVAL:
                self.assertIsNone(pattern.search(text), f"{source.name}: {pattern.pattern}")

    def test_postgres_desired_state_comes_from_secrets_not_from_a_container(self) -> None:
        for name in ("synology-diagnostics.yml", "repair-synology-autostart.yml"):
            text = (WORKFLOWS / name).read_text()
            self.assertIn("OTERYN_STAGING_PLATFORM_POSTGRES_DUMP_PASSWORD", text, name)
            self.assertIn('case "$configured" in', text, name)
            self.assertNotRegex(text, r'if \[\[ -n "\$\(docker ps -aq[^\]]*service=postgres', name)
            self.assertNotRegex(text, r'if \[\[ -n "\$postgres_ids" \]\]', name)
            self.assertIn("environment: synology-staging", text, f"{name} must see the environment secrets")
            self.assertIn("A RUNNING postgres service without its secrets", text, name)
            self.assertIn("partially configured", text, name)
        diag = (WORKFLOWS / "synology-diagnostics.yml").read_text()
        block = diag[diag.index('case "$configured" in'):]
        self.assertIn('.State.Running}}\')" = "true"', block.split("esac")[0])
        self.assertIn("Health.Status", block.split("esac")[0])

    def test_compose_keeps_every_data_volume_named_and_local(self) -> None:
        volumes = COMPOSE.read_text().split("\nvolumes:\n", 1)[1].split("\nnetworks:\n", 1)[0]
        for volume in ("mariadb_data", "postgres_data", "platform_storage", "postgres_tls", "postgres_client_ca"):
            self.assertRegex(volumes, rf"(?m)^  {volume}:$")


class ComposeServiceTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.compose = COMPOSE.read_text()
        cls.postgres = service_block(cls.compose, "postgres")

    def test_postgres_is_pinned_profile_gated_private_and_unpublished(self) -> None:
        self.assertRegex(self.postgres, r"(?m)^    image: postgres:17\.\d+@sha256:[0-9a-f]{64}$")
        self.assertIn('    profiles: ["postgres"]\n', self.postgres)
        self.assertIn("    restart: always\n", self.postgres)
        self.assertNotRegex(self.postgres, r"(?m)^    (ports|expose|network_mode):")
        self.assertIn("    networks:\n      - private\n    healthcheck:", self.postgres)
        self.assertIn('    command: ["postgres", "-c", "config_file=/bootstrap/postgres-postgresql.conf"]\n', self.postgres)
        for mount in (
            "      - postgres_data:/var/lib/postgresql/data\n",
            "      - postgres_tls:/run/oteryn-postgres-tls:ro\n",
            "      - bootstrap_files:/bootstrap:ro\n",
        ):
            self.assertIn(mount, self.postgres)

    def test_postgres_is_recreated_when_the_staged_configuration_changes(self) -> None:
        block = self.compose.split("\n  postgres:\n", 1)[1].split("\n  platform:\n", 1)[0]
        self.assertIn("oteryn.postgres-config.digest: ${OTERYN_POSTGRES_CONFIG_DIGEST:-}", block)
        lib = LIB.read_text()
        self.assertIn("OTERYN_POSTGRES_CONFIG_DIGEST", lib)
        self.assertLess(lib.index("export OTERYN_POSTGRES_CONFIG_DIGEST"), lib.index("--profile postgres up -d --no-deps postgres"))

    def test_fast_path_takes_the_full_path_inside_the_certificate_renewal_window(self) -> None:
        lib = LIB.read_text()
        init = TLS_INIT.read_text()
        deploy = DEPLOY.read_text()
        seconds = re.search(r"^OTERYN_POSTGRES_CERT_RENEWAL_SECONDS=(\d+)$", lib, re.M)
        self.assertIsNotNone(seconds)
        # The same window as tls-init's postgres_certificate_is_current.
        self.assertIn(f'openssl x509 -checkend {seconds.group(1)} -noout -in "$POSTGRES_TLS_DIR/server.crt"', init)
        self.assertIn("oteryn_postgres_certificate_is_current()", lib)
        self.assertIn("/run/oteryn-postgres-tls/server.crt", lib)
        preflight = deploy[deploy.index("_oteryn_fast_candidate()") : deploy.index("_oteryn_fast_restore_on_exit()")]
        self.assertIn('oteryn_postgres_certificate_is_current "$postgres_id"', preflight)
        self.assertIn("renewal window", preflight)
        self.assertLess(preflight.index("oteryn_postgres_certificate_is_current"), preflight.index("want_mtls="))

    def test_fast_path_requires_a_current_reconcile_record(self) -> None:
        lib = LIB.read_text()
        deploy = DEPLOY.read_text()
        preflight = deploy[deploy.index("_oteryn_fast_candidate()") : deploy.index("_oteryn_fast_restore_on_exit()")]
        self.assertIn("oteryn_postgres_reconcile_is_current ||", preflight)
        self.assertLess(preflight.index("oteryn_postgres_reconcile_is_current"), preflight.index("want_mtls="))
        reconcile = lib[lib.index("oteryn_reconcile_postgres() {") :]
        reconcile = reconcile[: reconcile.index("\n}\n")]
        # Cleared before any change, written only after the whole reconcile succeeded.
        self.assertLess(reconcile.index("oteryn_postgres_reconcile_clear"), reconcile.index("up -d --no-deps postgres"))
        self.assertGreater(reconcile.index("oteryn_postgres_reconcile_record"), reconcile.index("Verified PostgreSQL"))

    def test_reconcile_record_tracks_secrets_script_and_service_definition(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            state = Path(tmp)
            compose = state / "compose.yml"
            compose.write_text(COMPOSE.read_text())
            env = {
                **os.environ,
                "OTERYN_STATE_DIR": str(state),
                "OTERYN_POSTGRES_COMPOSE_FILE": str(compose),
                **{name: f"{name.lower()}-0123456789abcdef0123456789" for name in SECRET_NAMES},
            }

            def run(snippet: str, **extra: str) -> subprocess.CompletedProcess[str]:
                return subprocess.run(
                    ["bash", "-c", f'set -euo pipefail; source "{LIB}"; {snippet}'],
                    env={**env, **extra}, capture_output=True, text=True, check=False,
                )

            self.assertNotEqual(run("oteryn_postgres_reconcile_is_current").returncode, 0)
            self.assertEqual(run("oteryn_postgres_reconcile_record").returncode, 0)
            record = state / "postgres-reconcile.env"
            self.assertEqual(stat.S_IMODE(record.stat().st_mode), 0o600)
            for value in env.values():
                if "0123456789abcdef" in str(value):
                    self.assertNotIn(value, record.read_text())
            self.assertEqual(run("oteryn_postgres_reconcile_is_current").returncode, 0)
            rotated = run("oteryn_postgres_reconcile_is_current", PLATFORM_POSTGRES_BACKUP_PASSWORD="rotated-0123456789abcdef01234567")
            self.assertNotEqual(rotated.returncode, 0)
            compose.write_text(compose.read_text().replace("postgres:17.11@", "postgres:17.12@", 1))
            self.assertNotEqual(run("oteryn_postgres_reconcile_is_current").returncode, 0)
            compose.write_text(COMPOSE.read_text())
            self.assertEqual(run("oteryn_postgres_reconcile_is_current").returncode, 0)
            self.assertEqual(run("oteryn_postgres_reconcile_clear").returncode, 0)
            self.assertNotEqual(run("oteryn_postgres_reconcile_is_current").returncode, 0)

    def test_postgres_healthcheck_is_a_verified_tls_login_by_the_runtime_role(self) -> None:
        health = self.postgres.split("    healthcheck:\n", 1)[1]
        self.assertIn("pg_isready", health)
        self.assertIn("user=oteryn_platform_app sslmode=verify-full sslrootcert=/run/oteryn-postgres-tls/ca.crt", health)
        self.assertIn("host=postgres port=5432 dbname=oteryn_platform", health)
        # The probe password is read inside the container, never interpolated into the healthcheck.
        self.assertIn('PGPASSWORD=\\"$$OTERYN_POSTGRES_PROBE_PASSWORD\\"', health)

    def test_only_initdb_and_probe_secrets_reach_the_postgres_service(self) -> None:
        self.assertIn("      POSTGRES_PASSWORD: ${PLATFORM_POSTGRES_SUPERUSER_PASSWORD:-}\n", self.postgres)
        self.assertIn("      OTERYN_POSTGRES_PROBE_PASSWORD: ${PLATFORM_POSTGRES_APP_PASSWORD:-}\n", self.postgres)
        self.assertIn("--auth-local=peer --auth-host=scram-sha-256 --data-checksums", self.postgres)
        for name in ("OWNER", "BACKUP", "DUMP"):
            self.assertNotIn(f"PLATFORM_POSTGRES_{name}_PASSWORD", self.compose)

    def test_platform_runtime_gets_the_engine_connection_and_never_a_privileged_postgres_secret(self) -> None:
        anchors = {
            "connection": '"${OTERYN_PLATFORM_DB_CONNECTION:-mysql}"',
            "host": '"${OTERYN_PLATFORM_DB_HOST:-mariadb}"',
            "port": '"${OTERYN_PLATFORM_DB_PORT:-3306}"',
            "username": '"${OTERYN_PLATFORM_DB_USERNAME:-${PLATFORM_DB_USER}}"',
            "password": '"${OTERYN_PLATFORM_DB_PASSWORD:-${PLATFORM_DB_PASSWORD}}"',
            "sslmode": '"${OTERYN_PLATFORM_DB_SSLMODE:-prefer}"',
        }
        for name, value in anchors.items():
            self.assertIn(f"x-platform-db-{name}: &platform_db_{name} {value}\n", self.compose)
        for service in ("platform", "marketplace-scheduler"):
            block = service_block(self.compose, service)
            self.assertNotIn("PLATFORM_POSTGRES_", block, service)
            for key, anchor in (
                ("DB_CONNECTION", "connection"),
                ("DB_HOST", "host"),
                ("DB_PORT", "port"),
                ("DB_USERNAME", "username"),
                ("DB_PASSWORD", "password"),
                ("DB_SSLMODE", "sslmode"),
            ):
                self.assertIn(f"      {key}: *platform_db_{anchor}\n", block, service)
            self.assertIn("      DB_SSLROOTCERT: /run/oteryn-postgres-ca/ca.crt\n", block, service)
            self.assertIn("      - postgres_client_ca:/run/oteryn-postgres-ca:ro\n", block, service)
            self.assertNotIn("postgres_tls", block, service)
        self.assertNotIn("PLATFORM_POSTGRES_", MARKETPLACE_COMPOSE.read_text())

    def test_tls_init_writes_the_postgres_volumes(self) -> None:
        block = service_block(self.compose, "tls-init")
        for line in (
            "      POSTGRES_TLS_DIR: /postgres-tls\n",
            "      POSTGRES_CA_DIR: /postgres-ca\n",
            "      BOOTSTRAP_DIR: /bootstrap\n",
            "      - postgres_tls:/postgres-tls\n",
            "      - postgres_client_ca:/postgres-ca\n",
        ):
            self.assertIn(line, block)
        for service in ("internal-proxy", "gateway", "mariadb"):
            self.assertNotIn("postgres_tls", service_block(self.compose, service), service)


class ServerConfigurationTest(unittest.TestCase):
    def test_pg_hba_admits_only_tls_scram_logins_of_platform_roles(self) -> None:
        rules = [line.split() for line in PG_HBA.read_text().splitlines() if line.strip() and not line.lstrip().startswith("#")]
        self.assertTrue(rules)
        for rule in rules:
            kind, method = rule[0], rule[-1]
            self.assertIn(kind, ("local", "hostssl", "host"), rule)
            self.assertNotIn(method, ("trust", "password", "md5", "ident"), rule)
            if kind == "host":
                self.assertEqual(method, "reject", rule)
            elif kind == "local":
                self.assertIn(rule, (["local", "all", "postgres", "peer"], ["local", "all", "all", "reject"]))
            else:
                self.assertEqual(rule[3:], ["samenet", "scram-sha-256"], rule)
                self.assertTrue(rule[2].startswith("oteryn_platform_"), rule)
        accepted = {(rule[1], rule[2]) for rule in rules if rule[0] == "hostssl"}
        self.assertEqual(
            accepted,
            {
                ("oteryn_platform", "oteryn_platform_owner"),
                ("oteryn_platform", "oteryn_platform_app"),
                ("oteryn_platform", "oteryn_platform_dump"),
                ("replication", "oteryn_platform_backup"),
            },
        )
        self.assertEqual(rules[-2:], [["host", "all", "all", "all", "reject"], ["host", "replication", "all", "all", "reject"]])

    def test_postgresql_conf_enforces_tls13_scram_and_quiet_statements(self) -> None:
        settings = {}
        for line in POSTGRES_CONF.read_text().splitlines():
            line = line.split("#", 1)[0].strip()
            if line:
                key, value = (part.strip() for part in line.split("=", 1))
                settings[key] = value.strip("'")
        expected = {
            "ssl": "on",
            "ssl_min_protocol_version": "TLSv1.3",
            "ssl_cert_file": "/run/oteryn-postgres-tls/server.crt",
            "ssl_key_file": "/run/oteryn-postgres-tls/server.key",
            "password_encryption": "scram-sha-256",
            "hba_file": "/bootstrap/postgres-pg_hba.conf",
            "data_directory": "/var/lib/postgresql/data",
            "port": "5432",
            "log_statement": "none",
        }
        for key, value in expected.items():
            self.assertEqual(settings.get(key), value, key)


class RoleBootstrapTest(unittest.TestCase):
    PASSWORDS = ("ownerTESTONLY" + "o" * 16, "appTESTONLY" + "a" * 16, "backupTESTONLY" + "b" * 16, "dumpTESTONLY" + "d" * 16)

    def run_roles(self, lines: list[str]) -> tuple[subprocess.CompletedProcess[str], list[str]]:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            log = root / "psql.log"
            psql = root / "psql"
            psql.write_text('#!/bin/sh\nprintf "ARGS %s\\n" "$*" >> "$PSQL_LOG"\ncat >> "$PSQL_LOG"\nprintf "\\n--END--\\n" >> "$PSQL_LOG"\n')
            psql.chmod(0o755)
            env = {"PATH": f"{root}:{os.environ['PATH']}", "PSQL_LOG": str(log)}
            result = subprocess.run(
                ["sh", str(ROLES)], input="".join(f"{line}\n" for line in lines), text=True, capture_output=True, check=False, env=env
            )
            sessions = log.read_text().split("--END--")[:-1] if log.exists() else []
            return result, sessions

    def test_script_is_posix_sh(self) -> None:
        subprocess.run(["sh", "-n", str(ROLES)], check=True)

    def test_unsafe_password_sets_are_refused_before_psql(self) -> None:
        owner, app, backup, dump = self.PASSWORDS
        for lines in (
            [owner, app, backup],
            [owner, app, backup, ""],
            [owner, app, backup, "short"],
            [owner, app, backup, "x" * 20 + "';drop"],
            [owner, owner, backup, dump],
            [owner, app, backup, app],
        ):
            result, sessions = self.run_roles(lines)
            self.assertNotEqual(result.returncode, 0, lines)
            self.assertEqual(sessions, [], lines)
            for secret in self.PASSWORDS:
                self.assertNotIn(secret, result.stdout + result.stderr)

    def test_role_split_sql_and_passwords_only_as_psql_variables(self) -> None:
        result, sessions = self.run_roles(list(self.PASSWORDS))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(sessions), 2)
        cluster, database = sessions
        self.assertIn("ARGS -X -q -v ON_ERROR_STOP=1 -d postgres", cluster)
        self.assertIn("ARGS -X -q -v ON_ERROR_STOP=1 -d oteryn_platform", database)
        # Statement logging is off before any password is set; each password appears once, as a \set.
        self.assertLess(cluster.index("SET log_statement = 'none';"), cluster.index("\\set owner_password"))
        for variable, secret in zip(("owner", "app", "backup", "dump"), self.PASSWORDS):
            self.assertEqual(cluster.count(secret), 1, variable)
            self.assertIn(f"\\set {variable}_password '{secret}'", cluster)
            self.assertIn(f"PASSWORD :'{variable}_password';", cluster)
            self.assertNotIn(secret, database)
        roles = {
            "oteryn_platform_owner": "LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS",
            "oteryn_platform_app": "LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS",
            "oteryn_platform_backup": "LOGIN REPLICATION NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS NOINHERIT",
            "oteryn_platform_dump": "LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS",
        }
        for role, attributes in roles.items():
            self.assertIn(f"ALTER ROLE {role} WITH {attributes}", cluster)
        for statement in (
            "ALTER DATABASE oteryn_platform OWNER TO oteryn_platform_owner;",
            "REVOKE ALL ON DATABASE oteryn_platform FROM PUBLIC;",
            "GRANT CONNECT ON DATABASE oteryn_platform TO oteryn_platform_app, oteryn_platform_dump;",
        ):
            self.assertIn(statement, cluster)
        for statement in (
            "ALTER SCHEMA public OWNER TO oteryn_platform_owner;",
            "REVOKE ALL ON SCHEMA public FROM PUBLIC;",
            "GRANT USAGE ON SCHEMA public TO oteryn_platform_app, oteryn_platform_dump;",
            "ALTER DEFAULT PRIVILEGES FOR ROLE oteryn_platform_owner IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO oteryn_platform_app;",
            "ALTER DEFAULT PRIVILEGES FOR ROLE oteryn_platform_owner IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO oteryn_platform_app;",
            "ALTER DEFAULT PRIVILEGES FOR ROLE oteryn_platform_owner IN SCHEMA public GRANT SELECT ON TABLES TO oteryn_platform_dump;",
            "ALTER DEFAULT PRIVILEGES FOR ROLE oteryn_platform_owner IN SCHEMA public GRANT SELECT ON SEQUENCES TO oteryn_platform_dump;",
        ):
            self.assertIn(statement, database)
        # Stale privileges are revoked, current and default, before the intended sets are granted.
        for revoke in (
            "REVOKE ALL ON SCHEMA public FROM oteryn_platform_app, oteryn_platform_dump, oteryn_platform_backup;",
            "REVOKE ALL ON ALL TABLES IN SCHEMA public FROM oteryn_platform_app, oteryn_platform_dump, oteryn_platform_backup;",
            "REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM oteryn_platform_app, oteryn_platform_dump, oteryn_platform_backup;",
            "ALTER DEFAULT PRIVILEGES FOR ROLE oteryn_platform_owner IN SCHEMA public REVOKE ALL ON TABLES FROM oteryn_platform_app, oteryn_platform_dump, oteryn_platform_backup;",
            "ALTER DEFAULT PRIVILEGES FOR ROLE oteryn_platform_owner REVOKE ALL ON SEQUENCES FROM oteryn_platform_app, oteryn_platform_dump, oteryn_platform_backup;",
        ):
            self.assertIn(revoke, database)
            self.assertLess(database.index(revoke), database.index("GRANT USAGE ON SCHEMA public"), revoke)
        self.assertIn("has_table_privilege('oteryn_platform_dump'", LIB.read_text())
        # Neither the runtime nor the dump role ever receives CREATE, ownership, TRUNCATE or a superuser grant.
        for session in (cluster, database):
            for forbidden in ("GRANT CREATE", "GRANT ALL", "TRUNCATE", "pg_read_all_data", "pg_write_all_data", " SUPERUSER "):
                self.assertNotIn(forbidden, session)
        self.assertNotRegex(database, r"GRANT [^;]*(INSERT|UPDATE|DELETE)[^;]*TO oteryn_platform_dump")


class TlsInitTest(unittest.TestCase):
    def test_postgres_server_pair_is_issued_by_the_stack_ca_and_kept_apart(self) -> None:
        if shutil.which("openssl") is None:
            self.skipTest("openssl is not installed")
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            tls, server, client, boot = root / "tls", root / "pg-tls", root / "pg-ca", root / "boot"
            boot.mkdir()
            staged = boot / "postgres-pg_hba.conf"
            staged.write_text("x\n")
            staged.chmod(0o600)
            env = {
                "PATH": os.environ["PATH"],
                "TLS_DIR": str(tls),
                "POSTGRES_TLS_DIR": str(server),
                "POSTGRES_CA_DIR": str(client),
                "POSTGRES_UID": str(os.getuid()),
                "BOOTSTRAP_DIR": str(boot),
            }
            subprocess.run(["sh", str(TLS_INIT)], check=True, capture_output=True, env=env)
            self.assertEqual(sorted(p.name for p in server.iterdir()), ["ca.crt", "server.crt", "server.key"])
            self.assertEqual(sorted(p.name for p in client.iterdir()), ["ca.crt"])
            self.assertEqual(sorted(p.name for p in tls.iterdir()), ["ca.crt", "ca.key", "platform-internal.crt", "platform-internal.key"])
            self.assertEqual(stat.S_IMODE((server / "server.key").stat().st_mode), 0o600)
            self.assertEqual(stat.S_IMODE(server.stat().st_mode), 0o755)
            self.assertEqual(stat.S_IMODE(client.stat().st_mode), 0o755)
            self.assertEqual(stat.S_IMODE(staged.stat().st_mode), 0o644)
            self.assertEqual((client / "ca.crt").read_bytes(), (tls / "ca.crt").read_bytes())
            subprocess.run(
                ["openssl", "verify", "-CAfile", str(tls / "ca.crt"), str(server / "server.crt")], check=True, capture_output=True
            )
            san = subprocess.run(
                ["openssl", "x509", "-noout", "-ext", "subjectAltName", "-in", str(server / "server.crt")],
                check=True,
                capture_output=True,
                text=True,
            ).stdout
            self.assertIn("DNS:postgres", san)

            # Current certificates are kept; a missing server pair is reissued from the same CA.
            before = (server / "server.crt").read_bytes()
            subprocess.run(["sh", str(TLS_INIT)], check=True, capture_output=True, env=env)
            self.assertEqual((server / "server.crt").read_bytes(), before)
            ca = (tls / "ca.crt").read_bytes()
            (server / "server.crt").unlink()
            subprocess.run(["sh", str(TLS_INIT)], check=True, capture_output=True, env=env)
            self.assertEqual((tls / "ca.crt").read_bytes(), ca)
            self.assertTrue((server / "server.crt").exists())


class EngineSwitchTest(unittest.TestCase):
    def test_committed_engine_is_mariadb(self) -> None:
        lines = [line for line in CONTRACT.read_text().splitlines() if line.startswith("PLATFORM_DB_ENGINE=")]
        self.assertEqual(lines, ["PLATFORM_DB_ENGINE=mariadb"])

    def test_engine_is_read_as_data_with_a_mariadb_default(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            contract = Path(td) / "release-contract.env"
            for body, expected in (("", "mariadb"), ("PLATFORM_DB_ENGINE=mariadb\n", "mariadb"), ("PLATFORM_DB_ENGINE=postgres\r\n", "postgres")):
                contract.write_text("OTERYN_MIGRATION_POLICY=expand-contract\n" + body)
                result = run_lib("oteryn_platform_db_engine", {"OTERYN_RELEASE_CONTRACT_FILE": str(contract)})
                self.assertEqual((result.returncode, result.stdout.strip()), (0, expected), body)
            for body in ("PLATFORM_DB_ENGINE=pgsql\n", "PLATFORM_DB_ENGINE=$(id)\n"):
                contract.write_text(body)
                result = run_lib("oteryn_platform_db_engine", {"OTERYN_RELEASE_CONTRACT_FILE": str(contract)})
                self.assertNotEqual(result.returncode, 0, body)
            result = run_lib("oteryn_platform_db_engine", {"OTERYN_RELEASE_CONTRACT_FILE": str(Path(td) / "missing.env")})
            self.assertNotEqual(result.returncode, 0)

    def test_engine_settings_render_the_runtime_connection_for_both_engines(self) -> None:
        printer = (
            "oteryn_platform_db_engine_settings || exit 1; "
            'printf "%s|" "$OTERYN_PLATFORM_DB_ENGINE" "$OTERYN_PLATFORM_DB_CONNECTION" "$OTERYN_PLATFORM_DB_HOST" '
            '"$OTERYN_PLATFORM_DB_PORT" "$OTERYN_PLATFORM_DB_USERNAME" "$OTERYN_PLATFORM_DB_PASSWORD" "$OTERYN_PLATFORM_DB_SSLMODE"'
        )
        base = {"PLATFORM_DB_USER": "oteryn_platform", "PLATFORM_DB_PASSWORD": "mariadbTESTONLY", **FAKE}
        with tempfile.TemporaryDirectory() as td:
            contract = Path(td) / "release-contract.env"
            contract.write_text("PLATFORM_DB_ENGINE=mariadb\n")
            result = run_lib(printer, {**base, "OTERYN_RELEASE_CONTRACT_FILE": str(contract)})
            self.assertEqual(result.stdout, "mariadb|mysql|mariadb|3306|oteryn_platform|mariadbTESTONLY|prefer|", result.stderr)
            contract.write_text("PLATFORM_DB_ENGINE=postgres\n")
            result = run_lib(printer, {**base, "OTERYN_RELEASE_CONTRACT_FILE": str(contract)})
            app = FAKE["PLATFORM_POSTGRES_APP_PASSWORD"]
            self.assertEqual(result.stdout, f"postgres|pgsql|postgres|5432|oteryn_platform_app|{app}|verify-full|", result.stderr)

    def test_load_env_file_exports_the_engine_settings(self) -> None:
        lib = LIB.read_text()
        loader = lib.split("load_oteryn_env_file() {", 1)[1].split("\n}\n", 1)[0]
        self.assertIn("    oteryn_platform_db_engine_settings\n", loader)

    def test_secret_state_is_all_or_none_and_never_prints_a_value(self) -> None:
        base = {"PLATFORM_DB_PASSWORD": "mariadbTESTONLY" + "m" * 16, "MARIADB_ROOT_PASSWORD": "rootTESTONLY" + "r" * 16}
        cases = [
            ({}, 0, "absent"),
            (dict(FAKE), 0, "configured"),
            ({"PLATFORM_POSTGRES_APP_PASSWORD": FAKE["PLATFORM_POSTGRES_APP_PASSWORD"]}, 1, ""),
            ({**FAKE, "PLATFORM_POSTGRES_DUMP_PASSWORD": "short-but-safe"}, 1, ""),
            ({**FAKE, "PLATFORM_POSTGRES_DUMP_PASSWORD": "a" * 30 + " b"}, 1, ""),
            ({**FAKE, "PLATFORM_POSTGRES_DUMP_PASSWORD": FAKE["PLATFORM_POSTGRES_APP_PASSWORD"]}, 1, ""),
            ({**FAKE, "PLATFORM_POSTGRES_OWNER_PASSWORD": base["PLATFORM_DB_PASSWORD"]}, 1, ""),
            ({**FAKE, "PLATFORM_POSTGRES_SUPERUSER_PASSWORD": base["MARIADB_ROOT_PASSWORD"]}, 1, ""),
        ]
        for env, code, output in cases:
            result = run_lib("oteryn_postgres_secret_state", {**base, **env})
            self.assertEqual((result.returncode != 0, result.stdout.strip()), (bool(code), output), env.keys())
            for value in list(FAKE.values()) + list(base.values()):
                self.assertNotIn(value, result.stderr)

    def test_reconcile_skips_without_secrets_and_refuses_the_postgres_engine_without_them(self) -> None:
        result = run_lib('OTERYN_PLATFORM_DB_ENGINE=mariadb; oteryn_reconcile_postgres false')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("not started", result.stdout)
        result = run_lib('OTERYN_PLATFORM_DB_ENGINE=postgres; oteryn_reconcile_postgres false')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("requires all PostgreSQL secrets", result.stderr)

    def test_mariadb_only_operator_scripts_refuse_the_postgres_engine(self) -> None:
        for name in ("recover-schema.sh", "prepare-fresh-schema-baseline.sh", "marketplace-staging.sh"):
            text = (SCRIPTS / name).read_text()
            loader = 'load_oteryn_env_file "$ENV_FILE"\n'
            self.assertIn(loader, text, name)
            self.assertRegex(text.split(loader, 1)[1], r'^# [^\n]*\noteryn_require_mariadb_engine "[^"]+" \|\| exit 1\n', name)
        result = run_lib('OTERYN_PLATFORM_DB_ENGINE=postgres; oteryn_require_mariadb_engine "Schema recovery"')
        self.assertNotEqual(result.returncode, 0)
        result = run_lib('OTERYN_PLATFORM_DB_ENGINE=mariadb; oteryn_require_mariadb_engine "Schema recovery"')
        self.assertEqual(result.returncode, 0)


class DeployWiringTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.deploy = DEPLOY.read_text()

    def test_engine_and_secret_gates_run_before_any_stack_change(self) -> None:
        first_compose = self.deploy.index('compose=(docker compose --env-file "$ENV_FILE"')
        for gate in (
            'if [[ "$OTERYN_PLATFORM_DB_ENGINE" != mariadb ]]; then',
            'OTERYN_PLATFORM_DB_ENGINE" == postgres && "${OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED:-false}" == true',
            "oteryn_postgres_secret_state >/dev/null || exit 1",
        ):
            self.assertLess(self.deploy.index(gate), first_compose, gate)

    def test_bootstrap_files_are_staged_and_postgres_reconciled_after_mariadb(self) -> None:
        stage = self.deploy.split("stage_bootstrap_files() {", 1)[1].split("\n}\n", 1)[0]
        for source, target in (
            ("postgres/postgresql.conf", "postgres-postgresql.conf"),
            ("postgres/pg_hba.conf", "postgres-pg_hba.conf"),
            ("postgres/init/10-platform-roles.sh", "postgres-roles.sh"),
        ):
            self.assertIn(f'docker cp "$DEPLOY_DIR/{source}" "$tls_container:/bootstrap/{target}"', stage)
        reconcile = self.deploy.index('oteryn_reconcile_postgres "${compose[@]}" || exit 1')
        self.assertLess(self.deploy.index('"${compose[@]}" up -d --force-recreate tls-init'), reconcile)
        self.assertLess(self.deploy.index("FLUSH PRIVILEGES;"), reconcile)
        self.assertLess(reconcile, self.deploy.index("\nfinalize_previous_candidate_if_healthy\n"))
        self.assertLess(reconcile, self.deploy.index('"${compose[@]}" up -d platform'))

    def test_reconcile_passes_role_passwords_on_stdin_and_proves_the_contract(self) -> None:
        lib = LIB.read_text()
        body = lib.split("oteryn_reconcile_postgres() {", 1)[1].split("\n}\n", 1)[0]
        self.assertIn("exec -T -u postgres postgres sh /bootstrap/postgres-roles.sh", body)
        self.assertIn('"$PLATFORM_POSTGRES_OWNER_PASSWORD" "$PLATFORM_POSTGRES_APP_PASSWORD"', body)
        self.assertIn("SELECT pg_reload_conf()", body)
        self.assertIn("verify-full", body)
        self.assertIn("PostgreSQL accepted a connection without TLS.", body)
        self.assertIn("permission denied for schema public", body)
        self.assertIn("{{len .HostConfig.PortBindings}}", body)
        self.assertNotRegex(body, r"exec -T -e [A-Z_]*PASSWORD")
        role_sql = lib.split("_oteryn_postgres_role_sql() {", 1)[1].split("\n}\n", 1)[0]
        self.assertIn("IFS= read -r PGPASSWORD", role_sql)
        self.assertNotRegex(role_sql, r"-e PGPASSWORD")

    def test_health_check_requires_a_healthy_unpublished_postgres_once_configured(self) -> None:
        health = HEALTH.read_text()
        self.assertIn('postgres_secret_state="$(oteryn_postgres_secret_state)" || exit 1', health)
        self.assertIn('"${compose[@]}" --profile postgres ps -q postgres', health)
        self.assertIn("{{len .HostConfig.PortBindings}}", health)

    def test_deploy_workflow_renders_the_five_optional_secrets(self) -> None:
        workflow = (WORKFLOWS / "deploy-synology-staging.yml").read_text()
        for name in SECRET_NAMES:
            self.assertIn(f"      {name}: ${{{{ secrets.OTERYN_STAGING_{name} }}}}\n", workflow)
            self.assertIn(f"          {name}=${name}\n", workflow)
        self.assertIn('if [[ -n "${!name:-}" && ! "${!name}" =~ ^[A-Za-z0-9_.-]{24,}$ ]]; then', workflow)
        for name in ("character-bazaar-staging-control.yml", "recover-synology-staging-schema.yml"):
            self.assertNotIn("PLATFORM_POSTGRES_", (WORKFLOWS / name).read_text(), name)


if __name__ == "__main__":
    unittest.main(verbosity=2)
