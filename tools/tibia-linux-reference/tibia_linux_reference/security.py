from __future__ import annotations

import hashlib
import json
import os
import re
import stat
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Mapping, Sequence

from . import HarnessError


HIGH_CONFIDENCE_SECRET_PATTERNS = (
    re.compile(rb"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----"),
    re.compile(rb"\bgh[opusr]_[A-Za-z0-9_]{20,}\b"),
    re.compile(rb"\bgithub_pat_[A-Za-z0-9_]{20,}\b"),
    re.compile(rb"\bAKIA[0-9A-Z]{16}\b"),
    re.compile(rb"\bBearer\s+[A-Za-z0-9._~+/=-]{16,}\b", re.IGNORECASE),
)
SENSITIVE_ENV_KEY = re.compile(
    r"(?:^|_)(?:PASSWORD|PASSWD|TOKEN|SECRET|COOKIE|SESSION_KEY|AUTHENTICATOR|CREDENTIAL)(?:_|$)",
    re.IGNORECASE,
)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def is_within(path: Path, parent: Path) -> bool:
    try:
        path.resolve(strict=False).relative_to(parent.resolve(strict=False))
        return True
    except ValueError:
        return False


def require_private_directory(path: Path, repo_root: Path) -> None:
    resolved = path.resolve(strict=False)
    if is_within(resolved, repo_root):
        raise HarnessError("evidence directory must be outside the Git checkout")
    resolved.mkdir(mode=0o700, parents=True, exist_ok=True)
    os.chmod(resolved, 0o700)
    details = resolved.stat()
    if details.st_uid != os.getuid():
        raise HarnessError("evidence directory must be owned by the harness user")
    if stat.S_IMODE(details.st_mode) & 0o077:
        raise HarnessError("evidence directory must not grant group or other permissions")


def write_private_json(path: Path, payload: Mapping[str, object]) -> None:
    encoded = (json.dumps(payload, indent=2, sort_keys=True) + "\n").encode("utf-8")
    temporary = path.with_suffix(path.suffix + ".tmp")
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(encoded)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        if temporary.exists():
            temporary.unlink()


def exact_secret_hits(data: bytes, secrets: Sequence[str]) -> list[str]:
    hits: list[str] = []
    for index, secret in enumerate(secrets):
        if secret and secret.encode("utf-8") in data:
            hits.append(f"synthetic-secret-{index + 1}")
    return hits


def high_confidence_secret_present(data: bytes) -> bool:
    return any(pattern.search(data) for pattern in HIGH_CONFIDENCE_SECRET_PATTERNS)


def redact_text(text: str, sensitive_values: Sequence[str]) -> str:
    redacted = text
    for value in sorted((value for value in sensitive_values if value), key=len, reverse=True):
        redacted = redacted.replace(value, "[REDACTED]")
    encoded = redacted.encode("utf-8", errors="replace")
    for pattern in HIGH_CONFIDENCE_SECRET_PATTERNS:
        encoded = pattern.sub(b"[REDACTED]", encoded)
    return encoded.decode("utf-8", errors="replace")


def validate_no_secret_like_environment(environment: Mapping[str, str]) -> None:
    bad_keys = sorted(key for key, value in environment.items() if value and SENSITIVE_ENV_KEY.search(key))
    bad_values = sorted(key for key, value in environment.items() if value and high_confidence_secret_present(value.encode()))
    if bad_keys or bad_values:
        names = ", ".join(sorted(set(bad_keys + bad_values)))
        raise HarnessError(f"ordinary environment contains secret-like material in variable(s): {names}")


def validate_no_shell_trace(environment: Mapping[str, str], parent_cmdline: bytes | None = None) -> None:
    shellopts = environment.get("SHELLOPTS", "").split(":")
    if "xtrace" in shellopts or "verbose" in shellopts or environment.get("BASH_XTRACEFD"):
        raise HarnessError("shell tracing or verbose command echo is enabled")
    if parent_cmdline and (b"bash\x00-x" in parent_cmdline or b"sh\x00-x" in parent_cmdline):
        raise HarnessError("parent shell was launched with command tracing")


