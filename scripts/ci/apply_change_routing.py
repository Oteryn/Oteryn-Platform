#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
TEMP_WORKFLOW = REPOSITORY_ROOT / ".github/workflows/ci-routing-bootstrap.yml"

WORKFLOWS = {
    ".github/workflows/ci.yml": ("test", "ci"),
    ".github/workflows/phase7-production-like-validation.yml": ("validate", "phase7"),
    ".github/workflows/edge-security-emulation.yml": ("validate", "edge"),
    ".github/workflows/platform-db-outage-validation.yml": ("validate", "db_outage"),
    ".github/workflows/game-auth-ticket-concurrency.yml": (
        "concurrency-proof",
        "game_auth_concurrency",
    ),
}

CLASSIFIER_JOB = """  classify_changes:
    name: classify-changes
    runs-on: ubuntu-latest
    outputs:
      ci: ${{ steps.classify.outputs.ci }}
      phase7: ${{ steps.classify.outputs.phase7 }}
      edge: ${{ steps.classify.outputs.edge }}
      db_outage: ${{ steps.classify.outputs.db_outage }}
      game_auth_concurrency: ${{ steps.classify.outputs.game_auth_concurrency }}
      classes: ${{ steps.classify.outputs.classes }}
    steps:
      - name: Checkout exact classification range
        uses: actions/checkout@v7
        with:
          ref: ${{ github.event.pull_request.head.sha || github.sha }}
          fetch-depth: 0

      - name: Classify changed paths
        id: classify
        env:
          EVENT_NAME: ${{ github.event_name }}
          BASE_SHA: ${{ github.event.pull_request.base.sha || '' }}
          HEAD_SHA: ${{ github.event.pull_request.head.sha || github.sha }}
        run: |
          if [[ "$EVENT_NAME" == "pull_request" ]]; then
            python scripts/ci/classify_changes.py \
              --base "$BASE_SHA" \
              --head "$HEAD_SHA" \
              --github-output "$GITHUB_OUTPUT" \
              --summary "$GITHUB_STEP_SUMMARY"
          else
            python scripts/ci/classify_changes.py \
              --all \
              --github-output "$GITHUB_OUTPUT" \
              --summary "$GITHUB_STEP_SUMMARY"
          fi

"""

FAIL_CLOSED_STEP = """      - name: Require successful change classification
        if: ${{ needs.classify_changes.result != 'success' }}
        run: |
          echo "Change classification failed; this required gate is failing closed."
          exit 1

"""


def patch_workflow(relative_path: str, job_id: str, gate: str) -> None:
    path = REPOSITORY_ROOT / relative_path
    text = path.read_text(encoding="utf-8")

    if "  classify_changes:\n" in text:
        raise RuntimeError(f"{relative_path}: classifier job already exists")

    jobs_marker = "jobs:\n"
    job_marker = f"  {job_id}:\n"
    if text.count(jobs_marker) != 1:
        raise RuntimeError(f"{relative_path}: expected exactly one jobs marker")
    if text.count(job_marker) != 1:
        raise RuntimeError(f"{relative_path}: expected exactly one {job_id} job")

    job_replacement = (
        job_marker
        + "    needs: classify_changes\n"
        + "    if: ${{ always() && (needs.classify_changes.result != 'success' || "
        + f"needs.classify_changes.outputs.{gate} == 'true') }}\n"
    )
    text = text.replace(job_marker, job_replacement, 1)

    job_start = text.index(job_replacement)
    steps_marker = "    steps:\n"
    steps_start = text.index(steps_marker, job_start) + len(steps_marker)
    text = text[:steps_start] + FAIL_CLOSED_STEP + text[steps_start:]

    jobs_start = text.index(jobs_marker) + len(jobs_marker)
    text = text[:jobs_start] + CLASSIFIER_JOB + text[jobs_start:]

    if text.count("  classify_changes:\n") != 1:
        raise RuntimeError(f"{relative_path}: classifier insertion was not unique")
    if text.count("needs: classify_changes") != 1:
        raise RuntimeError(f"{relative_path}: required job dependency was not unique")
    if text.count("Require successful change classification") != 1:
        raise RuntimeError(f"{relative_path}: fail-closed step was not unique")

    path.write_text(text, encoding="utf-8")


def main() -> int:
    for relative_path, (job_id, gate) in WORKFLOWS.items():
        patch_workflow(relative_path, job_id, gate)

    TEMP_WORKFLOW.unlink()
    Path(__file__).unlink()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
