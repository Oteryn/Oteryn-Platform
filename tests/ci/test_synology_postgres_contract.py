"""PostgreSQL service, TLS and role split on Synology (docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md P4),
and the postgres-engine branches of deploy, recovery, baseline, Marketplace and preflight (P4b).

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
RECOVER = SCRIPTS / "recover-schema.sh"
BASELINE = SCRIPTS / "prepare-fresh-schema-baseline.sh"
MARKETPLACE = SCRIPTS / "marketplace-staging.sh"
PREFLIGHT = SCRIPTS / "production-target-preflight.sh"
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
        for volume in ("mariadb_data", "postgres_data", "platform_storage", "postgres_tls", "postgres_client_ca", "postgres_ca_signer"):
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
        # One implementation: the fast path runs tls-init's own predicate inside the container.
        self.assertIn("--check-postgres-pair", lib)
        self.assertIn("--check-postgres-pair", init)
        self.assertIn("openssl verify -CAfile", init)
        self.assertIn("postgres_key_is_protected", init)
        preflight = deploy[deploy.index("_oteryn_fast_candidate()") : deploy.index("_oteryn_fast_restore_on_exit()")]
        self.assertIn('oteryn_postgres_certificate_is_current "$postgres_id"', preflight)
        self.assertIn("pair is not current", preflight)
        self.assertLess(preflight.index("oteryn_postgres_certificate_is_current"), preflight.index("want_mtls="))

    def test_every_action_rejects_a_partial_secret_set_before_any_stack_change(self) -> None:
        workflow = (WORKFLOWS / "deploy-synology-staging.yml").read_text()
        config = workflow[: workflow.index("- name: Upgrade legacy release provenance state")]
        self.assertIn("postgres_configured != 0 && postgres_configured != 5", config)
        self.assertNotIn("inputs.action == 'deploy'", config[config.index("postgres_configured=0") - 400 : config.index("postgres_configured != 0")])
        rollback = (DEPLOY.parent / "rollback.sh").read_text()
        guard = rollback.index("oteryn_postgres_secret_state >/dev/null || exit 1")
        self.assertGreater(guard, rollback.index("load_oteryn_env_file"))
        for stop in ("release-state.sh", "docker compose", '"${compose[@]}"'):
            if stop in rollback:
                self.assertLess(guard, rollback.index(stop), stop)
        # The other workflows count the same five secrets and fail on a partial set before touching anything.
        for name in ("synology-diagnostics.yml", "repair-synology-autostart.yml"):
            text = (WORKFLOWS / name).read_text()
            self.assertIn("partially configured", text, name)

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
            "      POSTGRES_SIGNER_DIR: /postgres-signer\n",
            "      BOOTSTRAP_DIR: /bootstrap\n",
            "      - postgres_tls:/postgres-tls\n",
            "      - postgres_client_ca:/postgres-ca\n",
            "      - postgres_ca_signer:/postgres-signer\n",
        ):
            self.assertIn(line, block)
        for service in ("internal-proxy", "gateway", "mariadb"):
            self.assertNotIn("postgres_tls", service_block(self.compose, service), service)

    def test_the_postgres_ca_key_volume_is_mounted_by_tls_init_alone(self) -> None:
        # Any container that can read a CA key can mint a certificate Platform accepts for `postgres`.
        services = re.findall(r"(?m)^  ([a-z][a-z0-9-]*):$", self.compose.split("\nservices:\n", 1)[1].split("\nvolumes:\n", 1)[0])
        mounting = [name for name in services if "postgres_ca_signer" in service_block(self.compose, name)]
        self.assertEqual(mounting, ["tls-init"])
        # The stack CA key volume (internal_tls) never carries the PostgreSQL CA, and the PostgreSQL CA is not the stack CA.
        init = TLS_INIT.read_text()
        self.assertIn('-CAkey "$signing_key"', init)
        self.assertIn('issue_certificate postgres "$work" "$PG_CA_CERT" "$POSTGRES_SIGNER_DIR/pgca.key"', init)
        self.assertNotIn('"$TLS_DIR/pgca', init)


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

    def run_roles(self, lines: list[str], *args: str) -> tuple[subprocess.CompletedProcess[str], list[str]]:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            log = root / "psql.log"
            psql = root / "psql"
            psql.write_text('#!/bin/sh\nprintf "ARGS %s\\n" "$*" >> "$PSQL_LOG"\ncat >> "$PSQL_LOG"\nprintf "\\n--END--\\n" >> "$PSQL_LOG"\n')
            psql.chmod(0o755)
            env = {"PATH": f"{root}:{os.environ['PATH']}", "PSQL_LOG": str(log)}
            result = subprocess.run(
                ["sh", str(ROLES), *args], input="".join(f"{line}\n" for line in lines), text=True, capture_output=True, check=False, env=env
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
        # PUBLIC holds EXECUTE on functions and USAGE on types by default: revoked now and by default privileges.
        for public in (
            "REVOKE ALL ON ALL ROUTINES IN SCHEMA public FROM PUBLIC;",
            "ALTER DEFAULT PRIVILEGES FOR ROLE oteryn_platform_owner REVOKE EXECUTE ON FUNCTIONS FROM PUBLIC;",
            "ALTER DEFAULT PRIVILEGES FOR ROLE oteryn_platform_owner REVOKE USAGE ON TYPES FROM PUBLIC;",
            "REVOKE ALL ON TYPE %s FROM PUBLIC",
        ):
            self.assertIn(public, database)
            self.assertLess(database.index(public), database.index("GRANT USAGE ON SCHEMA public"), public)
        reconcile_sql = LIB.read_text()
        for needle in ("p.proacl IS NULL", "t.typacl IS NULL", "d.defaclobjtype = 'f'", "d.defaclobjtype = 'T'",
                       "has_database_privilege(grantee, 'oteryn_platform', 'CREATE,TEMPORARY')"):
            self.assertIn(needle, reconcile_sql)
        self.assertIn("has_table_privilege('oteryn_platform_dump'", LIB.read_text())
        # Neither the runtime nor the dump role ever receives CREATE, ownership, TRUNCATE or a superuser grant.
        for session in (cluster, database):
            for forbidden in ("GRANT CREATE", "GRANT ALL", "TRUNCATE", "pg_read_all_data", "pg_write_all_data", " SUPERUSER "):
                self.assertNotIn(forbidden, session)
        self.assertNotRegex(database, r"GRANT [^;]*(INSERT|UPDATE|DELETE)[^;]*TO oteryn_platform_dump")

    def test_database_only_mode_recreates_the_database_and_grants_without_touching_a_role(self) -> None:
        # The restore path (recover-schema.sh): no password is read, even when one is offered on stdin.
        result, sessions = self.run_roles(list(self.PASSWORDS), "--database-only")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(sessions), 2)
        cluster, database = sessions
        self.assertIn("ARGS -X -q -v ON_ERROR_STOP=1 -d postgres", cluster)
        self.assertIn("ARGS -X -q -v ON_ERROR_STOP=1 -d oteryn_platform", database)
        for secret in self.PASSWORDS:
            self.assertNotIn(secret, cluster + database)
        for forbidden in ("ALTER ROLE", "CREATE ROLE", "PASSWORD", "\\set", "REVOKE %I FROM %I"):
            self.assertNotIn(forbidden, cluster)
        # The roles must exist before anything is created; then the same database boundary as a full run.
        self.assertIn("RAISE EXCEPTION 'A Platform PostgreSQL role is missing", cluster)
        self.assertLess(cluster.index("RAISE EXCEPTION"), cluster.index("CREATE DATABASE oteryn_platform OWNER oteryn_platform_owner"))
        full_cluster = self.run_roles(list(self.PASSWORDS))[1][0]
        boundary = full_cluster[full_cluster.index("SELECT 'CREATE DATABASE"):]
        self.assertIn(boundary.strip(), cluster)
        self.assertEqual(database, self.run_roles(list(self.PASSWORDS))[1][1])

    def test_unknown_mode_is_refused_before_psql(self) -> None:
        result, sessions = self.run_roles(list(self.PASSWORDS), "--roles-only")
        self.assertEqual(result.returncode, 2)
        self.assertEqual(sessions, [])


class TlsInitTest(unittest.TestCase):
    def test_postgres_server_pair_is_issued_by_its_own_ca_and_kept_apart(self) -> None:
        if shutil.which("openssl") is None:
            self.skipTest("openssl is not installed")
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            tls, server, client, boot = root / "tls", root / "pg-tls", root / "pg-ca", root / "boot"
            signer = root / "pg-signer"
            boot.mkdir()
            staged = boot / "postgres-pg_hba.conf"
            staged.write_text("x\n")
            staged.chmod(0o600)
            env = {
                "PATH": os.environ["PATH"],
                "TLS_DIR": str(tls),
                "POSTGRES_TLS_DIR": str(server),
                "POSTGRES_CA_DIR": str(client),
                "POSTGRES_SIGNER_DIR": str(signer),
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
            self.assertEqual(sorted(p.name for p in signer.iterdir()), ["pgca.crt", "pgca.key"])
            self.assertEqual(stat.S_IMODE((signer / "pgca.key").stat().st_mode), 0o400)
            self.assertEqual((client / "ca.crt").read_bytes(), (signer / "pgca.crt").read_bytes())
            self.assertEqual((server / "ca.crt").read_bytes(), (signer / "pgca.crt").read_bytes())
            self.assertNotEqual((tls / "ca.crt").read_bytes(), (signer / "pgca.crt").read_bytes())
            subprocess.run(
                ["openssl", "verify", "-CAfile", str(signer / "pgca.crt"), str(server / "server.crt")], check=True, capture_output=True
            )
            # The stack CA (whose key the ingress can read) does not vouch for the server certificate.
            self.assertNotEqual(
                subprocess.run(["openssl", "verify", "-CAfile", str(tls / "ca.crt"), str(server / "server.crt")], capture_output=True).returncode,
                0,
            )
            for directory in (tls, server, client):
                self.assertFalse(any("pgca" in p.name for p in directory.iterdir()), directory)
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
            pg_ca = (signer / "pgca.crt").read_bytes()
            (server / "server.crt").unlink()
            subprocess.run(["sh", str(TLS_INIT)], check=True, capture_output=True, env=env)
            self.assertEqual((tls / "ca.crt").read_bytes(), ca)
            self.assertEqual((signer / "pgca.crt").read_bytes(), pg_ca)
            self.assertTrue((server / "server.crt").exists())

            # A nonempty key that belongs to another certificate, or a corrupt one, is not "current": both
            # files are reissued from the same CA. The old pair is never left half replaced.
            def public_key(args: list[str]) -> str:
                return subprocess.run(["openssl", *args], check=True, capture_output=True, text=True).stdout

            good_crt = (server / "server.crt").read_bytes()
            other = root / "other.key"
            subprocess.run(["openssl", "genrsa", "-out", str(other), "2048"], check=True, capture_output=True)
            for replacement in (other.read_bytes(), b"not a key\n"):
                (server / "server.key").write_bytes(replacement)
                subprocess.run(["sh", str(TLS_INIT)], check=True, capture_output=True, env=env)
                self.assertNotEqual((server / "server.crt").read_bytes(), good_crt)
                self.assertEqual(
                    public_key(["x509", "-noout", "-pubkey", "-in", str(server / "server.crt")]),
                    public_key(["pkey", "-pubout", "-in", str(server / "server.key")]),
                )
                good_crt = (server / "server.crt").read_bytes()
            self.assertEqual(sorted(p.name for p in server.iterdir()), ["ca.crt", "server.crt", "server.key"])
            self.assertEqual(sorted(p.name for p in tls.iterdir()), ["ca.crt", "ca.key", "platform-internal.crt", "platform-internal.key"])

            # A PostgreSQL CA key that is not the certificate's is never reused: a new CA is created and
            # everything it signs is reissued (rotation path).
            (signer / "pgca.key").chmod(0o600)
            (signer / "pgca.key").write_bytes(other.read_bytes())
            subprocess.run(["sh", str(TLS_INIT)], check=True, capture_output=True, env=env)
            self.assertNotEqual((signer / "pgca.crt").read_bytes(), pg_ca)
            self.assertEqual((client / "ca.crt").read_bytes(), (signer / "pgca.crt").read_bytes())
            self.assertEqual((server / "ca.crt").read_bytes(), (signer / "pgca.crt").read_bytes())
            subprocess.run(
                ["openssl", "verify", "-CAfile", str(signer / "pgca.crt"), str(server / "server.crt")], check=True, capture_output=True
            )
            self.assertEqual(sorted(p.name for p in signer.iterdir()), ["pgca.crt", "pgca.key"])

            # An existing deployment's pair, signed by the stack CA, is replaced and never accepted as current.
            old = root / "old"
            old.mkdir()
            legacy_env = {**env, "POSTGRES_TLS_DIR": str(old), "POSTGRES_CA_DIR": "", "POSTGRES_SIGNER_DIR": str(root / "legacy-signer")}
            subprocess.run(["sh", str(TLS_INIT)], check=True, capture_output=True, env=legacy_env)
            # Sign a certificate with the stack CA key and place it with the stack CA copy, as the earlier version did.
            subprocess.run(["openssl", "req", "-new", "-key", str(old / "server.key"), "-subj", "/CN=postgres", "-out", str(root / "l.csr")], check=True, capture_output=True)
            (root / "l.ext").write_text("subjectAltName=DNS:postgres\nextendedKeyUsage=serverAuth\n")
            (old / "server.crt").unlink()  # 0444, and a non-root runner cannot write over it
            subprocess.run(
                ["openssl", "x509", "-req", "-sha256", "-days", "30", "-in", str(root / "l.csr"), "-CA", str(tls / "ca.crt"), "-CAkey", str(tls / "ca.key"), "-CAcreateserial", "-CAserial", str(root / "l.srl"), "-extfile", str(root / "l.ext"), "-out", str(old / "server.crt")],
                check=True,
                capture_output=True,
            )
            (old / "ca.crt").unlink()
            shutil.copy(tls / "ca.crt", old / "ca.crt")
            legacy_check = {"PATH": os.environ["PATH"], "POSTGRES_TLS_DIR": str(old), "POSTGRES_UID": str(os.getuid())}
            self.assertNotEqual(
                subprocess.run(["sh", "-s", "--", "--check-postgres-pair"], input=TLS_INIT.read_bytes(), capture_output=True, env=legacy_check).returncode,
                0,
            )
            subprocess.run(["sh", str(TLS_INIT)], check=True, capture_output=True, env=legacy_env)
            self.assertEqual(
                subprocess.run(["sh", "-s", "--", "--check-postgres-pair"], input=TLS_INIT.read_bytes(), capture_output=True, env=legacy_check).returncode,
                0,
            )
            self.assertNotEqual((old / "ca.crt").read_bytes(), (tls / "ca.crt").read_bytes())

            # The predicate the deploy fast path runs inside the postgres container (--check-postgres-pair) is
            # the same one: it sees only the served volume and fails on anything inconsistent there.
            check_env = {"PATH": os.environ["PATH"], "POSTGRES_TLS_DIR": str(server), "POSTGRES_UID": str(os.getuid())}

            def fast_path_check() -> int:
                return subprocess.run(
                    ["sh", "-s", "--", "--check-postgres-pair"], input=TLS_INIT.read_bytes(), capture_output=True, env=check_env
                ).returncode

            self.assertEqual(fast_path_check(), 0)
            good_key = (server / "server.key").read_bytes()
            (server / "server.key").write_bytes(other.read_bytes())
            self.assertNotEqual(fast_path_check(), 0)  # a key of another certificate
            (server / "server.key").write_bytes(good_key)
            (server / "server.key").chmod(0o640)
            self.assertNotEqual(fast_path_check(), 0)  # readable by others: the server would refuse it
            (server / "server.key").chmod(0o600)
            self.assertEqual(fast_path_check(), 0)
            foreign = root / "foreign-ca"
            foreign.mkdir()
            subprocess.run(
                ["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "30", "-subj", "/CN=Foreign", "-keyout", str(foreign / "k"), "-out", str(foreign / "ca.crt")],
                check=True,
                capture_output=True,
            )
            good_ca = (server / "ca.crt").read_bytes()
            (server / "ca.crt").chmod(0o644)
            (server / "ca.crt").write_bytes((foreign / "ca.crt").read_bytes())
            self.assertNotEqual(fast_path_check(), 0)  # a certificate the CA copy does not verify
            (server / "ca.crt").write_bytes(good_ca)
            self.assertEqual(fast_path_check(), 0)

            # The platform-internal pair is checked for consistency too, not only for presence and expiry.
            old_ca = (tls / "ca.crt").read_bytes()
            (tls / "platform-internal.key").chmod(0o600)
            (tls / "platform-internal.key").write_bytes(other.read_bytes())
            subprocess.run(["sh", str(TLS_INIT)], check=True, capture_output=True, env=env)
            self.assertNotEqual((tls / "ca.crt").read_bytes(), old_ca)
            self.assertEqual(
                public_key(["x509", "-noout", "-pubkey", "-in", str(tls / "platform-internal.crt")]),
                public_key(["pkey", "-pubout", "-in", str(tls / "platform-internal.key")]),
            )
            self.assertEqual(fast_path_check(), 0)

    def test_postgres_key_is_never_written_to_the_shared_tls_volume(self) -> None:
        init = TLS_INIT.read_text()
        issue = init[init.index("issue_postgres_certificate() {") : init.index("publish_postgres_bootstrap_files() {")]
        self.assertIn('issue_certificate postgres "$work"', issue)
        self.assertIn('mktemp -d "$POSTGRES_TLS_DIR/.issue.XXXXXX"', issue)
        self.assertNotIn('"$TLS_DIR/postgres.', issue)


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

    def test_operator_scripts_no_longer_refuse_the_postgres_engine(self) -> None:
        self.assertNotIn("oteryn_require_mariadb_engine", LIB.read_text())
        for path in (DEPLOY, RECOVER, BASELINE, MARKETPLACE):
            text = path.read_text()
            self.assertNotIn("oteryn_require_mariadb_engine", text, path.name)
            self.assertNotIn("is reserved for the staging cutover", text, path.name)


class DeployWiringTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.deploy = DEPLOY.read_text()

    def test_engine_and_secret_gates_run_before_any_stack_change(self) -> None:
        first_compose = self.deploy.index('compose=(docker compose --env-file "$ENV_FILE"')
        for gate in (
            'OTERYN_PLATFORM_DB_ENGINE" == postgres && "${OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED:-false}" == true',
            'postgres_secret_state="$(oteryn_postgres_secret_state)" || exit 1',
            'echo "PLATFORM_DB_ENGINE=postgres requires all five PostgreSQL secrets." >&2',
            'if [[ "$PLATFORM_DB_NAME" != oteryn_platform ]]; then',
        ):
            self.assertLess(self.deploy.index(gate), first_compose, gate)
        self.assertNotIn('if [[ "$OTERYN_PLATFORM_DB_ENGINE" != mariadb ]]; then', self.deploy)

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



def fake_docker_env(td: Path) -> tuple[dict[str, str], Path]:
    """A fake `docker` on PATH that logs its argv, the DB_MIGRATIONS_* names it sees and its stdin."""
    log = td / "docker.log"
    fake = td / "docker"
    fake.write_text(
        "#!/bin/sh\n"
        'printf "ARGV %s\\n" "$*" >> "$DOCKER_LOG"\n'
        'printf "ENV DB_MIGRATIONS_USERNAME=%s DB_MIGRATIONS_PASSWORD=%s\\n" "${DB_MIGRATIONS_USERNAME:-}" "${DB_MIGRATIONS_PASSWORD:-}" >> "$DOCKER_LOG"\n'
        'case "$*" in *"pg_dump"*) cat >/dev/null; printf "PGDMP-archive";; *"pg_restore --list"*) cat >/dev/null;; *"psql"*) printf "0\\n";; esac\n'
    )
    fake.chmod(0o755)
    return {"PATH": f"{td}:{os.environ['PATH']}", "DOCKER_LOG": str(log)}, log


class PostgresEngineBranchTest(unittest.TestCase):
    """Plan P4b: the postgres engine migrates as the owner role and dumps, restores and checks PostgreSQL."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.lib = LIB.read_text()
        cls.deploy = DEPLOY.read_text()

    def body(self, text: str, name: str) -> str:
        return text.split(f"{name}() {{", 1)[1].split("\n}\n", 1)[0]

    def test_migration_runs_in_a_one_off_container_as_the_owner_with_fastcgi_off(self) -> None:
        args = self.lib.split("OTERYN_POSTGRES_MIGRATE_ARGS=(", 1)[1].split("\n)\n", 1)[0].split()
        self.assertEqual(args[:4], ["run", "--rm", "--no-deps", "-T"])
        self.assertIn("OTERYN_PLATFORM_FASTCGI_ENABLED=false", args)
        # The owner credentials pass by name only, never as a value in an argument.
        self.assertEqual(args.count("-e"), 3)
        self.assertIn("DB_MIGRATIONS_USERNAME", args)
        self.assertIn("DB_MIGRATIONS_PASSWORD", args)
        self.assertEqual(" ".join(args[args.index("platform"):]), "platform php artisan migrate --force --no-interaction --database=pgsql_migrations")
        config = (ROOT / "config/database.php").read_text()
        self.assertIn("'pgsql_migrations' => [", config)
        self.assertIn("DB_MIGRATIONS_USERNAME", config)

    def test_migrate_helper_hands_the_owner_password_only_through_the_environment(self) -> None:
        owner = FAKE["PLATFORM_POSTGRES_OWNER_PASSWORD"]
        with tempfile.TemporaryDirectory() as td:
            env, log = fake_docker_env(Path(td))
            result = run_lib("oteryn_platform_migrate_postgres docker compose -f x.yml", {**env, **FAKE})
            self.assertEqual(result.returncode, 0, result.stderr)
            argv, seen = log.read_text().splitlines()
        self.assertEqual(
            argv,
            "ARGV compose -f x.yml run --rm --no-deps -T -e OTERYN_PLATFORM_FASTCGI_ENABLED=false "
            "-e DB_MIGRATIONS_USERNAME -e DB_MIGRATIONS_PASSWORD platform php artisan migrate --force --no-interaction "
            "--database=pgsql_migrations",
        )
        self.assertEqual(seen, f"ENV DB_MIGRATIONS_USERNAME=oteryn_platform_owner DB_MIGRATIONS_PASSWORD={owner}")
        self.assertNotIn(owner, result.stdout + result.stderr)
        # Without the owner password nothing runs.
        with tempfile.TemporaryDirectory() as td:
            env, log = fake_docker_env(Path(td))
            result = run_lib("oteryn_platform_migrate_postgres docker compose", env)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(log.exists())

    def test_deploy_migrates_by_engine_and_the_wrapper_brackets_both_forms(self) -> None:
        branch = (
            'if [[ "$OTERYN_PLATFORM_DB_ENGINE" == postgres ]]; then\n'
            "    # Owner role in a one-off container; the runtime platform container never holds the owner password.\n"
            '    oteryn_platform_migrate_postgres "${compose[@]}"\n'
            "else\n"
            '    "${compose[@]}" exec -T platform php artisan migrate --force --no-interaction\n'
            "fi\n"
        )
        self.assertIn('"${compose[@]}" up -d platform\n' + branch, self.deploy)
        wrapper = self.body(self.lib, "docker")
        self.assertIn('|| "$joined" == *" ${OTERYN_POSTGRES_MIGRATE_ARGS[*]} "* ]]; then', wrapper)
        # The owner password never reaches the long-running services.
        compose = COMPOSE.read_text()
        for service in ("platform", "marketplace-scheduler"):
            block = service_block(compose, service) if f"\n  {service}:\n" in compose else ""
            self.assertNotIn("PLATFORM_POSTGRES_OWNER_PASSWORD", block, service)
            self.assertNotIn("DB_MIGRATIONS_PASSWORD", block, service)

    def test_deploy_wrapper_runs_the_before_and_after_hooks_around_the_postgres_migration(self) -> None:
        script = (
            'deploy_test() { _oteryn_before_platform_migrate() { echo BEFORE; }; '
            '_oteryn_after_platform_migrate() { echo AFTER; }; OTERYN_MIGRATION_PREPARED=1; '
            'docker compose -f x.yml "${OTERYN_POSTGRES_MIGRATE_ARGS[@]}"; }; deploy_test'
        )
        with tempfile.TemporaryDirectory() as td:
            env, log = fake_docker_env(Path(td))
            result = subprocess.run(
                ["bash", "-c", f'source "$1"; {script}', "deploy.sh", str(LIB)],
                text=True, capture_output=True, check=False, env={**env, "HOME": os.environ.get("HOME", "/tmp")},
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.split(), ["AFTER"])
            self.assertIn("--database=pgsql_migrations", log.read_text())

    def test_pre_migration_dump_is_a_validated_custom_archive_by_the_dump_role(self) -> None:
        dump = self.body(self.lib, "oteryn_postgres_dump_platform")
        self.assertIn("IFS= read -r PGPASSWORD", dump)
        self.assertNotRegex(dump, r"-e PGPASSWORD")
        self.assertIn("pg_dump --format=custom --no-password", dump)
        self.assertIn("user=oteryn_platform_dump sslmode=verify-full sslrootcert=/run/oteryn-postgres-tls/ca.crt", dump)
        self.assertIn("pg_restore --list", dump)
        self.assertIn("umask 077", dump)
        self.assertLess(dump.index("pg_restore --list"), dump.index('mv -- "$output.tmp" "$output"'))
        before = self.body(self.lib, "_oteryn_before_platform_migrate")
        self.assertIn('backup_file="$backup_dir/platform.dump"', before)
        self.assertIn("printf 'BACKUP_DB_ENGINE=postgres\\n'", before)
        self.assertLess(before.index("_oteryn_quiesce_platform_db_consumers"), before.index("oteryn_postgres_dump_platform"))

    def test_dump_helper_keeps_only_an_archive_that_pg_restore_can_list(self) -> None:
        dump_password = FAKE["PLATFORM_POSTGRES_DUMP_PASSWORD"]
        with tempfile.TemporaryDirectory() as td:
            env, log = fake_docker_env(Path(td))
            out = Path(td) / "platform.dump"
            result = run_lib(f'oteryn_postgres_dump_platform docker compose "{out}"', {**env, **FAKE})
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(out.read_text(), "PGDMP-archive")
            self.assertEqual(stat.S_IMODE(out.stat().st_mode), 0o600)
            self.assertFalse(Path(f"{out}.tmp").exists())
            self.assertNotIn(dump_password, log.read_text())
            # A failed listing discards the archive.
            failing = Path(td) / "docker"
            failing.write_text(failing.read_text().replace('*"pg_restore --list"*) cat >/dev/null;;', '*"pg_restore --list"*) exit 1;;'))
            out.unlink()
            result = run_lib(f'oteryn_postgres_dump_platform docker compose "{out}"', {**env, **FAKE})
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(out.exists())
            self.assertFalse(Path(f"{out}.tmp").exists())

    def test_restore_drops_recreates_and_restores_as_the_owner_without_a_password(self) -> None:
        restore = self.body(self.lib, "oteryn_postgres_restore_platform")
        steps = [
            "'DROP DATABASE IF EXISTS oteryn_platform WITH (FORCE)'",
            'sh -s -- --database-only <"$roles_script"',
            "pg_restore --exit-on-error --single-transaction --no-owner --no-privileges",
            "--role=oteryn_platform_owner --dbname=oteryn_platform <\"$dump_file\"",
        ]
        positions = [restore.index(step) for step in steps]
        self.assertEqual(positions, sorted(positions))
        self.assertEqual(restore.count('sh -s -- --database-only <"$roles_script"'), 2)
        self.assertIn("exec -T -u postgres postgres", restore)
        self.assertNotIn("PASSWORD", restore)

    def test_recovery_restores_only_into_the_engine_that_wrote_the_backup(self) -> None:
        recover = RECOVER.read_text()
        self.assertIn("unset BACKUP_DB_ENGINE BACKUP_BASELINE_KIND", recover)
        self.assertIn("BACKUP_DB_ENGINE|BACKUP_BASELINE_KIND|", recover)
        self.assertIn('backup_engine="${BACKUP_DB_ENGINE:-mariadb}"', recover)
        self.assertIn('[[ "$backup_engine" == "$OTERYN_PLATFORM_DB_ENGINE" ]] || {', recover)
        self.assertIn('[[ "$backup_engine" != postgres ]] || backup_file="$backup_dir/platform.dump"', recover)
        self.assertIn(
            'oteryn_postgres_restore_platform "${compose[@]}" "$DEPLOY_DIR/postgres/init/10-platform-roles.sh" "$backup_file"',
            recover,
        )
        self.assertIn('restored_table_count="$(oteryn_postgres_relation_count "${compose[@]}")"', recover)
        # The engine check runs with the other evidence checks, before uncertainty is persisted.
        self.assertLess(recover.index('[[ "$backup_engine" == "$OTERYN_PLATFORM_DB_ENGINE" ]]'), recover.index("SCHEMA_STATE=unknown"))
        self.assertLess(recover.index("SCHEMA_STATE=unknown"), recover.index("oteryn_postgres_restore_platform"))

    def test_recovery_refuses_a_backup_of_the_other_engine_before_any_change(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            state = root / "state"
            backup = state / "backups" / ("a" * 40 + "-before-" + "b" * 40)
            backup.mkdir(parents=True)
            (backup / "platform.sql").write_text("-- dump\n")
            evidence = backup / "evidence.env"
            contract = root / "release-contract.env"
            env_file = root / "staging.env"
            env_file.write_text("PLATFORM_DB_NAME=oteryn_platform\n")
            fake_bin = root / "bin"
            fake_bin.mkdir()
            (fake_bin / "docker").write_text("#!/bin/sh\necho docker-called >&2\nexit 99\n")
            (fake_bin / "docker").chmod(0o755)
            for engine_line, contract_engine in (("BACKUP_DB_ENGINE=postgres\n", "mariadb"), ("", "postgres"), ("BACKUP_DB_ENGINE=oracle\n", "mariadb")):
                evidence.write_text(engine_line + "BACKUP_FROM_RELEASE_SHA=" + "a" * 40 + "\n")
                contract.write_text(f"PLATFORM_DB_ENGINE={contract_engine}\n")
                result = subprocess.run(
                    ["bash", str(RECOVER), str(evidence)],
                    text=True, capture_output=True, check=False,
                    env={
                        "PATH": f"{fake_bin}:{os.environ['PATH']}", "HOME": str(root),
                        "OTERYN_ENV_FILE": str(env_file), "OTERYN_STATE_DIR": str(state),
                        "OTERYN_RELEASE_CONTRACT_FILE": str(contract),
                    },
                )
                self.assertNotEqual(result.returncode, 0, engine_line)
                self.assertRegex(result.stderr, r"(taken on \w+ but the deploy engine is|unsupported database engine)", result.stderr)
                self.assertNotIn("docker-called", result.stderr)

    def test_fresh_baseline_counts_and_dumps_postgres_with_its_engine_in_the_evidence(self) -> None:
        baseline = BASELINE.read_text()
        counter = self.body(baseline, "platform_db_table_count")
        self.assertIn('oteryn_postgres_relation_count "${platform_db_compose[@]}"', counter)
        self.assertIn("information_schema.tables WHERE table_schema='$PLATFORM_DB_NAME'", counter)
        self.assertEqual(baseline.count('"$(platform_db_table_count)"'), 2)
        self.assertIn('oteryn_postgres_dump_platform "${platform_db_compose[@]}" "$backup_file" || exit 1', baseline)
        self.assertIn('[[ "$OTERYN_PLATFORM_DB_ENGINE" != postgres ]] || backup_file="$backup_dir/platform.dump"', baseline)
        self.assertLess(baseline.index("printf 'BACKUP_DB_ENGINE=postgres\\n'"), baseline.index("printf 'BACKUP_BASELINE_KIND=fresh-empty\\n'"))

    def test_marketplace_uses_the_engine_database_service_and_owner_migration(self) -> None:
        marketplace = MARKETPLACE.read_text()
        self.assertIn('[[ "$OTERYN_PLATFORM_DB_ENGINE" != postgres ]] || platform_db_service=postgres', marketplace)
        self.assertIn('for service in "$platform_db_service" platform internal-proxy gateway; do', marketplace)
        self.assertIn('[[ "$service" != postgres ]] || profile=(--profile postgres)', marketplace)
        self.assertIn('pending_count="$(oteryn_postgres_non_terminal_auction_count "${compose[@]}")"', marketplace)
        enable = marketplace.split("    enable)\n", 1)[1].split("\n        ;;\n", 1)[0]
        self.assertLess(enable.index("--force-recreate platform"), enable.index('oteryn_platform_migrate_postgres "${compose[@]}"'))
        self.assertIn("requires all five PostgreSQL secrets", marketplace)

    def test_auction_count_helper_reads_the_platform_database_as_the_local_superuser(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            env, log = fake_docker_env(Path(td))
            result = run_lib("oteryn_postgres_non_terminal_auction_count docker compose", env)
            self.assertEqual((result.returncode, result.stdout), (0, "0\n"), result.stderr)
            argv = log.read_text()
        self.assertIn("ARGV compose --profile postgres exec -T -u postgres postgres psql -X -q -t -A -v ON_ERROR_STOP=1 -d oteryn_platform -c", argv)
        self.assertIn("tablename = 'character_auctions'", argv)

    def test_health_check_requires_the_secrets_and_the_app_role_on_the_postgres_engine(self) -> None:
        health = HEALTH.read_text()
        self.assertIn('if [[ "$OTERYN_PLATFORM_DB_ENGINE" == postgres && "$postgres_secret_state" != configured ]]; then', health)
        self.assertIn("DB_CONNECTION:pgsql DB_HOST:postgres DB_USERNAME:oteryn_platform_app DB_SSLMODE:verify-full", health)
        self.assertNotIn("container_env_value platform DB_PASSWORD", health)

    def test_route_publication_stays_refused_on_the_postgres_engine(self) -> None:
        self.assertIn(
            'if [[ "$OTERYN_PLATFORM_DB_ENGINE" == postgres && "${OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED:-false}" == true ]]; then',
            self.deploy,
        )


class PreflightPostgresTest(unittest.TestCase):
    """Plan P4b: the production-target preflight validates PostgreSQL TLS, roles and a restore drill."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.preflight = PREFLIGHT.read_text()
        cls.postgres = cls.preflight.split('if [[ "$platform_db_engine" == postgres ]]; then\n    postgres_sql() {', 1)[1]
        cls.postgres = cls.postgres.split('    postgres_tls_and_roles="PASS"\nfi\n', 1)[0]

    def test_engine_is_read_from_the_release_contract_and_adds_the_postgres_service(self) -> None:
        self.assertIn(
            "platform_db_engine=\"$(sed -n 's/^PLATFORM_DB_ENGINE=//p' \"$script_dir/../release-contract.env\")\"", self.preflight
        )
        self.assertIn("services+=(postgres)", self.preflight)
        self.assertIn("healthy_services+=(postgres)", self.preflight)
        self.assertIn("unpublished_services+=(postgres)", self.preflight)
        self.assertIn("assert_named_volume postgres /var/lib/postgresql/data", self.preflight)
        # MariaDB checks stay as they were on the mariadb engine.
        self.assertIn("assert_named_volume mariadb /var/lib/mysql", self.preflight)
        self.assertIn('[[ -n "$mariadb_root_password" ]] || fail "MARIADB_ROOT_PASSWORD is required"', self.preflight)

    def test_server_tls_hba_and_role_split_are_checked_as_the_local_superuser(self) -> None:
        for marker in (
            'docker exec -u postgres "${containers[postgres]}"',
            "SHOW ssl')\" == on ]]",
            "SHOW ssl_min_protocol_version')\" == TLSv1.3 ]]",
            "SHOW password_encryption')\" == scram-sha-256 ]]",
            "pg_hba_file_rules WHERE error IS NOT NULL",
            "(type = 'hostssl' AND auth_method = 'scram-sha-256')",
            "rolreplication <> (rolname = 'oteryn_platform_backup')",
            "r.rolname <> 'oteryn_platform_owner'",
            "a.client_addr IS NOT NULL AND NOT s.ssl",
        ):
            self.assertIn(marker, self.postgres)
        self.assertNotIn("PGPASSWORD", self.postgres)
        self.assertNotIn("PLATFORM_POSTGRES_", self.postgres.replace('grep -q "^PLATFORM_POSTGRES_"', ""))

    def test_platform_probe_proves_the_app_role_tls13_and_no_create(self) -> None:
        for marker in (
            'test "$DB_CONNECTION" = pgsql',
            'test "$DB_USERNAME" = oteryn_platform_app',
            'test "$DB_SSLMODE" = verify-full',
            'test -z "${DB_MIGRATIONS_PASSWORD+set}"',
            '$row["role"] !== "oteryn_platform_app"',
            '$row["version"] !== "TLSv1.3"',
            'CREATE TABLE oteryn_preflight_privilege_probe',
            '$e->getCode() === "42501"',
        ):
            self.assertIn(marker, self.postgres)

    def test_restore_drill_cleanup_failure_fails_the_drill_instead_of_reporting_pass(self) -> None:
        for engine_marker, end_marker in (
            ("def drop_restore_database():", "def dump_command(database):"),
            ("def drop_restore_databases():", "def dump_command(database):"),
        ):
            drop = self.preflight.split(engine_marker, 1)[1].split(end_marker, 1)[0]
            self.assertNotRegex(drop, r"except subprocess\.CalledProcessError:\s*\n\s*pass")
            self.assertIn("SystemExit", drop)
            self.assertIn("may still hold Platform data", drop)
        postgres = self.preflight.split("def drop_restore_database():", 1)[1].split("def dump_command(database):", 1)[0]
        self.assertIn("FROM pg_database WHERE datname", postgres)
        mariadb = self.preflight.split("def drop_restore_databases():", 1)[1].split("def dump_command(database):", 1)[0]
        self.assertIn("information_schema.schemata", mariadb)

    def test_postgres_restore_drill_streams_into_a_scratch_database_and_always_drops_it(self) -> None:
        drill = self.preflight.split('if [[ "$run_restore_drill" == "true" && "$platform_db_engine" == postgres ]]; then', 1)[1]
        drill = drill.split('elif [[ "$run_restore_drill" == "true" ]]; then', 1)[0]
        for marker in (
            "'pg_dump', '--format=plain', '--no-owner', '--no-privileges'",
            "'--single-transaction', '-d', target",
            "DROP DATABASE IF EXISTS \"{target}\" WITH (FORCE)",
            "TEMPLATE template0",
            "ALTER SCHEMA public OWNER TO oteryn_platform_owner",
            "(b'\\\\restrict ', b'\\\\unrestrict ')",
            "finally:\n    drop_restore_database()",
        ):
            self.assertIn(marker, drill)
        self.assertNotIn("PASSWORD", drill)
        # The dump streams between processes; only the summary is written, after the drill.
        self.assertNotRegex(drill.split("elapsed_ms", 1)[0], r"(?<![A-Za-z_.])open\(")

    def test_workflow_markers_still_hold_and_the_evidence_names_the_engine(self) -> None:
        for forbidden in ("set -x", ".Config.Env", "cat deploy/synology/.env", "mariadb-dump >"):
            self.assertNotIn(forbidden, self.preflight)
        for required in ("production_environment_proven", "database_restore_drill", "remaining_public_production_gaps", "DROP DATABASE IF EXISTS", "repository-ghcr-image.sh"):
            self.assertIn(required, self.preflight)
        self.assertIn('"platform_db_engine": "$platform_db_engine",', self.preflight)
        self.assertIn('"postgres_tls_and_roles": "$postgres_tls_and_roles",', self.preflight)
        workflow = (WORKFLOWS / "synology-production-target-preflight.yml").read_text()
        self.assertNotIn("PLATFORM_POSTGRES_", workflow)


if __name__ == "__main__":
    unittest.main(verbosity=2)
