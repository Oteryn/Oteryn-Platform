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


if __name__ == "__main__":
    unittest.main()
