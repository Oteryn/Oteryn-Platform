from __future__ import annotations

import json
import os
import re
import subprocess
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

from . import HarnessError
from .security import is_within, scan_configuration_bytes, sha256_file


SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
BUILD_ID_RE = re.compile(r"^[0-9a-f]{16,64}$")


@dataclass(frozen=True)
class ApprovedIdentity:
    client_version: str
    package_sha256: str
    executable_sha256: str
    elf_build_id: str
    package_source: str

    @classmethod
    def load(cls, path: Path) -> "ApprovedIdentity":
        raw = path.read_bytes()
        scan_configuration_bytes(raw)
        try:
            document = json.loads(raw)
        except json.JSONDecodeError as error:
            raise HarnessError(f"identity configuration is invalid JSON: {error}") from error
        required = {
            "client_version",
            "package_sha256",
            "executable_sha256",
            "elf_build_id",
            "package_source",
        }
        if set(document) != required:
            raise HarnessError("identity configuration must contain exactly the approved identity fields")
        values = {key: str(document[key]).strip() for key in required}
        if not SHA256_RE.fullmatch(values["package_sha256"]):
            raise HarnessError("approved package SHA-256 is invalid")
        if not SHA256_RE.fullmatch(values["executable_sha256"]):
            raise HarnessError("approved executable SHA-256 is invalid")
        if not BUILD_ID_RE.fullmatch(values["elf_build_id"]):
            raise HarnessError("approved ELF Build ID is invalid")
        if not values["client_version"] or not values["package_source"]:
            raise HarnessError("approved client version and package source are required")
        return cls(**values)


def elf_build_id(executable: Path) -> str:
    completed = subprocess.run(
        ["readelf", "-n", str(executable)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if completed.returncode != 0:
        raise HarnessError("readelf could not inspect the client executable")
    match = re.search(r"Build ID:\s*([0-9a-fA-F]+)", completed.stdout)
    if not match:
        raise HarnessError("client executable does not expose an ELF Build ID")
    return match.group(1).lower()


def verify_client_identity(
    approved: ApprovedIdentity,
    *,
    package_path: Path,
    executable_path: Path,
    repo_root: Path,
) -> dict[str, object]:
    package = package_path.resolve(strict=True)
    executable = executable_path.resolve(strict=True)
    if is_within(package, repo_root) or is_within(executable, repo_root):
        raise HarnessError("official package and executable must remain outside the Git checkout")
    if not package.is_file() or not executable.is_file():
        raise HarnessError("official package and executable must be regular files")
    if not os.access(executable, os.X_OK):
        raise HarnessError("official client executable is not executable")

    actual_package = sha256_file(package)
    actual_executable = sha256_file(executable)
    actual_build_id = elf_build_id(executable)
    mismatches = []
    if actual_package != approved.package_sha256:
        mismatches.append("package SHA-256")
    if actual_executable != approved.executable_sha256:
        mismatches.append("executable SHA-256")
    if actual_build_id != approved.elf_build_id:
        mismatches.append("ELF Build ID")
    if mismatches:
        raise HarnessError("official client identity mismatch: " + ", ".join(mismatches))

    return {
        "client_version": approved.client_version,
        "package_sha256": actual_package,
        "executable_sha256": actual_executable,
        "elf_build_id": actual_build_id,
        "package_source": approved.package_source,
        "package_path_classification": "private-outside-git",
        "executable_path_classification": "private-outside-git",
        "verified_at": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "binary_modified": False,
    }