def git_bytes(repo_root: Path, *arguments: str) -> bytes:
    completed = subprocess.run(
        ["git", "-C", str(repo_root), *arguments],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return completed.stdout


@dataclass(frozen=True)
class LeakScanResult:
    passed: bool
    categories: tuple[str, ...]
    files_scanned: int


def scan_prohibited_locations(
    *,
    repo_root: Path,
    evidence_root: Path,
    temporary_root: Path | None,
    secrets: Sequence[str],
    process_arguments: bytes,
    retained_environment_report: bytes,
    stdout: bytes,
    stderr: bytes,
) -> LeakScanResult:
    categories: set[str] = set()
    files_scanned = 0

    def inspect(category: str, data: bytes, *, generic: bool = True) -> None:
        if exact_secret_hits(data, secrets) or (generic and high_confidence_secret_present(data)):
            categories.add(category)

    inspect("git-diff", git_bytes(repo_root, "diff", "--binary"))
    inspect("git-diff", git_bytes(repo_root, "diff", "--cached", "--binary"))
    base_check = subprocess.run(
        ["git", "-C", str(repo_root), "rev-parse", "--verify", "origin/main"],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    if base_check.returncode == 0:
        inspect("git-diff", git_bytes(repo_root, "diff", "--binary", "origin/main...HEAD"))
    else:
        categories.add("git-base-unavailable")
    tracked = git_bytes(repo_root, "ls-files", "-z").split(b"\0")
    for relative in tracked:
        if not relative:
            continue
        candidate = repo_root / os.fsdecode(relative)
        if candidate.is_file():
            # The repository contains pre-existing synthetic credential fixtures. Exact run values
            # must be absent everywhere; generic token patterns are fail-closed for the current
            # branch diff and retained outputs without reclassifying unchanged baseline fixtures.
            inspect("tracked-files", candidate.read_bytes(), generic=False)
            files_scanned += 1

    for root, category in ((evidence_root, "evidence-or-artifacts"), (temporary_root, "temporary-files")):
        if root and root.exists():
            for candidate in root.rglob("*"):
                if candidate.is_file():
                    inspect(category, candidate.read_bytes())
                    files_scanned += 1

    inspect("process-arguments", process_arguments)
    inspect("retained-environment-report", retained_environment_report)
    inspect("stdout", stdout)
    inspect("stderr", stderr)

    history_candidates = [Path.home() / ".bash_history", Path.home() / ".zsh_history"]
    histfile = os.environ.get("HISTFILE")
    if histfile:
        history_candidates.append(Path(histfile))
    for candidate in history_candidates:
        if candidate.is_file():
            inspect("shell-history", candidate.read_bytes())
            files_scanned += 1

    return LeakScanResult(not categories, tuple(sorted(categories)), files_scanned)


def scan_configuration_bytes(data: bytes) -> None:
    if high_confidence_secret_present(data):
        raise HarnessError("configuration contains high-confidence secret-like material")


def sanitized_child_environment(profile: Path) -> dict[str, str]:
    allowed = {
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "XDG_RUNTIME_DIR",
        "XDG_SESSION_TYPE",
        "LANG",
        "LC_ALL",
        "PATH",
    }
    child = {key: value for key, value in os.environ.items() if key in allowed and value}
    child["HOME"] = str(profile)
    child["TMPDIR"] = str(profile / "tmp")
    child["HISTFILE"] = "/dev/null"
    child.pop("LD_PRELOAD", None)
    child.pop("LD_LIBRARY_PATH", None)
    return child


def read_proc_bytes(pid: int, name: str) -> bytes:
    try:
        return (Path("/proc") / str(pid) / name).read_bytes()
    except (FileNotFoundError, PermissionError):
        return b""


def ensure_no_injection_environment(environment: Mapping[str, str]) -> None:
    forbidden = [key for key in ("LD_PRELOAD", "LD_AUDIT") if environment.get(key)]
    if forbidden:
        raise HarnessError(f"prohibited injection environment present: {', '.join(forbidden)}")
