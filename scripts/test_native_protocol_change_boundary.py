#!/usr/bin/env python3
from __future__ import annotations

import unittest

from validate_native_protocol_change_boundary import (
    PRODUCER_TASK,
    BoundaryViolation,
    evaluate_changed_paths,
)


class NativeProtocolChangeBoundaryTest(unittest.TestCase):
    def test_unrelated_contract_and_runtime_are_not_applicable(self) -> None:
        result = evaluate_changed_paths(
            [
                "docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md",
                "app/GameAuth/OAuth/IssueGameLoginTicketFromOAuth.php",
            ],
            producer_task_exists=False,
        )

        self.assertEqual("NOT_APPLICABLE", result)

    def test_unrelated_architecture_and_runtime_are_not_applicable(self) -> None:
        result = evaluate_changed_paths(
            [
                "docs/architecture/SECURITY_ARCHITECTURE.md",
                "database/migrations/2026_08_07_example.php",
            ],
            producer_task_exists=False,
        )

        self.assertEqual("NOT_APPLICABLE", result)

    def test_native_protocol_documentation_only_passes(self) -> None:
        result = evaluate_changed_paths(
            ["docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md"],
            producer_task_exists=False,
        )

        self.assertEqual("DOCUMENTATION_ONLY", result)

    def test_native_runtime_requires_producer_task_in_change(self) -> None:
        with self.assertRaisesRegex(BoundaryViolation, "missing its active governed task"):
            evaluate_changed_paths(
                [
                    "docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md",
                    "app/GameAuth/Worlds/NativeProtocolWorld.php",
                ],
                producer_task_exists=True,
            )

    def test_native_runtime_requires_producer_task_to_exist(self) -> None:
        with self.assertRaisesRegex(BoundaryViolation, "missing its active governed task"):
            evaluate_changed_paths(
                [
                    "docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md",
                    PRODUCER_TASK,
                    "app/GameAuth/Worlds/NativeProtocolWorld.php",
                ],
                producer_task_exists=False,
            )

    def test_native_runtime_outside_allowlist_fails_closed(self) -> None:
        with self.assertRaisesRegex(BoundaryViolation, "escaped its governed runtime boundary"):
            evaluate_changed_paths(
                [
                    "docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md",
                    PRODUCER_TASK,
                    "app/Identity/Models/Identity.php",
                ],
                producer_task_exists=True,
            )

    def test_valid_native_producer_runtime_passes(self) -> None:
        result = evaluate_changed_paths(
            [
                "docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md",
                PRODUCER_TASK,
                "app/GameAuth/Worlds/NativeProtocolWorld.php",
                "services/game-gateway/src/native.rs",
                "tests/Feature/GameAuth/NativeProtocolContractTest.php",
            ],
            producer_task_exists=True,
        )

        self.assertEqual("GOVERNED_RUNTIME", result)


if __name__ == "__main__":
    unittest.main()
