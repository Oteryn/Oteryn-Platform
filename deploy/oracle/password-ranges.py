#!/usr/bin/env python3
"""Operator import of the breached-password corpus into registration_password_ranges.

Two steps, both resumable:

  password-ranges.py fetch DIR --min-count N
      Downloads every SHA-1 prefix 00000..FFFFF from the Pwned Passwords range
      API (no padding, generic User-Agent), validates every line, keeps the
      suffixes seen at least N times and writes one block file per three-hex
      block (256 prefixes) atomically. A block is written only when all of its
      prefixes downloaded and validated; any malformed line fails the prefix,
      never turns into an empty range. Existing block files are skipped.

  password-ranges.py sql DIR --expires-days D | psql -v ON_ERROR_STOP=1 ...
      Emits one transaction per complete block: COPY into a temporary table,
      then upsert into registration_password_ranges with
      expires_at = now() + D days. Re-running is idempotent.

The threshold and the expiry are operator decisions; record both with the
import (see crates/identity/src/abuse/OPERATIONS.md).
"""
import argparse
import concurrent.futures
import os
from pathlib import Path
import re
import sys
import time
import urllib.request

API = 'https://api.pwnedpasswords.com/range/'
USER_AGENT = 'password-range-import'
LINE = re.compile(r'^([0-9A-F]{35}):([1-9][0-9]*)$')
PREFIX = re.compile(r'^[0-9A-F]{5}$')
BLOCK = re.compile(r'^[0-9A-F]{3}$')
TOTAL_BLOCKS = 4096


class RangeError(Exception):
    pass


def parse_range(body, min_count):
    """Return the sorted suffixes seen at least min_count times; fail on any bad line."""
    if min_count < 1:
        raise RangeError('min_count must be positive')
    kept = []
    seen = set()
    lines = body.replace('\r\n', '\n').split('\n')
    if lines and lines[-1] == '':
        lines.pop()
    if not lines:
        raise RangeError('empty range body')
    for line in lines:
        m = LINE.match(line)
        if not m:
            raise RangeError('malformed range line')
        suffix, count = m.group(1), int(m.group(2))
        if suffix in seen:
            raise RangeError('duplicate suffix')
        seen.add(suffix)
        if count >= min_count:
            kept.append(suffix)
    kept.sort()
    return kept


def block_prefixes(block):
    if not BLOCK.match(block):
        raise RangeError('bad block')
    return [f'{block}{i:02X}' for i in range(256)]


def fetch_prefix(prefix, min_count, attempts=6):
    if not PREFIX.match(prefix):
        raise RangeError('bad prefix')
    last = None
    for attempt in range(attempts):
        try:
            req = urllib.request.Request(API + prefix, headers={'User-Agent': USER_AGENT})
            with urllib.request.urlopen(req, timeout=30) as resp:
                if resp.status != 200:
                    raise RangeError(f'http {resp.status}')
                return parse_range(resp.read().decode('ascii'), min_count)
        except Exception as e:  # retried, then the whole block fails
            last = e
            time.sleep(min(30, 2 ** attempt))
    raise RangeError(f'{prefix}: {type(last).__name__}')


def encode_row(prefix, suffixes):
    """One COPY text-format row: prefix TAB postgres text[] literal."""
    if not PREFIX.match(prefix):
        raise RangeError('bad prefix')
    for s in suffixes:
        if not re.match(r'^[0-9A-F]{35}$', s):
            raise RangeError('bad suffix')
    return f'{prefix}\t{{{",".join(suffixes)}}}\n'


def write_block(directory, block, rows):
    final = Path(directory) / f'{block}.tsv'
    tmp = final.with_suffix('.tmp')
    with open(tmp, 'w', encoding='ascii') as f:
        for prefix, suffixes in rows:
            f.write(encode_row(prefix, suffixes))
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, final)


def fetch(directory, min_count, workers, fetcher=fetch_prefix):
    Path(directory).mkdir(parents=True, exist_ok=True)
    failed = 0
    for b in range(TOTAL_BLOCKS):
        block = f'{b:03X}'
        if (Path(directory) / f'{block}.tsv').exists():
            continue
        prefixes = block_prefixes(block)
        try:
            with concurrent.futures.ThreadPoolExecutor(workers) as pool:
                results = list(pool.map(lambda p: fetcher(p, min_count), prefixes))
        except RangeError as e:
            failed += 1
            print(f'block {block} failed: {e}', file=sys.stderr, flush=True)
            continue
        write_block(directory, block, zip(prefixes, results))
        if b % 64 == 0:
            print(f'block {block} done', flush=True)
    done = len(list(Path(directory).glob('*.tsv')))
    print(f'complete blocks {done}/{TOTAL_BLOCKS}, failed this run {failed}', flush=True)
    return done == TOTAL_BLOCKS


def emit_sql(directory, expires_days, out):
    if not 1 <= expires_days <= 730:
        raise RangeError('expires_days out of range')
    for b in range(TOTAL_BLOCKS):
        path = Path(directory) / f'{b:03X}.tsv'
        if not path.exists():
            continue
        data = path.read_text(encoding='ascii')
        if data.count('\n') != 256:
            raise RangeError(f'incomplete block {path.name}')
        out.write('BEGIN;\n')
        out.write('CREATE TEMP TABLE import_ranges (prefix text, suffixes text[]) ON COMMIT DROP;\n')
        out.write('COPY import_ranges (prefix, suffixes) FROM STDIN;\n')
        out.write(data)
        out.write('\\.\n')
        out.write(
            'INSERT INTO registration_password_ranges (prefix, suffixes, expires_at) '
            f"SELECT prefix, suffixes, now() + interval '{int(expires_days)} days' FROM import_ranges "
            'ON CONFLICT (prefix) DO UPDATE SET suffixes = EXCLUDED.suffixes, '
            'expires_at = EXCLUDED.expires_at, updated_at = now();\n'
        )
        out.write('COMMIT;\n')


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest='cmd', required=True)
    f = sub.add_parser('fetch')
    f.add_argument('directory')
    f.add_argument('--min-count', type=int, required=True)
    f.add_argument('--workers', type=int, default=16)
    s = sub.add_parser('sql')
    s.add_argument('directory')
    s.add_argument('--expires-days', type=int, required=True)
    a = ap.parse_args(argv)
    if a.cmd == 'fetch':
        if not 1 <= a.workers <= 32:
            ap.error('workers must be 1..32')
        return 0 if fetch(a.directory, a.min_count, a.workers) else 1
    emit_sql(a.directory, a.expires_days, sys.stdout)
    return 0


if __name__ == '__main__':
    sys.exit(main())
