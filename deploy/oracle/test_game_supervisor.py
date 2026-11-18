"""Focused custody/registration-gap tests; never contact a Docker daemon."""
import importlib.util
import os
import shutil
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('oracle_supervisor', Path(__file__).with_name('game-supervisor.py'))
SUP = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUP)


DB_SPEC = importlib.util.spec_from_file_location('oracle_game_db', Path(__file__).with_name('prepare-game-db.py'))
DB = importlib.util.module_from_spec(DB_SPEC)
DB_SPEC.loader.exec_module(DB)


class DatabaseCertificateTests(unittest.TestCase):
    def test_prepare_issues_long_lived_game_database_ca(self):
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / 'source'
            postgres = source / 'deploy/synology/postgres'
            (postgres / 'backup').mkdir(parents=True)
            (postgres / 'postgresql.conf').write_text("ssl = on\n")
            (postgres / 'backup/archive-wal.sh').write_text('#!/bin/sh\n')
            root = Path(temporary) / 'game-db'
            ca_days = []

            def openssl(*args):
                self.assertEqual(args[0], 'openssl')
                for option in ('-out', '-keyout'):
                    if option in args:
                        Path(args[args.index(option) + 1]).write_text('fixture')
                if '-x509' in args:
                    ca_days.append(int(args[args.index('-days') + 1]))

            previous_umask = os.umask(0o077)
            self.addCleanup(os.umask, previous_umask)
            with patch.multiple(DB, ROOT=root, SOURCE=source, run=openssl), \
                 patch.object(DB.os, 'geteuid', return_value=0), \
                 patch.object(DB.os, 'chown'), patch.object(DB.os, 'fchown'):
                DB.main()
            self.assertEqual(len(ca_days), 1)
            self.assertGreaterEqual(ca_days[0], 365)
            self.assertEqual((root / 'client/ca.crt').read_bytes(), (root / 'signer/ca.crt').read_bytes())

    def database_certificates(self, root, ca_days):
        ca = root / 'signer'
        server = root / 'server'
        client = root / 'client'
        for directory in (ca, server, client):
            directory.mkdir()
        subprocess.run(
            ['openssl', 'req', '-x509', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:P-256', '-nodes',
             '-days', str(ca_days), '-subj', '/CN=Oteryn Oracle Game PostgreSQL CA',
             '-addext', 'basicConstraints=critical,CA:TRUE',
             '-keyout', str(ca / 'ca.key'), '-out', str(ca / 'ca.crt')],
            check=True, capture_output=True,
        )
        shutil.copyfile(ca / 'ca.crt', client / 'ca.crt')
        subprocess.run(
            ['openssl', 'genpkey', '-algorithm', 'EC', '-pkeyopt', 'ec_paramgen_curve:P-256',
             '-out', str(server / 'server.key')], check=True, capture_output=True,
        )
        with tempfile.TemporaryDirectory(dir=root) as temporary, patch.object(DB, 'SERVER_DAYS', 1), \
             patch.object(DB.os, 'fchown'):
            certificate = DB.issue_server_certificate(ca, server / 'server.key', Path(temporary))
            certificate.replace(server / 'server.crt')

    def test_game_database_renews_under_retained_ca_and_retries_failed_reload(self):
        oracle = Path(__file__).parent
        self.assertIn('OnCalendar=daily', (oracle / 'oteryn-game-db-certificate.timer').read_text())
        self.assertIn('prepare-game-db.py renew-server-certificate',
                      (oracle / 'oteryn-game-db-certificate.service').read_text())
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.database_certificates(root, ca_days=365)
            cert = root / 'server/server.crt'
            key = root / 'server/server.key'
            ca = root / 'client/ca.crt'
            old_cert, old_key, old_ca = cert.read_bytes(), key.read_bytes(), ca.read_bytes()

            actual_run = DB.run
            reload_count = 0

            def run(*args):
                nonlocal reload_count
                if args[0] != 'docker':
                    return actual_run(*args)
                self.assertEqual(args, ('docker', 'kill', '--signal', 'HUP', DB.POSTGRES))
                reload_count += 1
                self.assertNotEqual(cert.read_bytes(), old_cert)
                if reload_count == 1:
                    raise subprocess.CalledProcessError(1, args)

            with patch.multiple(DB, ROOT=root, run=run), patch.object(DB.os, 'fchown'), \
                 patch.object(DB.subprocess, 'check_output', return_value='true\n') as inspect:
                with self.assertRaises(subprocess.CalledProcessError):
                    DB.renew_server_certificate()
                renewed = cert.read_bytes()
                self.assertTrue((root / 'postgres-tls-reload-pending').exists())
                DB.renew_server_certificate()
                DB.renew_server_certificate()
                self.assertEqual(inspect.call_count, 2)
            self.assertEqual(reload_count, 2)
            self.assertFalse((root / 'postgres-tls-reload-pending').exists())
            self.assertEqual(cert.read_bytes(), renewed)
            self.assertEqual(key.read_bytes(), old_key)
            self.assertEqual(ca.read_bytes(), old_ca)
            self.assertTrue(DB.valid_for(cert, DB.RENEW_BEFORE_DAYS))

    def test_legacy_game_database_ca_extension_preserves_trust_and_signer(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.database_certificates(root, ca_days=30)
            ca = root / 'signer/ca.crt'
            client = root / 'client/ca.crt'
            key = root / 'signer/ca.key'
            old_key = key.read_bytes()
            old_client = root / 'old-ca.crt'
            shutil.copyfile(client, old_client)
            with patch.object(DB, 'ROOT', root):
                DB.extend_ca()
                self.assertEqual(ca.read_bytes(), client.read_bytes())
                self.assertNotEqual(client.read_bytes(), old_client.read_bytes())
                self.assertTrue(DB.valid_for(ca, 365))
                self.assertEqual(key.read_bytes(), old_key)
                DB.run('openssl', 'verify', '-CAfile', str(client), str(root / 'server/server.crt'))

                client.write_bytes(old_client.read_bytes())
                DB.extend_ca()
                self.assertEqual(ca.read_bytes(), client.read_bytes())

                with tempfile.TemporaryDirectory(dir=root) as work, patch.object(DB.os, 'fchown'):
                    leaf = DB.issue_server_certificate(ca.parent, root / 'server/server.key', Path(work))
                    DB.run('openssl', 'verify', '-CAfile', str(old_client), str(leaf))


class CustodyTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.base = self.root / 'game'
        (self.base / 'state').mkdir(parents=True)
        (self.base / 'node').mkdir()
        self.node = self.base / 'node/node.toml'
        self.node.write_text('[launch]\nauthorization_file = "old"\n')
        self.overrides = patch.multiple(SUP, ROOT=self.root, BASE=self.base, NODE=self.node, STOP=False)
        self.overrides.start()
        self.addCleanup(self.overrides.stop)
        # Unit tests run unprivileged; real UID/mode checks belong to host QA.
        self.ownership = patch.object(SUP.os, 'fchown')
        self.ownership.start()
        self.addCleanup(self.ownership.stop)

    def state(self, phase='authorizing'):
        return dict(cycle='a' * 32, phase=phase, previous='01a13fcf-b7f0-793a-af1c-8b8061a587d4',
                    launch='launch-retained.json')

    def result(self, text='', code=0):
        return subprocess.CompletedProcess([], code, text)

    def test_atomic_never_follows_destination_symlink(self):
        protected = self.root / 'protected'
        protected.write_bytes(b'retained')
        destination = self.root / 'destination'
        destination.symlink_to(protected)
        SUP.atomic(destination, b'new')
        self.assertEqual(protected.read_bytes(), b'retained')
        self.assertFalse(destination.is_symlink())
        self.assertEqual(destination.stat().st_mode & 0o777, 0o600)

    def test_atomic_refuses_symlink_parent(self):
        alias = self.root / 'alias'
        alias.symlink_to(self.base, target_is_directory=True)
        with self.assertRaises(OSError):
            SUP.atomic(alias / 'outside', b'bad')
        self.assertFalse((self.base / 'outside').exists())

    def test_retained_launch_is_reconciled_before_start(self):
        state = self.state()
        (self.base / 'state' / state['launch']).write_bytes(b'opaque retained authorization')
        with patch.object(SUP, 'ops', return_value=self.result()) as ops, \
             patch.object(SUP, 'inspect', return_value=None), \
             patch.object(SUP, 'invoke', return_value=self.result()), \
             patch.object(SUP, 'wait_registration', return_value=False):
            SUP.start(state, {})
        self.assertEqual(ops.call_args.args[1:3], ('authorization', 'reconcile'))
        self.assertEqual(state['phase'], 'starting')

    def test_uncertain_launch_never_starts_container(self):
        state = self.state()
        (self.base / 'state' / state['launch']).write_bytes(b'opaque retained authorization')
        with patch.object(SUP, 'ops', return_value=self.result(code=7)), \
             patch.object(SUP, 'invoke') as invoke:
            with self.assertRaises(RuntimeError):
                SUP.start(state, {})
        invoke.assert_not_called()
        self.assertTrue((self.root / 'reconciliation-required').exists())

    def test_registration_gap_stops_without_reminting(self):
        state = self.state('starting')
        current = {'Config': {'Labels': {'io.oteryn.launch-cycle': state['cycle']}},
                   'State': {'Running': False}}
        with patch.object(SUP, 'inspect', return_value=current), \
             patch.object(SUP, 'invoke', return_value=self.result('event=registering node_id=uncertain')), \
             patch.object(SUP, 'ops') as ops:
            with self.assertRaises(RuntimeError):
                SUP.wait_registration(state)
        ops.assert_not_called()
        self.assertIn('registration gap', (self.root / 'reconciliation-required').read_text())

    def test_absent_assignment_is_not_accepted_or_replaced(self):
        state = self.state('assigning')
        state.update(node_id=state['previous'], revision='17')
        (self.base / 'state' / ('assign-oracle-' + state['cycle'] + '.json')).write_bytes(b'retained')
        with patch.object(SUP, 'ops', return_value=self.result('assignment=absent')) as ops:
            with self.assertRaises(RuntimeError):
                SUP.start(state, {})
        self.assertEqual(ops.call_count, 1)
        self.assertEqual(ops.call_args.args[1:3], ('assignment', 'reconcile'))


    def marker(self, data):
        (self.base / 'run').mkdir(exist_ok=True)
        (self.base / 'run/step-wait-receipts').write_bytes(data)

    def test_step_wait_marker_absent_keeps_default_off(self):
        self.assertEqual(SUP.step_wait_receipts(), '0')

    def test_step_wait_marker_enables_only_exact_one(self):
        for data, expected in [(b'1', '1'), (b'1\n', '1'), (b'0', '0'), (b'11', '0'),
                               (b'', '0'), (b' 1', '0'), (b'yes', '0'), (b'1' * 32, '0'),
                               (b'1\n\n', '0'), (b'\n1', '0'), (b'1\ngarbage', '0'),
                               (b'1\r\n', '0'), (b'1' + b'\n' * 4096, '0'),
                               (b'1\n' + b'x' * (1 << 20), '0')]:
            self.marker(data)
            self.assertEqual(SUP.step_wait_receipts(), expected, data)

    def test_step_wait_marker_refuses_symlink_and_directory(self):
        target = self.root / 'elsewhere'
        target.write_bytes(b'1')
        (self.base / 'run').mkdir()
        (self.base / 'run/step-wait-receipts').symlink_to(target)
        self.assertEqual(SUP.step_wait_receipts(), '0')
        (self.base / 'run/step-wait-receipts').unlink()
        (self.base / 'run/step-wait-receipts').mkdir()
        self.assertEqual(SUP.step_wait_receipts(), '0')

    def launch_environment(self):
        seen = {}

        def record(args, **_):
            if 'up' in args:
                seen['value'] = SUP.os.environ.get(SUP.STEP_WAIT_ENV)
            return self.result()
        with patch.dict(SUP.os.environ, {SUP.STEP_WAIT_ENV: 'stale'}), \
             patch.object(SUP, 'inspect', return_value=None), \
             patch.object(SUP, 'invoke', side_effect=record), \
             patch.object(SUP, 'wait_registration', return_value=False):
            SUP.start(self.state('starting'), {})
        return seen['value']

    def test_new_incarnation_exports_marker_state(self):
        self.assertEqual(self.launch_environment(), '0')
        self.marker(b'1')
        self.assertEqual(self.launch_environment(), '1')
        self.marker(b'0')
        self.assertEqual(self.launch_environment(), '0')

    def test_compose_passes_switch_to_game_only_default_off(self):
        text = Path(__file__).with_name('game-runtime.compose.yml').read_text()
        game, ops = text.split('\n  ops:\n')
        self.assertIn('      OTERYN_STEP_WAIT_RECEIPTS: ${OTERYN_STEP_WAIT_RECEIPTS:-0}\n', game)
        self.assertIn('      io.oteryn.step-wait-receipts: ${OTERYN_STEP_WAIT_RECEIPTS:-0}\n', game)
        self.assertNotIn('STEP_WAIT', ops)

if __name__ == '__main__':
    unittest.main()
