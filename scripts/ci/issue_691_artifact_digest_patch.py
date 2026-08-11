#!/usr/bin/env python3
from pathlib import Path

path = Path('.github/workflows/native-auth-ephemeral-cutover-rehearsal.yml')
text = path.read_text(encoding='utf-8')
old = '''            test "$(jq -r '.digest' <<<"${metadata}")" = "${digest}"\n'''
new = '''            test "$(jq -r '.digest' <<<"${metadata}")" = "sha256:${digest}"\n'''
if text.count(old) != 1:
    raise SystemExit(f'expected exactly one artifact digest comparison, found {text.count(old)}')
text = text.replace(old, new, 1)
path.write_text(text, encoding='utf-8')
print('Issue #691 artifact digest normalization patch: PASS')
