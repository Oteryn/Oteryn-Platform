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

    def test_operational_mode_never_publishes_trust(self) -> None:
        text = SCRIPT.read_text()
        self.assertIn('"$local_mode" != true && ( "$native_admission" == true || "$with_native_admission" == true )', text)
        self.assertIn("P6a safeguards", text)

    def test_witness_directory_is_never_enumerated_by_parsing_ls(self) -> None:
        self.assertNotRegex(SCRIPT.read_text(), r"\$\(\s*ls\b|`ls\b|\| *ls\b")

    def test_refusal_checks_are_repeated_under_the_lock(self) -> None:
        text = SCRIPT.read_text()
        tail = text[text.index("\nrun_refusal_checks\n"):]
        position = 0
        for marker in ("\nrun_refusal_checks\n", "\nprepare_state_dir\n", "\nrun_refusal_checks\n", "\nrestrict_state_dir\n"):
            found = tail.index(marker, position)
            position = found + len(marker) - 1
        self.assertLess(tail.index("restrict_state_dir\n"), tail.index("Step 1/8"))

    def test_newline_only_file_name_is_not_dropped(self) -> None:
        self.witness_dir.mkdir(mode=0o700)
        (self.witness_dir / "\n").write_text("x")
        result = self.bootstrap("record-witness", "--local")
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("unsupported_name", result.stdout + result.stderr)
        self.assertFalse((self.state_dir / "witness-freeze.env").exists())

    def test_compose_one_offs_never_start_the_fastcgi_pool(self) -> None:
        # The one-off container runs the image entrypoint as PID 1; with FastCGI enabled it would start
        # PHP-FPM and map the command's normal exit to status 9 (platform-entrypoint.sh run_supervised).
        text = SCRIPT.read_text()
        oneoff = text[text.index("platform_oneoff() {"):text.index("platform_exec() {")]
        self.assertIn("-e OTERYN_PLATFORM_FASTCGI_ENABLED=false", oneoff)
        self.assertIn("run --rm --no-deps -T --entrypoint sh platform", text)
        runs = [line for line in text.splitlines() if '"${compose[@]}" run' in line]
        self.assertEqual(len(runs), 2, runs)

    def test_operational_world_host_follows_the_deploy_policy(self) -> None:
        # deploy.sh accepts GAME_WORLD_HOST only as loopback or RFC1918 IPv4; the operational bootstrap must refuse
        # anything else (DNS name, IPv6, public IPv4, a second line) before any change. --local keeps the broad rule.
        text = SCRIPT.read_text()
        self.assertIn('"$local_mode" != true && -n "${GAME_WORLD_HOST:-}"', text)
        condition = next(line for line in text.splitlines() if line.lstrip().startswith('{ [[ "$GAME_WORLD_HOST" =~ ^[0-9.]+$ ]]'))
        script = f'SCRIPT_DIR={SCRIPT.parent}; ok() {{ GAME_WORLD_HOST="$1"; {condition.strip().rstrip("\\")} || return 1; }}; ok "$1"'
        expectations = {
            "127.0.0.1": True, "10.1.2.3": True, "172.16.0.1": True, "192.168.1.5": True,
            "game.oteryn.invalid": False, "::1": False, "abc:def": False, "8.8.8.8": False,
            "172.32.0.1": False, "10.0.0.1\nevil": False, "01.2.3.4": False, "1.2.3": False,
        }
        for host, allowed in expectations.items():
            with self.subTest(host=host):
                result = subprocess.run(["bash", "-c", script, "x", host], capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode == 0, allowed, result.stderr)

    def test_record_witness_repeats_its_checks_under_the_lock(self) -> None:
        text = SCRIPT.read_text()
        body = text[text.index("\nrecord_witness() {"):text.index('if [[ "$subcommand" == record-witness ]]')]
        position = 0
        for marker in ("record_witness_checks\n", "prepare_state_dir\n", "record_witness_checks\n", "restrict_state_dir\n", 'FROZEN_AT=%s'):
            position = body.index(marker, position) + len(marker)

    def test_target_wide_lock_refuses_a_second_state_directory(self) -> None:
        # The state-directory lock does not cover two runs that use different state directories on the same
        # database and witness directory: the lock is also keyed on the witness directory itself.
        self.witness_dir.mkdir(mode=0o700)
        (self.witness_dir / "witness-store.id").write_text(secrets.token_hex(32) + "\n")
        lock = Path(str(self.witness_dir) + ".bootstrap-lock")
        lock.mkdir()
        result = self.bootstrap("record-witness", "--local")
        self.assert_refused(result, "target-wide lock")
        self.assertIn("Nothing was changed", result.stderr)
        self.assertFalse((self.state_dir / "witness-freeze.env").exists())
        self.assertFalse((self.state_dir / "run.lock").exists())
        lock.rmdir()
        self.assertEqual(self.bootstrap("record-witness", "--local").returncode, 0)
        self.assertFalse(lock.exists())
        self.assertFalse((self.state_dir / "run.lock").exists())

    def test_group_writable_state_directory_is_refused_before_any_record_is_read(self) -> None:
        self.witness_dir.mkdir(mode=0o700)
        (self.witness_dir / "witness-store.id").write_text(secrets.token_hex(32) + "\n")
        self.state_dir.mkdir(mode=0o700)
        self.assertEqual(self.bootstrap("record-witness", "--local").returncode, 0)
        self.state_dir.chmod(0o770)
        # The record is unchanged, so the idempotent path would succeed without the early check.
        self.assert_refused(self.bootstrap("record-witness", "--local"), "not group- or world-writable")
        self.state_dir.chmod(0o707)
        self.assert_refused(self.bootstrap("record-witness", "--local"), "not group- or world-writable")
        self.state_dir.chmod(0o700)

    def test_a_refused_run_never_removes_a_lock_it_did_not_take(self) -> None:
        self.witness_dir.mkdir(mode=0o700)
        (self.witness_dir / "witness-store.id").write_text(secrets.token_hex(32) + "\n")
        self.state_dir.mkdir(mode=0o700)
        held = self.state_dir / "run.lock"
        held.mkdir()
        self.assert_refused(self.bootstrap("record-witness", "--local"), "run.lock")
        self.assertTrue(held.is_dir())
        held.rmdir()
        target = Path(str(self.witness_dir) + ".bootstrap-lock")
        target.mkdir()
        held.mkdir()
        self.assert_refused(self.bootstrap("record-witness", "--local"), "target-wide lock")
        self.assertTrue(held.is_dir() and target.is_dir())
        held.rmdir()
        target.rmdir()

    def test_state_directory_mode_is_restored_by_a_refusal_before_the_first_change(self) -> None:
        text = SCRIPT.read_text()
        restrict = text[text.index("restrict_state_dir() {") : text.index("commit_state_dir() {")]
        self.assertIn('state_dir_original_mode="$mode"', restrict)
        release = text[text.index("release_locks() {") : text.index("prepare_state_dir() {")]
        self.assertIn('chmod "$state_dir_original_mode"', release)
        # failed() (exit 2, a change began) keeps the mode; so does every record write and the witness reset.
        self.assertLess(text.index("failed() {"), text.index("    commit_state_dir\n    printf 'Bootstrap failed"))
        run = text[text.index("# --- run: steps 1 to 8"):]
        self.assertLess(run.index("witness move-aside"), run.index("        commit_state_dir\n        printf 'MOVED_ASIDE_TO"))
        self.assertLess(run.index("    commit_state_dir\n    witness_output=\"$(witness prepare-empty)"), run.index("Step 2/8"))
        # The refusal for a move that moved nothing is reached before any commit, so it restores the mode.
        move_failed = run.index("the witness directory could not be moved aside")
        self.assertLess(run.index("witness move-aside"), move_failed)
        self.assertNotIn("commit_state_dir", run[run.index('if [[ -z "$moved_to" && "$witness_action"') : move_failed])
        self.assertIn("    commit_state_dir\n    printf '%s\\nFROZEN_AT=%s", text)

    def test_server_major_version_is_validated_before_the_first_change(self) -> None:
        text = SCRIPT.read_text()
        check = text.index("server_version_num")  # emitted by the probe
        reasons = text.index('server_version="$(value_of server_version_num')
        self.assertGreater(reasons, check)
        self.assertIn("^17[0-9]{4}$", text)
        self.assertIn("^1[67][0-9]{4}$", text)
        self.assertLess(reasons, text.index("\nprepare_state_dir\n"))
        self.assertIn('"$(value_of runtime_connection "$probe_output")" == pgsql', text)

    def test_held_target_lock_creates_no_state_directory(self) -> None:
        self.witness_dir.mkdir(mode=0o700)
        (self.witness_dir / "witness-store.id").write_text(secrets.token_hex(32) + "\n")
        lock = Path(str(self.witness_dir) + ".bootstrap-lock")
        lock.mkdir()
        self.assertFalse(self.state_dir.exists())
        self.assert_refused(self.bootstrap("record-witness", "--local"), "target-wide lock")
        self.assertFalse(self.state_dir.exists())
        lock.rmdir()

    def test_refusal_after_the_locks_removes_a_state_directory_it_created(self) -> None:
        # A witness directory holding a floor but no store id is refused by the checks that run again under the lock
        # only when it changes in between; here the first pass refuses, and nothing may be created either way.
        self.witness_dir.mkdir(mode=0o700)
        (self.witness_dir / (secrets.token_hex(32) + ".floor")).write_text("floor\n")
        self.assert_refused(self.bootstrap("record-witness", "--local"), "carries no witness-store.id")
        self.assertFalse(self.state_dir.exists())
        self.assertFalse(Path(str(self.witness_dir) + ".bootstrap-lock").exists())

    def test_world_host_is_validated_by_the_commands_own_filter(self) -> None:
        text = SCRIPT.read_text()
        self.assertIn("FILTER_VALIDATE_IP", text)
        self.assertIn("FILTER_VALIDATE_DOMAIN", text)
        self.assertNotIn("host_label", text)

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

    def test_witness_directory_of_another_owner_is_refused_in_the_inspection(self) -> None:
        text = SCRIPT.read_text()
        program = text[text.index("WITNESS_PROGRAM='") : text.index("witness() {")]
        inspect = program[program.index("inspect() {") : program.index("report() {")]
        self.assertIn('[ -O "$dir" ] || fail owner', inspect)
        # The predicate verify-aside applies later is the one the inspection applies now.
        self.assertIn('[ -O "$dir" ] || fail aside_owner', program)
        self.assertLess(inspect.index("fail owner"), inspect.index('for entry in'))
        self.assertIn("witness_parent_writable", program)
        run = text[text.index("# --- run: steps 1 to 8") :]
        self.assertIn('"$witness_parent_writable" == true', text)
        self.assertLess(text.index('"$witness_parent_writable" == true'), text.index("\nprepare_state_dir\n"))

    @unittest.skipUnless(os.geteuid() == 0, "needs root to hand a directory to another user")
    def test_foreign_owned_witness_directory_is_refused_by_record_witness(self) -> None:
        self.witness_dir.mkdir(mode=0o700)
        (self.witness_dir / "witness-store.id").write_text(secrets.token_hex(32) + "\n")
        os.chown(self.witness_dir, 65534, 65534)
        self.assert_refused(self.bootstrap("record-witness", "--local"), "cannot be used (owner)")
        self.assertFalse(self.state_dir.exists())

    def test_inspection_reports_a_writable_witness_parent(self) -> None:
        self.witness_dir.mkdir(mode=0o700)
        (self.witness_dir / "witness-store.id").write_text(secrets.token_hex(32) + "\n")
        result = self.bootstrap("record-witness", "--local")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("witness_parent_writable=true", self.witness_output())

    def witness_output(self) -> str:
        env = {**self.base_env(), "GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY": str(self.witness_dir)}
        program = SCRIPT.read_text()
        program = program[program.index("WITNESS_PROGRAM='") + len("WITNESS_PROGRAM='") :]
        program = program[: program.index("\n'\n")]
        return subprocess.run(["sh", "-c", program, "witness", "inspect"], env=env, capture_output=True, text=True).stdout

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
            "GAME_WORLD_HOST": ("https://game.oteryn.invalid", "game.oteryn.invalid/path", "-bad.invalid",
                                 "abc:def", "dead:beef", "1:2:3:4:5:6:7:8:9"),
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
        self.assertRegex(script, r'> "\$moved_file\.tmp" \\\n\s+&& chmod 600 "\$moved_file\.tmp" \\\n\s+&& mv -- "\$moved_file\.tmp" "\$moved_file" \\\n\s+\|\| \{ rm -f -- "\$moved_file\.tmp"; failed ')
        self.assertRegex(script, r'> "\$complete_file\.tmp" \\\n\s+&& chmod 600 "\$complete_file\.tmp" \\\n\s+&& mv -- "\$complete_file\.tmp" "\$complete_file" \\\n\s+\|\| \{ rm -f -- "\$complete_file\.tmp"; failed ')

    def test_failed_move_record_is_recoverable_by_adopting_the_intact_aside(self) -> None:
        script = SCRIPT.read_text()
        self.assertIn("find-aside)", script)
        self.assertIn('witness find-aside "$frozen_manifest" "$frozen_store_id"', script)
        self.assertIn('witness_aside_count "$aside_found")" == 1', script)
        adopt = script[script.index('if [[ -n "$adopt_aside_to" ]]'):]
        adopt = adopt[: adopt.index("echo \"Adopted")]
        self.assertLess(script.index("\nprepare_state_dir\n", script.index("Nothing was changed.")), script.index('if [[ -n "$adopt_aside_to" ]]'))
        self.assertIn("rm -f -- \"$moved_file.tmp\"", adopt)

    def test_record_write_failures_after_the_first_mutation_exit_2_and_clean_their_tmp(self) -> None:
        script = SCRIPT.read_text()
        record = script[script.index("record_witness() {") : script.index('if [[ "$subcommand" == record-witness ]]')]
        self.assertIn('rm -f -- "$freeze_file.tmp"; failed', record)
        self.assertNotIn('refuse "the freeze record', record)
        self.assertLess(script.index("\nfailed() {"), script.index("record_witness() {"))
        self.assertIn('rm -f -- "$complete_file.tmp"; failed', script)
        self.assertIn('rm -f -- "$moved_file.tmp"; failed', script)

    def compose_mode_owner_credentials(self, env_lines: list[str]) -> tuple[dict[str, str], str]:
        # Compose mode with a docker stub that records the migration owner variables it inherits (what
        # `-e NAME` would hand the one-off container) and then fails, so the invocation stops at a refusal.
        stub_dir = self.temp / "stub-bin"
        stub_dir.mkdir()
        record = self.temp / "docker-env"
        stub = stub_dir / "docker"
        stub.write_text(
            "#!/usr/bin/env bash\n"
            f'{{ printf "USERNAME=%s\\n" "${{DB_MIGRATIONS_USERNAME-<unset>}}"; '
            f'printf "PASSWORD=%s\\n" "${{DB_MIGRATIONS_PASSWORD-<unset>}}"; }} > "{record}"\n'
            "exit 1\n"
        )
        stub.chmod(0o700)
        env_file = self.temp / "staging.env"
        env_file.write_text("\n".join(["APP_ENV=production", *env_lines]) + "\n")
        env = self.base_env(PATH=f"{stub_dir}:{os.environ.get('PATH', '/usr/bin:/bin')}", OTERYN_ENV_FILE=str(env_file))
        del env["DB_MIGRATIONS_USERNAME"]
        del env["DB_MIGRATIONS_PASSWORD"]
        result = self.bootstrap("record-witness", env=env)
        output = result.stdout + result.stderr
        self.assertEqual(result.returncode, 1, output)
        self.assertTrue(record.exists(), output)
        seen = dict(line.split("=", 1) for line in record.read_text().splitlines())
        return seen, output

    def test_compose_mode_derives_the_owner_credentials_from_the_owner_password(self) -> None:
        # The staging environment file carries PLATFORM_POSTGRES_OWNER_PASSWORD only; lib.sh
        # oteryn_platform_migrate_postgres maps it to the owner role, and the bootstrap must do the same.
        password = "owner-" + secrets.token_hex(8)
        seen, output = self.compose_mode_owner_credentials([f"PLATFORM_POSTGRES_OWNER_PASSWORD={password}"])
        self.assertEqual(seen, {"USERNAME": "oteryn_platform_owner", "PASSWORD": password})
        self.assertNotIn(password, output)

    def test_compose_mode_keeps_explicit_owner_credentials(self) -> None:
        seen, _ = self.compose_mode_owner_credentials([
            "PLATFORM_POSTGRES_OWNER_PASSWORD=from-owner-secret",
            "DB_MIGRATIONS_USERNAME=explicit_owner",
            "DB_MIGRATIONS_PASSWORD=explicit-password",
        ])
        self.assertEqual(seen, {"USERNAME": "explicit_owner", "PASSWORD": "explicit-password"})

    def test_compose_mode_without_an_owner_password_derives_nothing(self) -> None:
        seen, _ = self.compose_mode_owner_credentials([])
        self.assertEqual(seen, {"USERNAME": "<unset>", "PASSWORD": "<unset>"})
        text = SCRIPT.read_text()
        self.assertIn('[[ -n "${DB_MIGRATIONS_USERNAME:-}" && -n "${DB_MIGRATIONS_PASSWORD:-}" ]]', text)


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
    def test_stale_lock_is_refused_without_changing_the_state_directory_mode(self) -> None:
        self.assertEqual(self.bootstrap("record-witness", "--local").returncode, 0)
        os.chmod(self.state_dir, 0o755)
        (self.state_dir / "run.lock").mkdir()
        self.assert_refused(self.bootstrap("run", "--local"), "Nothing was changed")
        self.assertEqual(stat.S_IMODE(self.state_dir.stat().st_mode), 0o755)
        self.assert_unchanged(0, None)

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

    def test_newline_named_file_in_the_new_directory_is_detected(self) -> None:
        self.mariadb_era_witness()
        self.assertEqual(self.bootstrap("record-witness", "--local").returncode, 0)
        env = self.base_env(OTERYN_PG_BOOTSTRAP_LOCAL_TEST_PLANT_AFTER_MOVE="\n")
        result = self.bootstrap("run", "--local", env=env)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("new_directory_not_empty", result.stderr)
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

    def test_failed_move_record_is_adopted_on_the_next_run(self) -> None:
        self.mariadb_era_witness()
        mariadb_tree = tree(self.witness_dir)
        self.assertEqual(self.bootstrap("record-witness", "--local").returncode, 0)
        # Simulate the failed record write: the directory was moved aside, no moved-aside record exists.
        aside = self.witness_dir.with_name(self.witness_dir.name + ".mariadb-era-20260101T000000Z")
        self.witness_dir.rename(aside)
        result = self.bootstrap("run", "--local")
        self.assertNotIn("no intact preserved copy", result.stderr)
        self.assertEqual(tree(aside), mariadb_tree)
        self.assertTrue((self.state_dir / "witness-moved-aside.env").exists() or result.returncode != 0)
        # A modified preserved copy is not adopted.
        if (self.state_dir / "witness-moved-aside.env").exists():
            return
        victim = next(p for p in sorted(aside.iterdir()) if p.name != "witness-store.id")
        victim.write_bytes(victim.read_bytes() + b"x")
        self.assert_refused(self.bootstrap("run", "--local"), "no intact preserved copy")


if __name__ == "__main__":
    unittest.main(verbosity=2)
