from __future__ import annotations

import io
import json
import os
import stat
import tempfile
import unittest
import uuid
from pathlib import Path
from unittest import mock

from tibia_linux_reference import HarnessError
from tibia_linux_reference.fake_client import read_secret_channel
from tibia_linux_reference.identity import ApprovedIdentity, verify_client_identity
from tibia_linux_reference.launcher import _parse_events, verify_cleanup
from tibia_linux_reference.manifest import SAFETY_FALSE_FIELDS, validate_manifest
from tibia_linux_reference.preflight import _network_isolator
from tibia_linux_reference.security import (
    redact_text,
    require_private_directory,
    sanitized_child_environment,
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

    def test_github_runtime_token_environment_is_rejected(self) -> None:
        with self.assertRaisesRegex(HarnessError, "ACTIONS_RUNTIME_TOKEN"):
            validate_no_secret_like_environment(
                {"ACTIONS_RUNTIME_TOKEN": "runtime-value-that-must-never-enter-the-harness"}
            )

    def test_sanitized_child_environment_drops_unapproved_values(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory)
            (profile / "tmp").mkdir()
            fake_environment = {
                "DISPLAY": "unix/:99",
                "PATH": "/usr/bin",
                "ACTIONS_RUNTIME_TOKEN": "must-not-survive",
                "UNRELATED": "must-not-survive",
            }
            with mock.patch.dict(os.environ, fake_environment, clear=True):
                child = sanitized_child_environment(profile)
        self.assertEqual(child["DISPLAY"], "unix/:99")
        self.assertEqual(child["PATH"], "/usr/bin")
        self.assertEqual(child["HISTFILE"], "/dev/null")
        self.assertNotIn("ACTIONS_RUNTIME_TOKEN", child)
        self.assertNotIn("UNRELATED", child)

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

    def test_high_confidence_new_evidence_is_rejected_without_baseline_false_positive(self) -> None:
        token = "gho_" + ("b" * 26)
        with tempfile.TemporaryDirectory() as directory:
            evidence = Path(directory)
            clean = scan_prohibited_locations(
                repo_root=REPO,
                evidence_root=evidence,
                temporary_root=None,
                secrets=["unique-synthetic-run-value-" + uuid.uuid4().hex],
                process_arguments=b"safe",
                retained_environment_report=b"safe",
                stdout=b"safe",
                stderr=b"safe",
            )
            self.assertTrue(clean.passed)
            (evidence / "new.log").write_text(token, encoding="utf-8")
            rejected = scan_prohibited_locations(
                repo_root=REPO,
                evidence_root=evidence,
                temporary_root=None,
                secrets=[],
                process_arguments=b"safe",
                retained_environment_report=b"safe",
                stdout=b"safe",
                stderr=b"safe",
            )
        self.assertFalse(rejected.passed)
        self.assertIn("evidence-or-artifacts", rejected.categories)

    def test_fake_client_consumes_synthetic_corpus_from_stdin_stream(self) -> None:
        payload = b"\n".join(
            [
                b"synthetic-login-value-1234567890",
                b"synthetic-password-value-1234567890",
                b"synthetic-token-value-1234567890",
                b"synthetic-auth-value-1234567890",
            ]
        ) + b"\n"
        with mock.patch("tibia_linux_reference.fake_client.emit") as emit:
            read_secret_channel(io.BytesIO(payload))
        emit.assert_called_once_with("credential_channel_consumed", mechanism="anonymous-pipe", value_count=4)

    def test_privileged_network_fallback_preserves_only_prevalidated_environment(self) -> None:
        with (
            mock.patch(
                "tibia_linux_reference.preflight.subprocess.run",
                side_effect=[mock.Mock(returncode=1), mock.Mock(returncode=0)],
            ),
            mock.patch("tibia_linux_reference.preflight.shutil.which", return_value="/usr/bin/sudo"),
            mock.patch("tibia_linux_reference.preflight.os.getuid", return_value=1001),
            mock.patch("tibia_linux_reference.preflight.os.getgid", return_value=1001),
        ):
            result = _network_isolator()
        self.assertEqual(result["kind"], "privileged-namespace-setup-with-uid-drop")
        self.assertEqual(result["prefix"][:4], ["/usr/bin/sudo", "-n", "-E", "unshare"])


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
                b'{"denied":true,"endpoint_classification":"RFC5737-TEST-NET-2+RFC3849+RFC2606.invalid","error_class":"OSError","event":"network_denial","interfaces":["lo"],"namespace":"net:[1]"}',
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

    def test_launcher_uses_stdio_pipe_instead_of_non_stdio_inherited_descriptor(self) -> None:
        launcher = (ROOT / "tibia_linux_reference" / "launcher.py").read_text(encoding="utf-8")
        self.assertIn("stdin=subprocess.PIPE", launcher)
        self.assertNotIn("pass_fds=", launcher)
        self.assertNotIn("--secret-fd", launcher)


class ManifestTests(unittest.TestCase):
    def _synthetic_document(self) -> dict[str, object]:
        return json.loads((ROOT / "examples" / "session-manifest.synthetic.json").read_text(encoding="utf-8"))

    def test_synthetic_example_matches_fail_closed_manifest_contract(self) -> None:
        validate_manifest(self._synthetic_document())

    def test_non_fail_closed_safety_is_rejected(self) -> None:
        document = self._synthetic_document()
        document["safety"]["official_service_contacted"] = True
        with self.assertRaises(HarnessError):
            validate_manifest(document)

    def test_unproven_network_denial_is_rejected(self) -> None:
        document = self._synthetic_document()
        document["network_denial"]["proven"] = False
        with self.assertRaises(HarnessError):
            validate_manifest(document)

    def test_schema_safety_contract_matches_runtime_validator(self) -> None:
        schema = json.loads((ROOT / "schema" / "session-manifest.schema.json").read_text(encoding="utf-8"))
        safety = schema["properties"]["safety"]
        self.assertEqual(set(safety["required"]), SAFETY_FALSE_FIELDS)
        for field in SAFETY_FALSE_FIELDS:
            self.assertIs(safety["properties"][field]["const"], False)
        network = schema["properties"]["network_denial"]
        self.assertIs(network["properties"]["proven"]["const"], True)
        self.assertIs(network["properties"]["raw_capture_created"]["const"], False)
        self.assertIs(network["properties"]["official_endpoint_contacted"]["const"], False)


class WorkflowTests(unittest.TestCase):
    def test_ci_harness_process_is_environment_sanitized(self) -> None:
        workflow = (REPO / ".github" / "workflows" / "tibia-linux-live-reference.yml").read_text(encoding="utf-8")
        self.assertIn("env -i", workflow)
        self.assertIn('DISPLAY="unix/:99"', workflow)
        self.assertIn("ulimit -c 0", workflow)
        self.assertIn("persist-credentials: false", workflow)
        self.assertIn('rm -f "$xvfb_log"', workflow)
        self.assertNotIn("upload-artifact", workflow)

    def test_ci_never_invokes_official_component_mode(self) -> None:
        workflow = (REPO / ".github" / "workflows" / "tibia-linux-live-reference.yml").read_text(encoding="utf-8")
        self.assertNotIn("official-component", workflow)
        self.assertNotIn("verify-identity", workflow)

    def test_task_checkpoint_changes_do_not_trigger_heavy_harness(self) -> None:
        workflow = (REPO / ".github" / "workflows" / "tibia-linux-live-reference.yml").read_text(encoding="utf-8")
        self.assertNotIn("docs/agents/tasks/", workflow)
        self.assertNotIn("docs/agents/reports/", workflow)


if __name__ == "__main__":
    unittest.main()
