from __future__ import annotations

import json
import os
import stat
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from tibia_linux_reference import HarnessError
from tibia_linux_reference.identity import ApprovedIdentity, verify_client_identity
from tibia_linux_reference.launcher import _parse_events, verify_cleanup
from tibia_linux_reference.manifest import validate_manifest
from tibia_linux_reference.security import (
    redact_text,
    require_private_directory,
    scan_prohibited_locations,
    sha256_file,
    validate_no_secret_like_environment,
    validate_no_shell_trace,
)


ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[1]


class SecurityTests(unittest.TestCase):
    def test_redaction_removes_exact_and_token_pattern_values(self) -> None:
        value = "SYNTHETIC-password-unique-value"
        token = "gho_" + ("a" * 26)
        rendered = redact_text(f"login={value} token={token}", [value])
        self.assertNotIn(value, rendered)
        self.assertNotIn("gho_", rendered)
        self.assertEqual(rendered.count("[REDACTED]"), 2)

    def test_secret_environment_is_rejected_without_echoing_value(self) -> None:
        with self.assertRaisesRegex(HarnessError, "API_TOKEN") as caught:
            validate_no_secret_like_environment({"API_TOKEN": "synthetic-value-that-is-never-reported"})
        self.assertNotIn("synthetic-value", str(caught.exception))

    def test_shell_trace_is_rejected(self) -> None:
        with self.assertRaises(HarnessError):
            validate_no_shell_trace({"SHELLOPTS": "braceexpand:xtrace"})

    def test_evidence_inside_repo_is_rejected(self) -> None:
        with self.assertRaises(HarnessError):
            require_private_directory(REPO / "forbidden-evidence", REPO)

    def test_exact_secret_leak_is_classified(self) -> None:
        secret = "synthetic-login-corpus-value-123456789"
        with tempfile.TemporaryDirectory() as directory:
            evidence = Path(directory)
            (evidence / "bad.log").write_text(secret, encoding="utf-8")
            result = scan_prohibited_locations(
                repo_root=REPO,
                evidence_root=evidence,
                temporary_root=None,
                secrets=[secret],
                process_arguments=b"safe",
                retained_environment_report=b"safe",
                stdout=b"safe",
                stderr=b"safe",
            )
        self.assertFalse(result.passed)
        self.assertIn("evidence-or-artifacts", result.categories)


class IdentityTests(unittest.TestCase):
    def test_template_placeholders_are_rejected(self) -> None:
        with self.assertRaises(HarnessError):
            ApprovedIdentity.load(ROOT / "identity.template.json")

    def test_exact_identity_match_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            private = Path(directory)
            package = private / "client.tar"
            executable = private / "client"
            package.write_bytes(b"synthetic package")
            executable.write_bytes(b"synthetic ELF fixture")
            executable.chmod(executable.stat().st_mode | stat.S_IXUSR)
            approved = ApprovedIdentity(
                client_version="synthetic-1",
                package_sha256=sha256_file(package),
                executable_sha256=sha256_file(executable),
                elf_build_id="a7256985ece88dc38f45b9248c6119c22359ae6a",
                package_source="synthetic-test-fixture",
            )
            with mock.patch("tibia_linux_reference.identity.elf_build_id", return_value=approved.elf_build_id):
                result = verify_client_identity(
                    approved,
                    package_path=package,
                    executable_path=executable,
                    repo_root=REPO,
                )
        self.assertEqual(result["binary_modified"], False)
        self.assertEqual(result["package_path_classification"], "private-outside-git")


class LifecycleTests(unittest.TestCase):
    def test_event_parser_rejects_unbounded_fields(self) -> None:
        with self.assertRaises(HarnessError):
            _parse_events(b'{"event":"process_state","state":"started","raw":"forbidden"}\n')

    def test_expected_fake_event_sequence_parses_deterministically(self) -> None:
        data = b'\n'.join(
            [
                b'{"event":"process_state","state":"started"}',
                b'{"event":"credential_channel_consumed","mechanism":"anonymous-pipe","value_count":4}',
                b'{"denied":true,"endpoint_classification":"TEST-NET-2:443","error_class":"OSError","event":"network_denial","interfaces":["lo"],"namespace":"net:[1]"}',
                b'{"backend":"x11","event":"window_state","state":"mapped"}',
                b'{"backend":"x11","event":"window_state","state":"destroyed"}',
                b'{"event":"process_state","state":"exiting"}',
            ]
        )
        self.assertEqual([item["event"] for item in _parse_events(data)], [
            "process_state", "credential_channel_consumed", "network_denial", "window_state", "window_state", "process_state"
        ])

    def test_cleanup_requires_profile_removal_and_empty_raw_root(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            raw = Path(directory)
            profile = raw / "profile"
            profile.mkdir()
            failed = verify_cleanup(process_stopped=True, profile=profile, raw_root=raw)
            self.assertEqual(failed["result"], "FAIL")
            profile.rmdir()
            passed = verify_cleanup(process_stopped=True, profile=profile, raw_root=raw)
            self.assertEqual(passed["result"], "PASS")


class ManifestTests(unittest.TestCase):
    def test_synthetic_example_matches_schema(self) -> None:
        document = json.loads((ROOT / "examples" / "session-manifest.synthetic.json").read_text(encoding="utf-8"))
        validate_manifest(document)


if __name__ == "__main__":
    unittest.main()
