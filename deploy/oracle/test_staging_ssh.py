"""Exercise the staging runner's forced SSH command without touching Oracle."""
import importlib.util
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
SHA = 'b' * 40
OTHER = 'c' * 40


def load():
    spec = importlib.util.spec_from_file_location('staging_ssh', HERE / 'staging-ssh.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def completed(code, stdout=''):
    return subprocess.CompletedProcess([], code, stdout=stdout)


class StagingSsh(unittest.TestCase):
    def setUp(self):
        self.ssh = load()

    def test_accepts_only_the_bounded_commands(self):
        parse = self.ssh.parse
        self.assertEqual(parse('status'), ['status'])
        self.assertEqual(parse('logs'), ['logs'])
        self.assertEqual(parse(f'deploy {SHA}'), ['deploy', SHA])
        self.assertEqual(parse(f'deploy {SHA} specs/legal-pages.spec.mjs'),
                         ['deploy', SHA, 'specs/legal-pages.spec.mjs'])
        self.assertEqual(parse(f'deploy {SHA} --pg-tests oteryn-db,oteryn-web'),
                         ['deploy', SHA, '--pg-tests', 'oteryn-db,oteryn-web'])
        self.assertEqual(parse(f'deploy {SHA} specs/a/b.spec.mjs --pg-tests oteryn-db'),
                         ['deploy', SHA, 'specs/a/b.spec.mjs', '--pg-tests', 'oteryn-db'])

    def test_refuses_everything_else(self):
        for command in (
            '', 'bash', 'status; id', 'logs extra', 'deploy', f'deploy {SHA.upper()}',
            f'deploy {SHA[:39]}', f'deploy {SHA} ../x.spec.mjs', f'deploy {SHA} specs/../x.spec.mjs',
            f'deploy {SHA} specs/x.js', f'deploy {SHA} /etc/passwd', f'deploy {SHA} --pg-tests',
            f'deploy {SHA} --pg-tests a;b', f'deploy {SHA} specs/a.spec.mjs --other x',
            f'deploy {SHA} specs/a.spec.mjs specs/b.spec.mjs',
        ):
            with self.subTest(command=command), self.assertRaises(self.ssh.Refusal):
                self.ssh.parse(command)

    def test_refused_command_never_reaches_the_cli(self):
        with patch.object(self.ssh.subprocess, 'run') as run:
            self.assertEqual(self.ssh.main({'SSH_ORIGINAL_COMMAND': 'sh -c id'}), 64)
        run.assert_not_called()

    def test_deploy_waits_for_another_pending_sha(self):
        status = f'{{"current_sha": "{OTHER}", "ready": true}}\n{{"sha": "{OTHER}", "phase": "build"}}\n'
        with patch.object(self.ssh.subprocess, 'run', return_value=completed(2, status)) as run:
            self.assertEqual(self.ssh.main({'SSH_ORIGINAL_COMMAND': f'deploy {SHA}'}), 75)
        self.assertEqual(run.call_count, 1)
        self.assertEqual(run.call_args.args[0][-1], 'status')

    def test_deploy_runs_cli_with_an_argument_list(self):
        results = [completed(0, f'{{"sha": "{OTHER}", "phase": "passed"}}\n'), completed(0)]
        with patch.object(self.ssh.subprocess, 'run', side_effect=results) as run:
            self.assertEqual(self.ssh.main({'SSH_ORIGINAL_COMMAND': f'deploy {SHA} specs/a.spec.mjs'}), 0)
        argv = run.call_args.args[0]
        self.assertEqual(argv[1:], [self.ssh.CLI, 'deploy', SHA, 'specs/a.spec.mjs'])
        self.assertNotIn('shell', run.call_args.kwargs)

    def test_same_sha_pending_is_left_to_the_cli(self):
        results = [completed(2, f'{{"sha": "{SHA}", "phase": "build"}}\n'), completed(1)]
        with patch.object(self.ssh.subprocess, 'run', side_effect=results):
            self.assertEqual(self.ssh.main({'SSH_ORIGINAL_COMMAND': f'deploy {SHA}'}), 1)

    def test_status_return_code_is_forwarded(self):
        with patch.object(self.ssh.subprocess, 'run', return_value=completed(2)):
            self.assertEqual(self.ssh.main({'SSH_ORIGINAL_COMMAND': 'status'}), 2)


if __name__ == '__main__':
    unittest.main()
