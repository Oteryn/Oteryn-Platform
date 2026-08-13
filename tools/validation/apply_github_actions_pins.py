#!/usr/bin/env python3
"""Apply the reviewed Issue #1008 GitHub Actions tag-to-SHA mapping."""
from pathlib import Path

PINS = {
    "actions/checkout@v7": ("actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1", "v7.0.1"),
    "actions/upload-artifact@v7": ("actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a", "v7.0.1"),
    "actions/setup-node@v7": ("actions/setup-node@820762786026740c76f36085b0efc47a31fe5020", "v7.0.0"),
    "actions/setup-python@v7": ("actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97", "v7.0.0"),
    "actions/setup-go@v7": ("actions/setup-go@b7ad1dad31e06c5925ef5d2fc7ad053ef454303e", "v7.0.0"),
    "actions/download-artifact@v8": ("actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c", "v8.0.1"),
    "shivammathur/setup-php@v2": ("shivammathur/setup-php@f3e473d116dcccaddc5834248c87452386958240", "2.37.2"),
    "docker/setup-buildx-action@v4": ("docker/setup-buildx-action@bb05f3f5519dd87d3ba754cc423b652a5edd6d2c", "v4.2.0"),
    "docker/metadata-action@v6": ("docker/metadata-action@dc802804100637a589fabce1cb79ff13a1411302", "v6.2.0"),
    "docker/login-action@v4": ("docker/login-action@dbcb813823bdd20940b903addbd779551569679f", "v4.6.0"),
    "docker/build-push-action@v7": ("docker/build-push-action@53b7df96c91f9c12dcc8a07bcb9ccacbed38856a", "v7.3.0"),
}

root = Path(".github/workflows")
changed = 0
replacements = 0
for path in sorted([*root.rglob("*.yml"), *root.rglob("*.yaml")]):
    original = path.read_text(encoding="utf-8")
    text = original
    for mutable, (pinned, version) in PINS.items():
        needle = f"uses: {mutable}"
        replacement = f"uses: {pinned} # {version}"
        count = text.count(needle)
        if count:
            text = text.replace(needle, replacement)
            replacements += count
    if text != original:
        path.write_text(text, encoding="utf-8")
        changed += 1
print(f"Pinned {replacements} mutable uses reference(s) across {changed} workflow file(s).")
