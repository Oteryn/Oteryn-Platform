#!/usr/bin/env python3
from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import control_room as room


class ControlRoomLivenessTests(unittest.TestCase):
    def make_task(self) -> room.Task:
        return room.Task(
            task_id="OTERYN-TEST-live",
            lane="oteryn-platform-core",
            path="docs/agents/tasks/active/OTERYN-TEST-live.md",
            raw_status="implementing",
            state="RUNNING",
            updated_at="2026-08-07T10:00:00+02:00",
            age_minutes=0,
            branch="repair/live",
            pr="12",
            phase="implement",
            execution_mode="github-only",
            next_action="Continue implementation.",
            blocker="",
            task_kind="implementation",
            context_pressure="low",
            context_growth="stable",
            decomposition_decision="single",
            estimate_confidence="high",
            session_id="test",
            session_role="implementer",
            validation_level="focused",
            policy_version="2",
            policy_status="V2",
            session_rotation_count=0,
            heavy_validation_runs=0,
            stale_takeover_count=0,
            human_interruptions=0,
        )

    def test_live_invalid_is_distinct_from_local_running_state(self):
        task = self.make_task()
        updated = room.apply_liveness(
            [task],
            {
                task.task_id: {
                    "task_id": task.task_id,
                    "live_valid": False,
                    "live_state": "TERMINAL_ARCHIVE_PENDING",
                    "findings": [
                        {
                            "severity": "error",
                            "code": "terminal_pr_active_task",
                            "message": "terminal PR is still represented as active",
                        }
                    ],
                }
            },
        )[0]
        self.assertEqual(updated.state, "RUNNING")
        self.assertFalse(updated.live_valid)
        self.assertEqual(updated.live_state, "TERMINAL_ARCHIVE_PENDING")
        self.assertIn("terminal_pr_active_task", updated.live_findings[0])
        metrics = room.coordination_metrics([updated])
        self.assertEqual(metrics["active_tasks"], 1)
        self.assertEqual(metrics["live_invalid_tasks"], 1)

    def test_missing_task_in_report_fails_live_visibility(self):
        updated = room.apply_liveness([self.make_task()], {})[0]
        self.assertFalse(updated.live_valid)
        self.assertEqual(updated.live_state, "MISSING_FROM_REPORT")

    def test_report_loader_validates_shape(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "report.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "tasks": [
                            {
                                "task_id": "OTERYN-TEST-live",
                                "live_valid": True,
                                "live_state": "OPEN_PR",
                                "findings": [],
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            loaded = room.load_liveness_report(path)
        self.assertTrue(loaded["OTERYN-TEST-live"]["live_valid"])

    def test_markdown_surfaces_live_contradiction_independent_of_staleness(self):
        task = room.apply_liveness(
            [self.make_task()],
            {
                "OTERYN-TEST-live": {
                    "task_id": "OTERYN-TEST-live",
                    "live_valid": False,
                    "live_state": "OPEN_PR",
                    "findings": [
                        {
                            "severity": "error",
                            "code": "branch_pr_mismatch",
                            "message": "branch does not match",
                        }
                    ],
                }
            },
        )[0]
        config = {
            "repository": "blakinio/Oteryn-Platform",
            "lanes": [{"id": "oteryn-platform-core"}],
            "rollout": {"enforcement_mode": "advisory"},
        }
        output = room.markdown(config, [task], 45)
        self.assertIn("live=INVALID/OPEN_PR", output)
        self.assertIn("live contradiction: branch_pr_mismatch", output)


if __name__ == "__main__":
    unittest.main()
