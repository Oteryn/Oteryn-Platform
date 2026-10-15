import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('oracle_deploy', Path(__file__).with_name('actions-deploy.py'))
deploy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(deploy)


class AuthorityBoundary(unittest.TestCase):
    def test_component_cannot_select_another_component_or_general_shell(self):
        for component, command in [('other', 'status'), ('game', 'bash'),
                                   ('game', 'status; id'), ('game', 'deploy ' + 'a' * 40 + ' extra'),
                                   ('platform', 'deploy ../../main'), ('game', 'deploy ' + 'A' * 40)]:
            with self.subTest(component=component, command=command), self.assertRaises(ValueError):
                deploy.parse(component, command)

    def test_exact_current_main_and_aggregate_success_are_independent(self):
        sha = 'a' * 40
        good = {'id': 10, 'name': 'game-gate', 'head_sha': sha,
                'app': {'slug': 'github-actions'}, 'conclusion': 'success'}
        # Change exactly one authority fact per negative case.
        cases = [({'commit': {'sha': 'b' * 40}}, [good]),
                 ({'commit': {'sha': sha}}, []),
                 ({'commit': {'sha': sha}}, [dict(good, head_sha='b' * 40)]),
                 ({'commit': {'sha': sha}}, [dict(good, app={'slug': 'other'})]),
                 ({'commit': {'sha': sha}}, [dict(good, conclusion='failure')]),
                 ({'commit': {'sha': sha}}, [good, dict(good, id=11, conclusion=None)])]
        for main, checks in cases:
            with self.subTest(main=main, checks=checks), patch.object(deploy, 'api', side_effect=[main, {'check_runs': checks}]), self.assertRaises(ValueError):
                deploy.qualify('game', sha)
        with patch.object(deploy, 'api', side_effect=[{'commit': {'sha': sha}}, {'check_runs': [good]}]):
            deploy.qualify('game', sha)

    def test_running_container_must_match_retained_incarnation(self):
        with patch.object(deploy, 'ROOT') as root, patch.object(deploy, 'run'), patch.object(deploy, 'inspect') as inspect:
            root.__truediv__.return_value.read_text.return_value = '{"phase":"ready","cycle":"expected"}'
            root.__truediv__.return_value.exists.return_value = False
            inspect.return_value = {'State': {'Running': True}, 'Config': {'Labels': {'io.oteryn.launch-cycle': 'other'}}}
            with self.assertRaisesRegex(ValueError, 'incarnation'):
                deploy.status('game')

    def test_key_cannot_bypass_workflow_or_environment_release(self):
        sha = 'a' * 40
        workflow = {'head_sha': sha, 'head_branch': 'main', 'event': 'workflow_dispatch',
                    'path': '.github/workflows/oracle-operations.yml', 'status': 'in_progress',
                    'run_attempt': 1, 'display_title': 'Oracle deploy ' + sha,
                    'repository': {'full_name': 'Oteryn/Oteryn-Game'},
                    'head_repository': {'full_name': 'Oteryn/Oteryn-Game'}}
        job = {'name': 'operate', 'status': 'in_progress', 'runner_name': 'oteryn-oracle-game'}
        for field, value in [('head_sha', 'b' * 40), ('head_branch', 'other'),
                             ('event', 'pull_request'), ('path', '.github/workflows/other.yml'),
                             ('status', 'completed'), ('run_attempt', 2),
                             ('display_title', 'Oracle status '),
                             ('head_repository', {'full_name': 'other/repo'})]:
            with self.subTest(field=field), patch.object(deploy, 'api', return_value=dict(workflow, **{field: value})), self.assertRaises(ValueError):
                deploy.qualify_workflow('game', sha, 100, 1)
        for changed_job in [dict(job, status='waiting'), dict(job, runner_name='other')]:
            with patch.object(deploy, 'api', side_effect=[workflow, {'jobs': [changed_job]}]), self.assertRaises(ValueError):
                deploy.qualify_workflow('game', sha, 100, 1)
        with patch.object(deploy, 'api', side_effect=[workflow, {'jobs': [job]}]):
            deploy.qualify_workflow('game', sha, 100, 1)


if __name__ == '__main__':
    unittest.main()
