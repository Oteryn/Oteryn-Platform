#!/usr/bin/env python3
from pathlib import Path

health_path = Path('deploy/synology/scripts/health-check.sh')
health = health_path.read_text(encoding='utf-8')
old = "docker exec \"$platform_container\" grep -q 'Scan with your authenticator app' /var/www/html/resources/views/identity/mfa/settings.blade.php\ndocker exec \"$platform_container\" grep -q 'mfa-qr' /var/www/html/public/css/mfa.css\n"
new = "docker exec \"$platform_container\" grep -q 'portal.identity.mfa_scan' /var/www/html/resources/views/identity/mfa/settings.blade.php\ndocker exec \"$platform_container\" grep -q 'mfa-qr-panel' /var/www/html/resources/views/identity/mfa/settings.blade.php\ndocker exec \"$platform_container\" grep -q 'mfa-qr-code' /var/www/html/resources/views/identity/mfa/settings.blade.php\ndocker exec \"$platform_container\" grep -q 'mfa-qr' /var/www/html/public/css/mfa.css\n"
if old not in health:
    raise SystemExit('expected localized-copy MFA health assertion not found')
health_path.write_text(health.replace(old, new, 1), encoding='utf-8')

test_path = Path('tests/ci/test_synology_auto_staging_deploy.py')
test = test_path.read_text(encoding='utf-8')
method = '''\n    def test_mfa_health_check_uses_semantic_markers_not_localized_copy(self) -> None:\n        health = (ROOT / "deploy/synology/scripts/health-check.sh").read_text(encoding="utf-8")\n        self.assertNotIn("grep -q 'Scan with your authenticator app'", health)\n        self.assertIn("grep -q 'portal.identity.mfa_scan'", health)\n        self.assertIn("grep -q 'mfa-qr-panel'", health)\n        self.assertIn("grep -q 'mfa-qr-code'", health)\n\n'''
if 'def test_mfa_health_check_uses_semantic_markers_not_localized_copy' not in test:
    marker = '\n\nif __name__ == "__main__":\n'
    if marker not in test:
        raise SystemExit('test insertion marker missing')
    test = test.replace(marker, method + marker, 1)
    test_path.write_text(test, encoding='utf-8')
