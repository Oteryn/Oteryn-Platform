#!/usr/bin/env python3
"""Clean-slate PostgreSQL bootstrap (deploy/synology/scripts/bootstrap-postgres.sh, plan P3).

Static cases need only bash. Cases marked live need the Platform dependencies (vendor/), PHP with
pdo_pgsql and a PostgreSQL server reachable through PGSQL_INTEGRATION_HOST, _PORT, _USER and
_PASSWORD (the role must create and drop databases). Each live case runs the script in --local mode
against its own scratch database and its own witness and state directories. Live cases skip when
that is missing, unless OTERYN_PG_BOOTSTRAP_REQUIRE_LIVE=1. OTERYN_PG_BOOTSTRAP_PHP selects PHP.

Run: python3 tests/ci/test_synology_postgres_bootstrap.py
"""

from __future__ import annotations

import base64
import json
import os
import secrets
import shutil
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPOSITORY_ROOT / "deploy" / "synology" / "scripts" / "bootstrap-postgres.sh"
PHP = os.environ.get("OTERYN_PG_BOOTSTRAP_PHP", "php")
REQUIRE_LIVE = os.environ.get("OTERYN_PG_BOOTSTRAP_REQUIRE_LIVE") == "1"
PG = {
    "host": os.environ.get("PGSQL_INTEGRATION_HOST", ""),
    "port": os.environ.get("PGSQL_INTEGRATION_PORT", "5432"),
    "user": os.environ.get("PGSQL_INTEGRATION_USER", ""),
    "password": os.environ.get("PGSQL_INTEGRATION_PASSWORD", ""),
}

# Runs one SQL statement per argument on a database through PDO, printing a JSON list of the
# first column of the last statement's rows.
PDO_PROGRAM = r"""
[$self, $host, $port, $user, $password, $database] = array_slice($argv, 0, 6);
$pdo = new PDO("pgsql:host={$host};port={$port};dbname={$database}", $user, $password, [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION]);
$rows = [];
foreach (array_slice($argv, 6) as $sql) {
    $statement = $pdo->query($sql);
    $rows = $statement->columnCount() > 0 ? $statement->fetchAll(PDO::FETCH_COLUMN) : [];
}
echo json_encode(array_values($rows));
"""

USER_RELATIONS_SQL = (
    "select count(*) from pg_catalog.pg_class c join pg_catalog.pg_namespace n on n.oid = c.relnamespace "
    "where c.relkind in ('r', 'p', 'v', 'm', 'S', 'f') and n.nspname not in ('pg_catalog', 'information_schema') "
    "and n.nspname not like 'pg\\_toast%' and n.nspname not like 'pg\\_temp\\_%'"
)


def platform_unavailable_reason() -> str | None:
    if not (REPOSITORY_ROOT / "vendor" / "autoload.php").is_file():
        return "vendor/ is not installed"
    if shutil.which(PHP) is None and not Path(PHP).is_file():
        return f"PHP binary {PHP} not found"
    boot = subprocess.run(
        [PHP, "artisan", "--version"], cwd=REPOSITORY_ROOT, capture_output=True, text=True, check=False,
        env={"PATH": os.environ.get("PATH", "/usr/bin:/bin"), "APP_KEY": "base64:" + "A" * 43 + "=",
             "DB_CONNECTION": "sqlite", "DB_DATABASE": ":memory:"},
    )
    if boot.returncode != 0:
        return f"{PHP} cannot boot the Platform (set OTERYN_PG_BOOTSTRAP_PHP)"
    return None


def live_unavailable_reason() -> str | None:
    reason = platform_unavailable_reason()
    if reason is not None:
        return reason
    if not (PG["host"] and PG["user"] and PG["password"]):
        return "PGSQL_INTEGRATION_HOST, _USER and _PASSWORD are not set"
    probe = subprocess.run(
        [PHP, "-r", "echo extension_loaded('pdo_pgsql') && extension_loaded('sodium') ? 'ok' : 'no';"],
        capture_output=True, text=True, check=False,
    )
    if probe.stdout.strip() != "ok":
        return "PHP lacks pdo_pgsql or sodium"
    return None


