#!/usr/bin/env python3
from __future__ import annotations

import sys
from typing import Any

import branch_lifecycle_legacy as legacy
from branch_lifecycle_legacy import *  # noqa: F401,F403

ValidationError = legacy.ValidationError
ApiError = legacy.ApiError


class GitHubClient(legacy.GitHubClient):
    """Compatibility facade that reconciles post-delete REST lag via Git transport."""

    def _deleted_refs(self) -> set[str]:
        refs = getattr(self, "_git_verified_deleted_refs", None)
        if refs is None:
            refs = set()
            setattr(self, "_git_verified_deleted_refs", refs)
        return refs

    def _git_remote_ref_sha(self, branch: str) -> str | None:
        ref = f"refs/heads/{branch}"
        remote = self._validated_git_remote()
        result = self._run_git(
            ["git", "ls-remote", "--refs", remote, ref],
            timeout=60,
            purpose=f"post-delete remote verification for {branch}",
        )
        if result.returncode != 0:
            raise ValidationError(
                f"post-delete git remote verification failed for {branch} "
                f"with exit code {result.returncode}"
            )
        matches: list[str] = []
        for raw in result.stdout.splitlines():
            line = raw.strip()
            if not line:
                continue
            fields = line.split()
            if len(fields) != 2 or fields[1] != ref:
                raise ValidationError(
                    f"post-delete git remote verification returned unexpected ref data for {branch}"
                )
            sha = fields[0]
            if not legacy.FULL_SHA_RE.fullmatch(sha):
                raise ValidationError(
                    f"post-delete git remote verification returned invalid SHA for {branch}"
                )
            matches.append(sha)
        if len(matches) > 1:
            raise ValidationError(
                f"post-delete git remote verification returned multiple exact refs for {branch}"
            )
        return matches[0] if matches else None

    def delete_branch(self, branch: str, *, expected_sha: str | None = None) -> None:
        super().delete_branch(branch, expected_sha=expected_sha)
        self._deleted_refs().add(branch)

    def create_branch(self, branch: str, sha: str) -> None:
        self._deleted_refs().discard(branch)
        super().create_branch(branch, sha)

    def get_ref(self, branch: str) -> dict[str, Any] | None:
        if branch in self._deleted_refs():
            remote_sha = self._git_remote_ref_sha(branch)
            if remote_sha is None:
                return None
            self._deleted_refs().discard(branch)
        return super().get_ref(branch)


legacy.GitHubClient = GitHubClient


def __getattr__(name: str) -> Any:
    return getattr(legacy, name)


def main(argv: list[str] | None = None) -> int:
    legacy.GitHubClient = GitHubClient
    return legacy.main(argv)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValidationError, ApiError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
