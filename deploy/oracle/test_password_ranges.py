"""Offline tests for the breached-password range importer."""
import importlib.util
import io
from pathlib import Path
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('password_ranges', HERE / 'password-ranges.py')
pr = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pr)

A = '0' * 35
B = '1' * 35
C = 'A' * 35


class ParseRange(unittest.TestCase):
    def test_threshold_keeps_frequent_suffixes_sorted(self):
        body = f'{C}:150\r\n{A}:3\r\n{B}:100\r\n'
        self.assertEqual(pr.parse_range(body, 100), [B, C])
        self.assertEqual(pr.parse_range(body, 1), [A, B, C])

    def test_confirmed_range_below_threshold_is_empty(self):
        self.assertEqual(pr.parse_range(f'{A}:2\n', 100), [])

    def test_bad_lines_fail_the_prefix(self):
        for body in (
            '',
            f'{A}:0\n',            # padding entry
            f'{C.lower()}:5\n',    # lowercase
            f'{A[:-1]}:5\n',       # short suffix
            f'{A}5\n',             # missing separator
            f'{A}:-1\n',
            f'{A}:5\n{A}:6\n',     # duplicate
            f'{A}:5\n\n{B}:5\n',   # blank line inside
            '<html>error</html>',
        ):
            with self.assertRaises(pr.RangeError, msg=body):
                pr.parse_range(body, 1)

    def test_min_count_must_be_positive(self):
        with self.assertRaises(pr.RangeError):
            pr.parse_range(f'{A}:5\n', 0)


class Blocks(unittest.TestCase):
    def test_block_has_256_prefixes(self):
        ps = pr.block_prefixes('0AF')
        self.assertEqual(len(ps), 256)
        self.assertEqual(ps[0], '0AF00')
        self.assertEqual(ps[-1], '0AFFF')

    def test_encode_row_rejects_bad_values(self):
        self.assertEqual(pr.encode_row('0AF00', [A, B]), f'0AF00\t{{{A},{B}}}\n')
        self.assertEqual(pr.encode_row('0AF00', []), '0AF00\t{}\n')
        for prefix, suffixes in (('0af00', []), ('0AF0', []), ('0AF00', ['x'])):
            with self.assertRaises(pr.RangeError):
                pr.encode_row(prefix, suffixes)

    def test_failed_prefix_leaves_no_block_and_rerun_resumes(self):
        calls = []

        def flaky(prefix, min_count):
            calls.append(prefix)
            if prefix == '00010':
                raise pr.RangeError('boom')
            return [B] if prefix.endswith('0') else []

        old = pr.TOTAL_BLOCKS
        pr.TOTAL_BLOCKS = 2
        try:
            with tempfile.TemporaryDirectory() as d:
                self.assertFalse(pr.fetch(d, 100, 4, fetcher=flaky))
                self.assertFalse((Path(d) / '000.tsv').exists())
                self.assertTrue((Path(d) / '001.tsv').exists())
                calls.clear()
                self.assertTrue(pr.fetch(d, 100, 4, fetcher=lambda p, m: []))
                self.assertTrue(all(p.startswith('000') for p in calls))
                out = io.StringIO()
                pr.emit_sql(d, 365, out)
                sql = out.getvalue()
                self.assertEqual(sql.count('BEGIN;'), 2)
                self.assertIn("interval '365 days'", sql)
                self.assertIn(f'00110\t{{{B}}}\n', sql)
                self.assertIn('00000\t{}\n', sql)
        finally:
            pr.TOTAL_BLOCKS = old

    def test_only_full_import_is_complete_and_never_downgrades(self):
        old = pr.TOTAL_BLOCKS
        pr.TOTAL_BLOCKS = 1
        try:
            for min_count, complete in ((1, 'true'), (100, 'false')):
                with tempfile.TemporaryDirectory() as d:
                    self.assertTrue(pr.fetch(d, min_count, 4, fetcher=lambda p, m: []))
                    out = io.StringIO()
                    pr.emit_sql(d, 365, out)
                    sql = out.getvalue()
                    self.assertIn(f"interval '365 days', {complete}, pg_current_xact_id() FROM import_ranges", sql)
                    self.assertIn('complete = EXCLUDED.complete', sql)
                    self.assertIn('complete_xact = pg_current_xact_id()', sql)
                    self.assertIn("(prefix, suffixes, expires_at, complete, complete_xact) ", sql)
                    self.assertIn('WHERE EXCLUDED.complete OR NOT registration_password_ranges.complete', sql)
        finally:
            pr.TOTAL_BLOCKS = old

    def test_threshold_is_pinned_to_the_directory(self):
        old = pr.TOTAL_BLOCKS
        pr.TOTAL_BLOCKS = 1
        try:
            with tempfile.TemporaryDirectory() as d:
                self.assertTrue(pr.fetch(d, 100, 4, fetcher=lambda p, m: []))
                with self.assertRaises(pr.RangeError):
                    pr.fetch(d, 1, 4, fetcher=lambda p, m: [])
            with tempfile.TemporaryDirectory() as d:
                (Path(d) / '000.tsv').write_text('', encoding='ascii')
                with self.assertRaises(pr.RangeError):
                    pr.fetch(d, 1, 4, fetcher=lambda p, m: [])
            with tempfile.TemporaryDirectory() as d:
                (Path(d) / '000.tsv').write_text(''.join(f'{p}\t{{}}\n' for p in pr.block_prefixes('000')), encoding='ascii')
                out = io.StringIO()
                pr.emit_sql(d, 365, out)
                self.assertIn("interval '365 days', false, pg_current_xact_id() FROM", out.getvalue())
        finally:
            pr.TOTAL_BLOCKS = old

    def test_sql_refuses_incomplete_block_and_bad_expiry(self):
        with tempfile.TemporaryDirectory() as d:
            (Path(d) / '000.tsv').write_text('00000\t{}\n', encoding='ascii')
            with self.assertRaises(pr.RangeError):
                pr.emit_sql(d, 365, io.StringIO())
            with self.assertRaises(pr.RangeError):
                pr.emit_sql(d, 0, io.StringIO())


if __name__ == '__main__':
    unittest.main()
