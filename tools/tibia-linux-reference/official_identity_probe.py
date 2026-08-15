#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import tarfile
import tempfile
from pathlib import Path, PurePosixPath

OFFICIAL_SOURCE = "https://static.tibia.com/download/tibia.x64.tar.gz"
ELF_MAGIC = b"\x7fELF"
BUILD_ID_RE = re.compile(r"Build ID:\s*([0-9a-fA-F]+)")
VERSION_TOKEN_RE = re.compile(rb"(?<![0-9A-Za-z])(?:1[0-9]|2[0-9])\.[0-9]{1,2}(?:\.[0-9A-Za-z]{1,12}){1,3}(?![0-9A-Za-z])")
MAX_VERSION_TOKENS = 24


def sha256_stream(handle) -> tuple[str, int, bytes]:
    digest = hashlib.sha256()
    total = 0
    prefix = bytearray()
    while True:
        block = handle.read(1024 * 1024)
        if not block:
            break
        digest.update(block)
        total += len(block)
        if len(prefix) < 4:
            prefix.extend(block[: 4 - len(prefix)])
    return digest.hexdigest(), total, bytes(prefix)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def safe_member_name(name: str) -> str:
    path = PurePosixPath(name)
    if path.is_absolute() or ".." in path.parts or not path.parts:
        raise ValueError("archive contains an unsafe member path")
    return path.as_posix()


def readelf_build_id(path: Path) -> str | None:
    completed = subprocess.run(
        ["readelf", "-n", str(path)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if completed.returncode != 0:
        return None
    match = BUILD_ID_RE.search(completed.stdout)
    return match.group(1).lower() if match else None


def file_description(path: Path) -> str:
    completed = subprocess.run(
        ["file", "-b", "--", str(path)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return completed.stdout.strip()[:300] if completed.returncode == 0 else "unknown"


def version_tokens(path: Path) -> list[str]:
    data = path.read_bytes()
    tokens = sorted({match.decode("ascii", errors="ignore") for match in VERSION_TOKEN_RE.findall(data)})
    return tokens[:MAX_VERSION_TOKENS]


def inspect_archive(archive: Path, source_url: str) -> dict[str, object]:
    if source_url != OFFICIAL_SOURCE:
        raise ValueError("source URL is not the approved official CipSoft Linux package endpoint")
    if not archive.is_file():
        raise ValueError("archive does not exist")

    package_sha = sha256_file(archive)
    package_size = archive.stat().st_size
    members: list[str] = []
    elfs: list[dict[str, object]] = []
    launcher_paths: list[str] = []

    with tarfile.open(archive, mode="r:gz") as bundle, tempfile.TemporaryDirectory(prefix="tibia-identity-") as tmpdir:
        temp_root = Path(tmpdir)
        for member in bundle.getmembers():
            safe_name = safe_member_name(member.name)
            members.append(safe_name)
            lowered = safe_name.lower()
            if "launcher" in lowered or lowered.endswith("/tibia") or lowered.endswith("/tibia.bin"):
                launcher_paths.append(safe_name)
            if not member.isfile():
                continue
            extracted = bundle.extractfile(member)
            if extracted is None:
                raise ValueError("regular archive member could not be read")
            digest, size, prefix = sha256_stream(extracted)
            if prefix != ELF_MAGIC:
                continue
            extracted = bundle.extractfile(member)
            if extracted is None:
                raise ValueError("ELF archive member could not be re-read")
            temp_path = temp_root / f"elf-{len(elfs)}"
            with temp_path.open("wb") as handle:
                while True:
                    block = extracted.read(1024 * 1024)
                    if not block:
                        break
                    handle.write(block)
            os.chmod(temp_path, 0o500)
            elfs.append(
                {
                    "path": safe_name,
                    "sha256": digest,
                    "size_bytes": size,
                    "archive_mode_octal": oct(member.mode & 0o777),
                    "build_id": readelf_build_id(temp_path),
                    "file": file_description(temp_path),
                    "version_tokens": version_tokens(temp_path),
                }
            )

    return {
        "schema_version": 1,
        "source_url": source_url,
        "package_filename": archive.name,
        "package_sha256": package_sha,
        "package_size_bytes": package_size,
        "archive_member_count": len(members),
        "archive_members": sorted(members),
        "launcher_or_client_paths": sorted(set(launcher_paths)),
        "elf_files": elfs,
        "binary_execution_performed": False,
        "credentials_used": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--source-url", required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    document = inspect_archive(args.archive, args.source_url)
    encoded = json.dumps(document, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
