#!/usr/bin/python3
"""Forced SSH command for the Oracle staging runner key.

Installed root-owned next to staging-deploy.py and referenced from one
``restrict,command=`` line in root's authorized_keys. The key holder may only
read staging status/logs or queue a deploy of an exact SHA; the staging CLI
validates the SHA, the branch eligibility and the spec filter again.
"""
import json
import os
import re
import subprocess
import sys

CLI = '/opt/oteryn-staging/staging-deploy.py'
SHA_RE = re.compile(r'[0-9a-f]{40}')
SPEC_RE = re.compile(r'specs/[A-Za-z0-9_-][A-Za-z0-9._/-]{0,199}\.spec\.mjs')
CRATES_RE = re.compile(r'[a-z0-9_-]{1,64}(,[a-z0-9_-]{1,64}){0,19}')
PENDING = 2


class Refusal(ValueError):
    pass


def parse(command):
    words = command.split()
    if words in (['status'], ['logs']):
        return words
    if not words or words[0] != 'deploy' or len(words) not in (2, 3, 4, 5):
        raise Refusal('allowed: status | logs | deploy <sha> [specs/<file>.spec.mjs] [--pg-tests a,b]')
    argv, rest = ['deploy'], words[1:]
    sha = rest.pop(0)
    if not SHA_RE.fullmatch(sha):
        raise Refusal('deploy requires a full lowercase SHA')
    argv.append(sha)
    if rest and rest[0] != '--pg-tests':
        spec = rest.pop(0)
        if '..' in spec.split('/') or not SPEC_RE.fullmatch(spec):
            raise Refusal('spec must be a specs/<file>.spec.mjs path')
        argv.append(spec)
    if rest:
        if len(rest) != 2 or rest[0] != '--pg-tests' or not CRATES_RE.fullmatch(rest[1]):
            raise Refusal('--pg-tests requires comma-separated crate names')
        argv += rest
    return argv


def cli(argv, capture=False):
    return subprocess.run([sys.executable, CLI, *argv], stdin=subprocess.DEVNULL,
                          stdout=subprocess.PIPE if capture else None, text=True, check=False)


def pending_other(sha):
    result = cli(['status'], capture=True)
    if result.returncode != PENDING:
        return None
    for line in result.stdout.splitlines():
        try:
            record = json.loads(line)
        except ValueError:
            continue
        if isinstance(record, dict) and 'phase' in record and record.get('sha') != sha:
            return record.get('sha') or 'unknown'
    return None


def main(environ=os.environ):
    try:
        argv = parse(environ.get('SSH_ORIGINAL_COMMAND', ''))
    except Refusal as error:
        print(f'refused: {error}', file=sys.stderr)
        return 64
    if argv[0] == 'deploy':
        other = pending_other(argv[1])
        if other:
            print(f'refused: deploy of {other} is still pending', file=sys.stderr)
            return 75
    return cli(argv).returncode


if __name__ == '__main__':
    sys.exit(main())
