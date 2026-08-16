from __future__ import annotations

from pathlib import Path

from .launcher import run_official_component as _run_official_component
from .official_host import run_official_host_preflight


def run_official_component(
    *,
    repo_root: Path,
    evidence_root: Path,
    identity_file: Path,
    package_path: Path,
    executable_path: Path,
    observation_seconds: float,
) -> dict[str, object]:
    """Enforce dedicated-host gates before delegating to the bounded launcher."""
    run_official_host_preflight(repo_root=repo_root, evidence_root=evidence_root)
    return _run_official_component(
        repo_root=repo_root,
        evidence_root=evidence_root,
        identity_file=identity_file,
        package_path=package_path,
        executable_path=executable_path,
        observation_seconds=observation_seconds,
    )