def sql(database: str, *statements: str) -> list:
    result = subprocess.run(
        [PHP, "-r", PDO_PROGRAM, PG["host"], PG["port"], PG["user"], PG["password"], database, *statements],
        capture_output=True, text=True, check=False,
    )
    if result.returncode != 0:
        raise AssertionError(f"SQL failed on {database}: {result.stdout}{result.stderr}")
    return json.loads(result.stdout)


def tree(directory: Path) -> dict[str, bytes] | None:
    if not directory.exists():
        return None
    return {entry.name: entry.read_bytes() for entry in sorted(directory.iterdir())}


class BootstrapCase(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = Path(tempfile.mkdtemp(prefix="oteryn-pg-bootstrap-"))
        self.addCleanup(shutil.rmtree, self.temp, True)
        self.state_dir = self.temp / "state"
        self.witness_dir = self.temp / "storage-volume" / "native-evidence-witness"
        self.witness_dir.parent.mkdir(mode=0o700)
        self.storage = self.temp / "laravel-storage"
        for sub in ("app", "logs", "framework/cache/data", "framework/sessions", "framework/views"):
            (self.storage / sub).mkdir(parents=True, mode=0o700)
        self.database = ""

    def base_env(self, **overrides: str) -> dict[str, str]:
        env = {
            "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
            "HOME": str(self.temp),
            "LANG": "C.UTF-8",
            "OTERYN_PG_BOOTSTRAP_STATE_DIR": str(self.state_dir),
            "OTERYN_PG_BOOTSTRAP_PHP": PHP,
            "OTERYN_PG_BOOTSTRAP_PLATFORM_ROOT": str(REPOSITORY_ROOT),
            "LARAVEL_STORAGE_PATH": str(self.storage),
            "APP_NAME": "Oteryn",
            "APP_ENV": "production",
            "APP_DEBUG": "false",
            "APP_KEY": "base64:" + base64.b64encode(secrets.token_bytes(32)).decode(),
            "APP_URL": "https://platform.oteryn.invalid",
            "LOG_CHANNEL": "stderr",
            "SESSION_DRIVER": "array",
            "SESSION_SECURE_COOKIE": "true",
            "SESSION_HTTP_ONLY": "true",
            "CACHE_STORE": "array",
            "QUEUE_CONNECTION": "sync",
            "MAIL_MAILER": "smtp",
            "MAIL_HOST": "smtp.oteryn.com",
            "MAIL_PORT": "587",
            "MAIL_FROM_ADDRESS": "noreply@oteryn.com",
            "DB_CONNECTION": "pgsql",
            "DB_HOST": PG["host"],
            "DB_PORT": PG["port"],
            "DB_DATABASE": self.database or "unused",
            "DB_USERNAME": PG["user"],
            "DB_PASSWORD": PG["password"],
            "DB_MIGRATIONS_USERNAME": PG["user"],
            "DB_MIGRATIONS_PASSWORD": PG["password"],
            "GAME_AUTH_NATIVE_ADMISSION_ENABLED": "false",
            "GAME_AUTH_NATIVE_EVIDENCE_FRESH_KEY_PURPOSE": "fresh_admission",
            "GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY": str(self.witness_dir),
            "GAME_WORLD_ID": "1",
            "GAME_WORLD_SLUG": "oteryn",
            "GAME_WORLD_NAME": "Oteryn",
            "GAME_WORLD_REGION": "EU",
            "GAME_WORLD_HOST": "game.oteryn.invalid",
            "GAME_WORLD_PORT": "7172",
            "OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED": "false",
        }
        env.update(overrides)
        return env

    def bootstrap(self, *args: str, env: dict[str, str] | None = None) -> subprocess.CompletedProcess:
        return subprocess.run(
            ["bash", str(SCRIPT), *args], env=env or self.base_env(), capture_output=True, text=True,
            check=False, timeout=900,
        )

    def assert_refused(self, result: subprocess.CompletedProcess, fragment: str) -> None:
        output = result.stdout + result.stderr
        self.assertEqual(result.returncode, 1, output)
        self.assertIn(fragment, output)


class StaticBootstrapTest(BootstrapCase):
    def test_script_parses(self) -> None:
        subprocess.run(["bash", "-n", str(SCRIPT)], check=True)
        self.assertTrue(os.access(SCRIPT, os.X_OK), "bootstrap-postgres.sh must be executable")

    def test_unknown_subcommand_and_flags_are_refused(self) -> None:
        for args in ((), ("drop",), ("run", "--force"), ("record-witness", "--with-native-admission")):
            with self.subTest(args=args):
                result = self.bootstrap(*args)
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertIn("Usage:", result.stderr)

    def test_local_mode_requires_a_state_directory(self) -> None:
        env = self.base_env()
        del env["OTERYN_PG_BOOTSTRAP_STATE_DIR"]
        self.assert_refused(self.bootstrap("run", "--local", env=env), "needs OTERYN_PG_BOOTSTRAP_STATE_DIR")

    def test_compose_mode_never_honours_the_local_test_seam(self) -> None:
        env = self.base_env(OTERYN_PG_BOOTSTRAP_LOCAL_TEST_PLANT_AFTER_MOVE="stray.floor")
        self.assert_refused(self.bootstrap("run", env=env), "--local test seam only")

    def test_run_without_a_freeze_record_is_refused(self) -> None:
        self.assert_refused(self.bootstrap("run", "--local"), "no MariaDB-era witness freeze record")
        self.assertFalse(self.witness_dir.exists())

    def test_record_witness_refuses_a_directory_without_a_store_id(self) -> None:
        self.witness_dir.mkdir(mode=0o700)
        (self.witness_dir / (secrets.token_hex(32) + ".floor")).write_text("floor\n")
        self.assert_refused(self.bootstrap("record-witness", "--local"), "carries no witness-store.id")
        self.assertFalse((self.state_dir / "witness-freeze.env").exists())

    def test_record_witness_refuses_links_and_malformed_ids(self) -> None:
        self.witness_dir.mkdir(mode=0o700)
        (self.witness_dir / "witness-store.id").write_text("not-a-store-id\n")
        self.assert_refused(self.bootstrap("record-witness", "--local"), "malformed_store_id")
        (self.witness_dir / "witness-store.id").write_text(secrets.token_hex(32) + "\n")
        (self.witness_dir / "elsewhere.floor").symlink_to(self.temp)
        self.assert_refused(self.bootstrap("record-witness", "--local"), "unsupported_entry")

    def test_record_witness_is_idempotent_and_refuses_a_different_record(self) -> None:
        self.witness_dir.mkdir(mode=0o700)
        (self.witness_dir / "witness-store.id").write_text(secrets.token_hex(32) + "\n")
        first = self.bootstrap("record-witness", "--local")
        self.assertEqual(first.returncode, 0, first.stderr)
        record = (self.state_dir / "witness-freeze.env").read_text()
        self.assertEqual(stat.S_IMODE((self.state_dir / "witness-freeze.env").stat().st_mode), 0o600)
        again = self.bootstrap("record-witness", "--local")
        self.assertEqual(again.returncode, 0, again.stderr)
        self.assertIn("unchanged", again.stdout)
        (self.witness_dir / (secrets.token_hex(32) + ".floor")).write_text("advanced\n")
        self.assert_refused(self.bootstrap("record-witness", "--local"), "a different freeze record already exists")
        self.assertEqual((self.state_dir / "witness-freeze.env").read_text(), record)

    def test_wrong_engine_is_refused_before_any_change(self) -> None:
        reason = platform_unavailable_reason()
        if reason is not None:
            self.skipTest(reason)
        env = self.base_env(DB_CONNECTION="sqlite", DB_DATABASE=":memory:")
        self.assertEqual(self.bootstrap("record-witness", "--local", env=env).returncode, 0)
        result = self.bootstrap("run", "--local", env=env)
        self.assert_refused(result, "the target is not PostgreSQL")
        self.assertIn("Nothing was changed.", result.stderr)
        self.assertFalse(self.witness_dir.exists())

    def test_state_directory_link_is_refused_before_chmod(self) -> None:
        target = self.temp / "unrelated"
        target.mkdir(mode=0o755)
        self.state_dir.symlink_to(target)
        self.assert_refused(self.bootstrap("record-witness", "--local"), "not a link")
        self.assertEqual(stat.S_IMODE(target.stat().st_mode), 0o755)

    def test_invalid_world_options_are_refused_before_any_change(self) -> None:
        reason = platform_unavailable_reason()
        if reason is not None:
            self.skipTest(reason)
        env = self.base_env(DB_CONNECTION="sqlite", DB_DATABASE=":memory:")
        self.assertEqual(self.bootstrap("record-witness", "--local", env=env).returncode, 0)
        cases = {
            "GAME_WORLD_ID": ("0", "abc", "-1", "9223372036854775808", "99999999999999999999"),
            "GAME_WORLD_PORT": ("abc", "0", "65536", "99999"),
            "GAME_WORLD_SLUG": ("Oteryn", "-oteryn", "a" * 65),
            "GAME_WORLD_NAME": ("n" * 101,),
            "GAME_WORLD_REGION": ("EU West", "r" * 33),
            "GAME_WORLD_HOST": ("https://game.oteryn.invalid", "game.oteryn.invalid/path", "-bad.invalid"),
        }
        for name, values in cases.items():
            for value in values:
                with self.subTest(name=name, value=value):
                    result = self.bootstrap("run", "--local", env=self.base_env(
                        DB_CONNECTION="sqlite", DB_DATABASE=":memory:", **{name: value}))
                    self.assert_refused(result, f"{name} must")
                    self.assertFalse(self.witness_dir.exists())

    def test_whitespace_only_world_options_are_refused_like_the_command_trims_them(self) -> None:
        reason = platform_unavailable_reason()
        if reason is not None:
            self.skipTest(reason)
        self.assertEqual(self.bootstrap("record-witness", "--local", env=self.base_env(
            DB_CONNECTION="sqlite", DB_DATABASE=":memory:")).returncode, 0)
        for name in ("GAME_WORLD_ID", "GAME_WORLD_SLUG", "GAME_WORLD_NAME", "GAME_WORLD_REGION",
                     "GAME_WORLD_HOST", "GAME_WORLD_PORT"):
            with self.subTest(name=name):
                result = self.bootstrap("run", "--local", env=self.base_env(
                    DB_CONNECTION="sqlite", DB_DATABASE=":memory:", **{name: "   "}))
                self.assert_refused(result, f"{name} is not set")
                self.assertFalse(self.witness_dir.exists())

    def test_stale_state_record_path_is_refused_before_any_change(self) -> None:
        reason = platform_unavailable_reason()
        if reason is not None:
            self.skipTest(reason)
        env = self.base_env(DB_CONNECTION="sqlite", DB_DATABASE=":memory:")
        self.assertEqual(self.bootstrap("record-witness", "--local", env=env).returncode, 0)
        for name in ("witness-moved-aside.env.tmp", "bootstrap-complete.env.tmp"):
            with self.subTest(name=name):
                stale = self.state_dir / name
                stale.mkdir()
                self.addCleanup(stale.rmdir)
                self.assert_refused(self.bootstrap("run", "--local", env=env), f"{name} already exists")
                self.assertFalse(self.witness_dir.exists())
                stale.rmdir()

    def test_refused_run_does_not_create_or_chmod_the_state_directory(self) -> None:
        script = SCRIPT.read_text()
        validate = script.index('must be an owner-only directory, not a link.')
        prepare = script.index('prepare_state_dir() {')
        self.assertLess(validate, prepare)
        head = script[:prepare]
        self.assertNotIn('mkdir -p', head)
        self.assertNotIn('chmod 700', head)
        self.assertNotIn('mkdir -- "$lock_dir"', head)
        # The first call happens after the last refusal block of run, and before the freeze record write.
        run_refusals = script.index('Nothing was changed.')
        self.assertLess(run_refusals, script.index('\nprepare_state_dir\n', run_refusals))
        record = script[script.index('record_witness() {'):script.index('if [[ "$subcommand" == record-witness ]]')]
        self.assertLess(record.index('prepare_state_dir'), record.index('> "$freeze_file.tmp"'))

    def test_missing_state_directory_stays_absent_when_the_invocation_is_refused(self) -> None:
        reason = platform_unavailable_reason()
        if reason is not None:
            self.skipTest(reason)
        self.assertFalse(self.state_dir.exists())
        env = self.base_env(DB_CONNECTION="sqlite", DB_DATABASE=":memory:")
        self.assert_refused(self.bootstrap("run", "--local", env=env), "no MariaDB-era witness freeze record")
        self.assertFalse(self.state_dir.exists())

    def test_native_completion_requires_a_successful_nonempty_witness_inspection(self) -> None:
        script = SCRIPT.read_text()
        tail = script[script.index('final_witness="$(witness inspect)"'):]
        self.assertIn('final_witness="$(witness inspect)" || failed', tail)
        self.assertNotIn('witness inspect)" || true', tail)
        self.assertIn('final_store_id" =~ ^[0-9a-f]{64}$', tail)
        self.assertIn('witness_directory "$final_witness")" == "$frozen_dir"', tail)
        self.assertLess(tail.index('=~ ^[0-9a-f]{64}$'), tail.index('bootstrap-complete.env') if 'bootstrap-complete.env' in tail else len(tail))

    def test_record_writes_after_the_first_change_exit_with_the_failed_status(self) -> None:
        script = SCRIPT.read_text()
        self.assertRegex(script, r'> "\$moved_file\.tmp" \\\n\s+&& chmod 600 "\$moved_file\.tmp" \\\n\s+&& mv -- "\$moved_file\.tmp" "\$moved_file" \\\n\s+\|\| failed ')
        self.assertRegex(script, r'> "\$complete_file\.tmp" \\\n\s+&& chmod 600 "\$complete_file\.tmp" \\\n\s+&& mv -- "\$complete_file\.tmp" "\$complete_file" \\\n\s+\|\| failed ')


class LiveBootstrapCase(BootstrapCase):
    def setUp(self) -> None:
        reason = live_unavailable_reason()
        if reason is not None:
            if REQUIRE_LIVE:
                self.fail(f"live PostgreSQL bootstrap test required but unavailable: {reason}")
            self.skipTest(reason)
        super().setUp()
        self.database = "oteryn_pg_bootstrap_" + secrets.token_hex(6)
        sql("postgres", f'create database "{self.database}"')
        self.addCleanup(sql, "postgres", f'drop database if exists "{self.database}" with (force)')

    def relations(self) -> int:
        return int(sql(self.database, USER_RELATIONS_SQL)[0])

    def mariadb_era_witness(self) -> str:
        store_id = secrets.token_hex(32)
        self.witness_dir.mkdir(mode=0o700)
        (self.witness_dir / "witness-store.id").write_text(store_id + "\n")
        (self.witness_dir / "witness-store.lock").write_bytes(b"")
        floor = secrets.token_hex(32)
        (self.witness_dir / f"{floor}.floor").write_text('{"sequence":7}\n')
        (self.witness_dir / f"{floor}.floor.lock").write_bytes(b"")
        return store_id

    def native_admission_env(self) -> dict[str, str]:
        keys = self.temp / "native-admission"
        keys.mkdir(mode=0o700)
        key_file = keys / "current.key"
        key_file.write_text(base64.urlsafe_b64encode(secrets.token_bytes(32)).decode().rstrip("=") + "\n")
        key_file.chmod(0o600)
        return self.base_env(
            APP_ENV="preproduction",
            GAME_AUTH_NATIVE_ADMISSION_ENABLED="true",
            GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_FILE=str(key_file),
            GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_ID="oteryn-admission-ci-" + secrets.token_hex(3),
        )

    def assert_unchanged(self, relations: int, witness: dict[str, bytes] | None) -> None:
        self.assertEqual(self.relations(), relations)
        self.assertEqual(tree(self.witness_dir), witness)


class LiveBootstrapTest(LiveBootstrapCase):
    def test_bootstrap_without_native_admission_then_rerun_is_refused(self) -> None:
        env = self.base_env()
        self.assertEqual(self.bootstrap("record-witness", "--local", env=env).returncode, 0)
        result = self.bootstrap("run", "--local", env=env)
        output = result.stdout + result.stderr
        self.assertEqual(result.returncode, 0, output)
        self.assertIn("bootstrap_result=complete", result.stdout)
        self.assertIn("Production configuration invariant checks passed.", output)
        self.assertRegex(result.stdout, r"oauth_client_id=[A-Za-z0-9-]+\n")
        self.assertRegex(result.stdout, r"oauth_rust_client_id=[A-Za-z0-9-]+\n")
        self.assertEqual(tree(self.witness_dir), {})
        self.assertEqual(stat.S_IMODE(self.witness_dir.stat().st_mode), 0o700)
        self.assertEqual(sql(self.database, "select count(*) from native_game_evidence_witness_stores"), [0])
        self.assertEqual(sql(self.database, "select count(*) from native_game_signing_trust_profiles"), [0])
        self.assertEqual(sql(self.database, "select slug from game_worlds where id = 1"), ["oteryn"])
        self.assertTrue((self.storage / "oauth-private.key").is_file())
        relations = self.relations()

        self.assert_refused(self.bootstrap("run", "--local", env=env), "already records a completed bootstrap")
        self.assert_unchanged(relations, {})

        # A fresh state directory does not help: the target database itself is not empty.
        env["OTERYN_PG_BOOTSTRAP_STATE_DIR"] = str(self.temp / "state-again")
        self.assertEqual(self.bootstrap("record-witness", "--local", env=env).returncode, 0)
        rerun = self.bootstrap("run", "--local", env=env)
        self.assert_refused(rerun, "the target database is not empty")
        self.assertIn("Nothing was changed.", rerun.stderr)
        self.assert_unchanged(relations, {})

    def test_bootstrap_with_native_admission_moves_the_mariadb_witness_aside(self) -> None:
        env = self.native_admission_env()
        mariadb_store_id = self.mariadb_era_witness()
        mariadb_tree = tree(self.witness_dir)
        self.assertEqual(self.bootstrap("record-witness", "--local", env=env).returncode, 0)

        refused = self.bootstrap("run", "--local", env=env)
        self.assert_refused(refused, "Pass --with-native-admission only at that boundary")
        self.assert_unchanged(0, mariadb_tree)

        result = self.bootstrap("run", "--local", "--with-native-admission", env=env)
        output = result.stdout + result.stderr
        self.assertEqual(result.returncode, 0, output)
        self.assertIn("production:verify-configuration not run", output)
        self.assertIn("Native admission signing self-check passed", output)
        moved = [p for p in self.witness_dir.parent.iterdir() if p.name.startswith(self.witness_dir.name + ".mariadb-era-")]
        self.assertEqual(len(moved), 1)
        self.assertIn(f"witness_moved_aside_to={moved[0]}", result.stdout)
        self.assertEqual(tree(moved[0]), mariadb_tree)
        new_store_id = (self.witness_dir / "witness-store.id").read_text().strip()
        self.assertRegex(new_store_id, r"^[0-9a-f]{64}$")
        self.assertNotEqual(new_store_id, mariadb_store_id)
        self.assertEqual(sql(self.database, "select store_id from native_game_evidence_witness_stores"), [new_store_id])
        self.assertGreaterEqual(sql(self.database, "select count(*) from native_game_signing_trust_profiles")[0], 1)
        relations = self.relations()
        witness = tree(self.witness_dir)

        # After the first witnessed write: no re-run, whatever the state directory says.
        self.assert_refused(self.bootstrap("run", "--local", "--with-native-admission", env=env),
                            "already records a completed bootstrap")
        (self.state_dir / "bootstrap-complete.env").unlink()
        rerun = self.bootstrap("run", "--local", "--with-native-admission", env=env)
        self.assert_refused(rerun, "a PostgreSQL witness wrote to it")
        self.assertIn("already has a native evidence witness-store binding", rerun.stderr)
        self.assertIn("already has native signing trust rows", rerun.stderr)
        self.assert_unchanged(relations, witness)
        self.assertEqual(tree(moved[0]), mariadb_tree)


class LiveRefusalTest(LiveBootstrapCase):
    def test_existing_witness_binding_row_is_refused(self) -> None:
        sql(self.database, "create table native_game_evidence_witness_stores (id smallint primary key, store_id char(64))",
            f"insert into native_game_evidence_witness_stores values (1, '{secrets.token_hex(32)}')")
        self.assertEqual(self.bootstrap("record-witness", "--local").returncode, 0)
        self.assert_refused(self.bootstrap("run", "--local"), "already has a native evidence witness-store binding")
        self.assert_unchanged(1, None)

    def test_existing_trust_row_is_refused(self) -> None:
        sql(self.database, "create table native_game_signing_trust_profiles (id bigint primary key)",
            "insert into native_game_signing_trust_profiles values (1)")
        self.assertEqual(self.bootstrap("record-witness", "--local").returncode, 0)
        self.assert_refused(self.bootstrap("run", "--local"), "already has native signing trust rows")
        self.assert_unchanged(1, None)

    def test_foreign_store_id_is_refused(self) -> None:
        self.mariadb_era_witness()
        self.assertEqual(self.bootstrap("record-witness", "--local").returncode, 0)
        (self.witness_dir / "witness-store.id").write_text(secrets.token_hex(32) + "\n")
        witness = tree(self.witness_dir)
        self.assert_refused(self.bootstrap("run", "--local"), "not the MariaDB-era id recorded at the freeze")
        self.assert_unchanged(0, witness)

    def test_missing_profile_directory_is_refused(self) -> None:
        # A trust publication whose database transaction rolled back can leave a witness floor,
        # with or without a new store id, and no database row.
        self.assertEqual(self.bootstrap("record-witness", "--local").returncode, 0)
        self.witness_dir.mkdir(mode=0o700)
        (self.witness_dir / (secrets.token_hex(32) + ".floor")).write_text('{"sequence":1}\n')
        witness = tree(self.witness_dir)
        self.assert_refused(self.bootstrap("run", "--local"), "carries no witness-store.id")
        self.assert_unchanged(0, witness)
        (self.witness_dir / "witness-store.id").write_text(secrets.token_hex(32) + "\n")
        witness = tree(self.witness_dir)
        self.assert_refused(self.bootstrap("run", "--local"), "not the MariaDB-era id recorded at the freeze")
        self.assert_unchanged(0, witness)

    def test_recorded_directory_advanced_after_the_freeze_is_refused(self) -> None:
        self.mariadb_era_witness()
        self.assertEqual(self.bootstrap("record-witness", "--local").returncode, 0)
        (self.witness_dir / (secrets.token_hex(32) + ".floor")).write_text('{"sequence":1}\n')
        witness = tree(self.witness_dir)
        self.assert_refused(self.bootstrap("run", "--local"), "its files changed since the freeze")
        self.assert_unchanged(0, witness)

    def test_recorded_directory_removed_without_the_bootstrap_is_refused(self) -> None:
        self.mariadb_era_witness()
        self.assertEqual(self.bootstrap("record-witness", "--local").returncode, 0)
        shutil.rmtree(self.witness_dir)
        self.assert_refused(self.bootstrap("run", "--local"), "this bootstrap did not move it aside")
        self.assert_unchanged(0, None)

    def test_new_directory_not_empty_after_the_move_stops_before_the_schema(self) -> None:
        self.mariadb_era_witness()
        mariadb_tree = tree(self.witness_dir)
        self.assertEqual(self.bootstrap("record-witness", "--local").returncode, 0)
        env = self.base_env(OTERYN_PG_BOOTSTRAP_LOCAL_TEST_PLANT_AFTER_MOVE="stray.floor")
        result = self.bootstrap("run", "--local", env=env)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("new_directory_not_empty", result.stderr)
        self.assertEqual(self.relations(), 0)
        moved = [p for p in self.witness_dir.parent.iterdir() if p.name.startswith(self.witness_dir.name + ".mariadb-era-")]
        self.assertEqual(len(moved), 1)
        self.assertEqual(tree(moved[0]), mariadb_tree)
        # The MariaDB-era directory is gone from its place, so a second run cannot move anything.
        self.assert_refused(self.bootstrap("run", "--local"), "a PostgreSQL witness wrote to it")
        self.assertEqual(self.relations(), 0)

    def test_retry_requires_the_preserved_mariadb_era_witness_to_be_intact(self) -> None:
        self.mariadb_era_witness()
        self.assertEqual(self.bootstrap("record-witness", "--local").returncode, 0)
        env = self.base_env(OTERYN_PG_BOOTSTRAP_LOCAL_TEST_PLANT_AFTER_MOVE="stray.floor")
        self.assertEqual(self.bootstrap("run", "--local", env=env).returncode, 2)
        (self.witness_dir / "stray.floor").unlink()
        aside = next(p for p in self.witness_dir.parent.iterdir() if p.name.startswith(self.witness_dir.name + ".mariadb-era-"))
        victim = next(p for p in sorted(aside.iterdir()) if p.name != "witness-store.id")
        original = victim.read_bytes()
        victim.write_bytes(original + b"x")
        self.assert_refused(self.bootstrap("run", "--local"), "is missing, replaced or modified")
        victim.write_bytes(original)
        moved_name = aside.name
        parked = aside.with_name("parked")
        aside.rename(parked)
        self.assert_refused(self.bootstrap("run", "--local"), "aside_missing")
        aside.symlink_to(parked)
        self.assert_refused(self.bootstrap("run", "--local"), "aside_missing")
        aside.unlink()
        parked.rename(aside)
        self.assertEqual(aside.name, moved_name)

    def test_retry_check_is_content_based_not_a_name_prefix(self) -> None:
        script = SCRIPT.read_text()
        self.assertIn("verify-aside)", script)
        branch = script[script.index('moved_to="$(read_record_key "$moved_file" MOVED_ASIDE_TO)"'):]
        branch = branch[:branch.index("witness_action=prepare-empty")]
        self.assertIn('witness verify-aside "$moved_to" "$frozen_manifest" "$frozen_store_id"', branch)
        self.assertIn('WITNESS_STORE_ID)" != "$frozen_store_id"', branch)
        program = script[script.index("verify-aside)"):script.index("prepare-empty)")]
        for needle in ('[ ! -L "$dir" ]', '[ -O "$dir" ]', '"$manifest" = "$expected_manifest"', '"$store_id" = "$expected_store_id"'):
            self.assertIn(needle, program)


if __name__ == "__main__":
    unittest.main(verbosity=2)
