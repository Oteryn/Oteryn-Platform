"""Focused custody/registration-gap tests; never contact a Docker daemon."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('oracle_supervisor', Path(__file__).with_name('game-supervisor.py'))
SUP = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUP)


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
                               (b'', '0'), (b' 1', '0'), (b'yes', '0'), (b'1' * 32, '0')]:
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
