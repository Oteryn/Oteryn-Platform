#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path

workflow_path = Path('.github/workflows/deploy-synology-staging.yml')
workflow = workflow_path.read_text(encoding='utf-8')
old_tool = '          python3 --version >/dev/null\n'
if old_tool not in workflow:
    raise SystemExit('expected python3 runner-tool assertion not found')
workflow = workflow.replace(old_tool, '', 1)

old_workflow_ipv4 = '''          python3 - "$CANARY_GAME_BIND_ADDRESS_INPUT" <<'PY'\n          import ipaddress\n          import sys\n\n          try:\n              address = ipaddress.ip_address(sys.argv[1])\n          except ValueError as exc:\n              raise SystemExit(f'canary_game_bind_address is invalid: {exc}') from exc\n\n          if address.version != 4 or address.is_unspecified or address.is_multicast or address.is_link_local:\n              raise SystemExit('canary_game_bind_address must be a usable IPv4 address')\n          if not (address.is_private or address.is_loopback):\n              raise SystemExit('canary_game_bind_address must be loopback or RFC1918 private IPv4')\n          PY\n'''
new_workflow_ipv4 = '''          if ! bash deploy/synology/scripts/validate-ipv4.sh "$CANARY_GAME_BIND_ADDRESS_INPUT" private-or-loopback; then\n            echo "canary_game_bind_address must be loopback or RFC1918 private IPv4" >&2\n            exit 1\n          fi\n'''
if old_workflow_ipv4 not in workflow:
    raise SystemExit('expected workflow Python IPv4 validator not found')
workflow = workflow.replace(old_workflow_ipv4, new_workflow_ipv4, 1)
workflow_path.write_text(workflow, encoding='utf-8')

deploy_path = Path('deploy/synology/scripts/deploy.sh')
deploy = deploy_path.read_text(encoding='utf-8')
old_python_requirement = '''if ! command -v python3 >/dev/null 2>&1; then\n    echo "python3 is required on the deployment runner." >&2\n    exit 1\nfi\n\n'''
if old_python_requirement not in deploy:
    raise SystemExit('expected deploy.sh Python requirement not found')
deploy = deploy.replace(old_python_requirement, '', 1)
start = deploy.index('validate_ipv4_policy() {')
end = deploy.index('\n\nvalidate_port() {', start)
old_function = deploy[start:end]
if 'python3 - "$address" "$policy"' not in old_function:
    raise SystemExit('expected deploy.sh Python IPv4 validator not found')
new_function = '''validate_ipv4_policy() {\n    local address="$1"\n    local policy="$2"\n    bash "$SCRIPT_DIR/validate-ipv4.sh" "$address" "$policy"\n}'''
deploy = deploy[:start] + new_function + deploy[end:]
deploy_path.write_text(deploy, encoding='utf-8')

helper_path = Path('deploy/synology/scripts/validate-ipv4.sh')
helper_path.write_text('''#!/usr/bin/env bash\nset -euo pipefail\n\naddress="${1:-}"\npolicy="${2:-}"\n\nfail() {\n    echo "$1" >&2\n    exit 1\n}\n\nif [[ "$policy" != "loopback" && "$policy" != "private-or-loopback" ]]; then\n    fail "unsupported IPv4 policy"\nfi\n\nIFS='.' read -r a b c d extra <<< "$address"\nif [[ -n "${extra:-}" || -z "${a:-}" || -z "${b:-}" || -z "${c:-}" || -z "${d:-}" ]]; then\n    fail "address must contain exactly four IPv4 octets"\nfi\n\nfor octet in "$a" "$b" "$c" "$d"; do\n    if [[ ! "$octet" =~ ^(0|[1-9][0-9]{0,2})$ ]]; then\n        fail "IPv4 octets must be canonical decimal values"\n    fi\n    if (( 10#$octet > 255 )); then\n        fail "IPv4 octet exceeds 255"\n    fi\ndone\n\na_n=$((10#$a))\nb_n=$((10#$b))\n\nif [[ "$policy" == "loopback" ]]; then\n    [[ "$address" == "127.0.0.1" ]] || fail "service must remain bound to exact loopback 127.0.0.1"\n    exit 0\nfi\n\nif (( a_n == 127 )); then\n    exit 0\nfi\nif (( a_n == 10 )); then\n    exit 0\nfi\nif (( a_n == 172 && b_n >= 16 && b_n <= 31 )); then\n    exit 0\nfi\nif (( a_n == 192 && b_n == 168 )); then\n    exit 0\nfi\n\nfail "game bind must be loopback or an RFC1918 private IPv4 address"\n''', encoding='utf-8')

