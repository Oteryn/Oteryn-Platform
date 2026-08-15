from __future__ import annotations

import importlib.util
import io
import os
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
PACKAGE_ROOT = ROOT / "tibia_linux_reference"
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


identity_probe = load_module("official_identity_probe", ROOT / "official_identity_probe.py")
host_preflight = load_module("official_host_preflight", ROOT / "official_host_preflight.py")
from tibia_linux_reference import HarnessError  # noqa: E402


class OfficialIdentityProbeTests(unittest.TestCase):
    def make_archive(self, directory: Path, arcname: str = "Tibia/Tibia") -> Path:
        archive = directory / "tibia.x64.tar.gz"
        executable = Path(sys.executable)
        data = executable.read_bytes()
        with tarfile.open(archive, "w:gz") as bundle:
            info = tarfile.TarInfo(arcname)
            info.mode = 0o755
            info.size = len(data)
            bundle.addfile(info, io.BytesIO(data))
        return archive

    def test_probe_hashes_elf_without_executing_it(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            archive = self.make_archive(Path(directory))
            document = identity_probe.inspect_archive(archive, identity_probe.OFFICIAL_SOURCE)
        self.assertFalse(document["binary_execution_performed"])
        self.assertFalse(document["credentials_used"])
        self.assertEqual(document["launcher_or_client_paths"], ["Tibia/Tibia"])
        self.assertEqual(len(document["elf_files"]), 1)
        self.assertEqual(document["elf_files"][0]["path"], "Tibia/Tibia")
        self.assertEqual(len(document["elf_files"][0]["sha256"]), 64)

    def test_probe_rejects_non_official_source(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            archive = self.make_archive(Path(directory))
            with self.assertRaisesRegex(ValueError, "approved official"):
                identity_probe.inspect_archive(archive, "https://example.invalid/tibia.tar.gz")

    def test_probe_rejects_archive_path_traversal(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            archive = self.make_archive(Path(directory), "../escape")
            with self.assertRaisesRegex(ValueError, "unsafe member"):
                identity_probe.inspect_archive(archive, identity_probe.OFFICIAL_SOURCE)


class OfficialHostPreflightTests(unittest.TestCase):
    def test_ci_environment_is_rejected_before_official_execution(self) -> None:
        with mock.patch.dict(os.environ, {"CI": "true"}, clear=True):
            with self.assertRaisesRegex(HarnessError, "CI runner"):
                host_preflight.require_normal_host("oteryn-tibia-ref")

    def test_container_marker_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / ".dockerenv"
            marker.touch()
            with (
                mock.patch.dict(os.environ, {}, clear=True),
                mock.patch.object(host_preflight, "CONTAINER_MARKERS", (marker,)),
            ):
                with self.assertRaisesRegex(HarnessError, "containers"):
                    host_preflight.require_normal_host("oteryn-tibia-ref")

    def test_software_only_gl_renderer_is_rejected(self) -> None:
        completed = mock.Mock(
            returncode=0,
            stdout="direct rendering: Yes\nOpenGL renderer string: llvmpipe (LLVM 19.1.1, 256 bits)\n",
            stderr="",
        )
        with (
            mock.patch.dict(os.environ, {"DISPLAY": ":0"}, clear=True),
            mock.patch.object(host_preflight.shutil, "which", return_value="/usr/bin/glxinfo"),
            mock.patch.object(host_preflight.subprocess, "run", return_value=completed),
        ):
            with self.assertRaisesRegex(HarnessError, "software-only"):
                host_preflight.require_accelerated_graphics()

    def test_direct_hardware_gl_renderer_is_accepted(self) -> None:
        completed = mock.Mock(
            returncode=0,
            stdout="direct rendering: Yes\nOpenGL renderer string: AMD Radeon RX test (radeonsi)\n",
            stderr="",
        )
        with (
            mock.patch.dict(os.environ, {"DISPLAY": ":0"}, clear=True),
            mock.patch.object(host_preflight.shutil, "which", return_value="/usr/bin/glxinfo"),
            mock.patch.object(host_preflight.subprocess, "run", return_value=completed),
        ):
            result = host_preflight.require_accelerated_graphics()
        self.assertTrue(result["direct_rendering"])
        self.assertFalse(result["software_renderer"])
        self.assertIn("AMD Radeon", result["renderer"])


class HostSetupScriptSafetyTests(unittest.TestCase):
    def test_luks_setup_is_interactive_blank_disk_only_and_fail_closed(self) -> None:
        script = (ROOT / "official_evidence_luks_setup.sh").read_text(encoding="utf-8")
        self.assertIn('"DESTROY:$device"', script)
        self.assertIn("[[ -t 0 && -t 1 ]]", script)
        self.assertIn('wipefs -n "$device"', script)
        self.assertIn('findmnt -n -o SOURCE /', script)
        self.assertIn('umount "$mountpoint"', script)
        self.assertIn('cryptsetup close "$mapper"', script)
        self.assertNotIn("--key-file", script)
        self.assertNotIn("echo -n", script)

    def test_host_prepare_creates_no_secret_and_no_admin_membership(self) -> None:
        script = (ROOT / "official_host_prepare.sh").read_text(encoding="utf-8")
        self.assertIn("useradd --create-home --user-group", script)
        self.assertIn("sudo adm docker lxd", script)
        self.assertNotIn("usermod -aG sudo", script)
        self.assertNotIn("chpasswd", script)
        self.assertNotIn("passwd ", script)
        self.assertIn('"access_credential_generated":false', script)


if __name__ == "__main__":
    unittest.main()
