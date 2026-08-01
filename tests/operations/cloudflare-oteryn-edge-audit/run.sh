#!/usr/bin/env bash
set -euo pipefail

python3 -m py_compile scripts/operations/cloudflare-oteryn-edge-audit.py
python3 -m unittest discover \
  -s tests/operations/cloudflare-oteryn-edge-audit \
  -p 'test_*.py' \
  -v