test_path = Path('tests/ci/test_synology_auto_staging_deploy.py')
test = test_path.read_text(encoding='utf-8')
if 'import subprocess\n' not in test:
    test = test.replace('import unittest\n', 'import subprocess\nimport unittest\n', 1)
marker = 'DEPLOY_WORKFLOW = ROOT / ".github/workflows/deploy-synology-staging.yml"\n'
constants = 'DEPLOY_SCRIPT = ROOT / "deploy/synology/scripts/deploy.sh"\nIPV4_HELPER = ROOT / "deploy/synology/scripts/validate-ipv4.sh"\n'
if constants not in test:
    if marker not in test:
        raise SystemExit('test constant insertion marker missing')
    test = test.replace(marker, marker + constants, 1)
setup_marker = '        cls.deploy_workflow = DEPLOY_WORKFLOW.read_text(encoding="utf-8")\n'
setup_add = '        cls.deploy_script = DEPLOY_SCRIPT.read_text(encoding="utf-8")\n'
if setup_add not in test:
    if setup_marker not in test:
        raise SystemExit('test setup insertion marker missing')
    test = test.replace(setup_marker, setup_marker + setup_add, 1)
methods = '''\n    def run_ipv4_helper(self, address: str, policy: str) -> subprocess.CompletedProcess[str]:\n        return subprocess.run(\n            ["bash", str(IPV4_HELPER), address, policy],\n            cwd=ROOT,\n            text=True,\n            stdout=subprocess.PIPE,\n            stderr=subprocess.PIPE,\n            check=False,\n        )\n\n    def test_synology_runtime_deploy_no_longer_requires_python(self) -> None:\n        self.assertNotIn("python3 --version", self.deploy_workflow)\n        self.assertNotIn("python3 -", self.deploy_workflow)\n        self.assertNotIn("command -v python3", self.deploy_script)\n        self.assertNotIn("python3 -", self.deploy_script)\n        self.assertIn("validate-ipv4.sh", self.deploy_workflow)\n        self.assertIn("validate-ipv4.sh", self.deploy_script)\n\n    def test_shell_ipv4_helper_accepts_only_loopback_or_rfc1918_for_game_bind(self) -> None:\n        accepted = (\n            "127.0.0.1", "127.20.1.2", "10.0.0.1", "172.16.0.1",\n            "172.31.255.254", "192.168.1.2",\n        )\n        rejected = (\n            "0.0.0.0", "8.8.8.8", "169.254.1.1", "172.15.0.1",\n            "172.32.0.1", "192.167.1.1", "224.0.0.1", "256.1.1.1",\n            "192.168.001.2", "192.168.1", "not-an-ip",\n        )\n        for address in accepted:\n            with self.subTest(address=address):\n                self.assertEqual(self.run_ipv4_helper(address, "private-or-loopback").returncode, 0)\n        for address in rejected:\n            with self.subTest(address=address):\n                self.assertNotEqual(self.run_ipv4_helper(address, "private-or-loopback").returncode, 0)\n\n    def test_shell_ipv4_helper_keeps_service_binds_on_exact_loopback(self) -> None:\n        self.assertEqual(self.run_ipv4_helper("127.0.0.1", "loopback").returncode, 0)\n        for address in ("127.0.0.2", "10.0.0.1", "192.168.1.2"):\n            with self.subTest(address=address):\n                self.assertNotEqual(self.run_ipv4_helper(address, "loopback").returncode, 0)\n'''
insertion = '\n\nif __name__ == "__main__":\n'
if 'def test_synology_runtime_deploy_no_longer_requires_python' not in test:
    if insertion not in test:
        raise SystemExit('test method insertion marker missing')
    test = test.replace(insertion, methods + insertion, 1)
test_path.write_text(test, encoding='utf-8')

Path('.github/workflows/temp-synology-pythonless-writer.yml').unlink()
Path('.github/scripts/temp_synology_fix_writer.py').unlink()
