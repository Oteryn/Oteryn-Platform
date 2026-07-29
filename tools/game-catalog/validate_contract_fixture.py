#!/usr/bin/env python3
"""Validate the shared Oteryn Game Catalog v1 schema and synthetic fixture."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path, PurePosixPath
from typing import Any, Iterable

EXPECTED_CONTRACT = "oteryn.game-catalog"
EXPECTED_SCHEMA_VERSION = "1.0.0"
EXPECTED_SCHEMA_SHA256 = "099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b"
EXPECTED_FIXTURE_SHA256 = "c947e461c1ee8f6fbf511c9890b61135d2585d6c16e2e99a0f72dd5a946c2181"

PUBLIC_ITEM_AVAILABILITY = {
    "obtainable",
    "quest_only",
    "boss_only",
    "event_only",
    "npc_only",
    "starter",
}
PUBLIC_CREATURE_AVAILABILITY = {
    "encounterable",
    "boss_only",
    "event_only",
    "quest_only",
}


class ContractValidationError(ValueError):
    """Raised when the shared contract fixture is invalid."""


def _reject_duplicate_object_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ContractValidationError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


def _load_json(path: Path) -> tuple[bytes, Any]:
    raw = path.read_bytes()
    if raw.startswith(b"\xef\xbb\xbf"):
        raise ContractValidationError(f"{path}: UTF-8 BOM is forbidden")
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise ContractValidationError(f"{path}: invalid UTF-8: {exc}") from exc
    try:
        parsed = json.loads(text, object_pairs_hook=_reject_duplicate_object_pairs)
    except json.JSONDecodeError as exc:
        raise ContractValidationError(f"{path}: invalid JSON: {exc}") from exc
    return raw, parsed


def _sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractValidationError(message)


def _schema_validate(schema: Any, fixture: Any) -> None:
    try:
        from jsonschema import Draft202012Validator
    except ImportError as exc:
        raise ContractValidationError(
            "jsonschema is required; install the pinned CI dependency before validation"
        ) from exc

    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema, format_checker=Draft202012Validator.FORMAT_CHECKER).iter_errors(fixture),
        key=lambda error: tuple(str(part) for part in error.absolute_path),
    )
    if errors:
        rendered: list[str] = []
        for error in errors[:20]:
            path = "$"
            for part in error.absolute_path:
                path += f"[{part}]" if isinstance(part, int) else f".{part}"
            rendered.append(f"{path}: {error.message}")
        suffix = "" if len(errors) <= 20 else f"; {len(errors) - 20} additional errors"
        raise ContractValidationError("JSON Schema validation failed: " + " | ".join(rendered) + suffix)


def _safe_relative_source_path(value: Any, owner: str) -> None:
    if value is None:
        return
    _require(isinstance(value, str), f"{owner}: source_path must be a string or null")
    _require(value != "", f"{owner}: source_path must not be empty")
    path = PurePosixPath(value)
    _require(not path.is_absolute(), f"{owner}: source_path must be relative")
    _require(".." not in path.parts, f"{owner}: source_path must not contain '..'")


def _release_order(releases: dict[str, dict[str, Any]], key: str | None, owner: str) -> int | None:
    if key is None:
        return None
    _require(key in releases, f"{owner}: unknown release reference {key!r}")
    return int(releases[key]["release_order"])


def _range_contains(
    releases: dict[str, dict[str, Any]],
    introduced: str | None,
    removed: str | None,
    target: str,
    owner: str,
) -> bool:
    target_order = _release_order(releases, target, owner)
    introduced_order = _release_order(releases, introduced, owner)
    removed_order = _release_order(releases, removed, owner)
    _require(target_order is not None, f"{owner}: target release is required")
    if introduced_order is not None and target_order < introduced_order:
        return False
    if removed_order is not None and target_order >= removed_order:
        return False
    return True


def _validate_range(
    releases: dict[str, dict[str, Any]],
    introduced: str | None,
    removed: str | None,
    owner: str,
) -> None:
    introduced_order = _release_order(releases, introduced, owner)
    removed_order = _release_order(releases, removed, owner)
    if introduced_order is not None and removed_order is not None:
        _require(
            introduced_order < removed_order,
            f"{owner}: removed_in must be an exclusive upper bound later than introduced_in",
        )


def _visible_entities(
    fixture: dict[str, Any],
    releases: dict[str, dict[str, Any]],
    target: str,
) -> set[str]:
    visible: set[str] = set()
    for entity in fixture["entities"]:
        availability = (
            PUBLIC_ITEM_AVAILABILITY
            if entity["type"] == "item"
            else PUBLIC_CREATURE_AVAILABILITY
        )
        if (
            entity["runtime_present"]
            and entity["enabled"]
            and entity["completeness"] == "complete"
            and entity["availability"] in availability
            and _range_contains(
                releases,
                entity["introduced_in"],
                entity["removed_in"],
                target,
                entity["canonical_key"],
            )
        ):
            visible.add(entity["canonical_key"])
    return visible


def _visible_relations(
    fixture: dict[str, Any],
    releases: dict[str, dict[str, Any]],
    target: str,
    visible_entities: set[str],
) -> set[str]:
    visible: set[str] = set()
    for relation in fixture["relations"]:
        if (
            relation["enabled"]
            and relation["completeness"] == "complete"
            and relation["source"] in visible_entities
            and relation["target"] in visible_entities
            and _range_contains(
                releases,
                relation["introduced_in"],
                relation["removed_in"],
                target,
                relation["canonical_key"],
            )
        ):
            visible.add(relation["canonical_key"])
    return visible


def _validate_fixture_semantics(fixture: Any) -> dict[str, list[str]]:
    _require(isinstance(fixture, dict), "fixture root must be an object")
    _require(fixture.get("contract") == EXPECTED_CONTRACT, "unexpected contract ID")
    _require(
        fixture.get("schema_version") == EXPECTED_SCHEMA_VERSION,
        "unexpected schema version",
    )

    releases_list = fixture.get("releases")
    entities = fixture.get("entities")
    relations = fixture.get("relations")
    snapshot = fixture.get("snapshot")
    _require(isinstance(releases_list, list), "releases must be an array")
    _require(isinstance(entities, list), "entities must be an array")
    _require(isinstance(relations, list), "relations must be an array")
    _require(isinstance(snapshot, dict), "snapshot must be an object")

    release_keys = [release["key"] for release in releases_list]
    release_orders = [release["release_order"] for release in releases_list]
    _require(len(release_keys) == len(set(release_keys)), "duplicate release key")
    _require(len(release_orders) == len(set(release_orders)), "duplicate release_order")
    expected_release_sort = sorted(
        releases_list, key=lambda release: (release["release_order"], release["key"])
    )
    _require(releases_list == expected_release_sort, "releases are not deterministically sorted")
    releases = {release["key"]: release for release in releases_list}

    for field in (
        "runtime_release",
        "content_target_release",
        "verified_content_through_release",
        "contains_content_through_release",
    ):
        _release_order(releases, snapshot.get(field), f"snapshot.{field}")

    _require(snapshot.get("entity_count") == len(entities), "declared entity_count mismatch")
    _require(snapshot.get("relation_count") == len(relations), "declared relation_count mismatch")

    entity_keys = [entity["canonical_key"] for entity in entities]
    _require(len(entity_keys) == len(set(entity_keys)), "duplicate entity canonical key")
    _require(
        entities == sorted(entities, key=lambda entity: (entity["type"], entity["canonical_key"])),
        "entities are not deterministically sorted",
    )

    for entity in entities:
        owner = entity["canonical_key"]
        _validate_range(releases, entity["introduced_in"], entity["removed_in"], owner)
        _safe_relative_source_path(entity.get("source_path"), owner)
        identifiers = entity["identifiers"]
        _require(
            identifiers == sorted(
                identifiers, key=lambda identifier: (identifier["namespace"], identifier["value"])
            ),
            f"{owner}: identifiers are not deterministically sorted",
        )

    relation_keys = [relation["canonical_key"] for relation in relations]
    _require(len(relation_keys) == len(set(relation_keys)), "duplicate relation canonical key")
    _require(
        relations == sorted(
            relations, key=lambda relation: (relation["type"], relation["canonical_key"])
        ),
        "relations are not deterministically sorted",
    )

    entity_key_set = set(entity_keys)
    for relation in relations:
        owner = relation["canonical_key"]
        _validate_range(releases, relation["introduced_in"], relation["removed_in"], owner)
        _safe_relative_source_path(relation.get("source_path"), owner)
        _require(relation["source"] in entity_key_set, f"{owner}: dangling source endpoint")
        _require(relation["target"] in entity_key_set, f"{owner}: dangling target endpoint")
        data = relation["data"]
        _require(
            0 <= data["chance_numerator"] <= data["chance_denominator"],
            f"{owner}: invalid loot probability",
        )
        _require(
            data["minimum_count"] <= data["maximum_count"],
            f"{owner}: maximum_count is below minimum_count",
        )

    _require(len(releases_list) >= 2, "fixture must contain at least two releases")
    _require(
        any(entity["type"] == "item" and entity["introduced_in"] == "15.21" for entity in entities),
        "fixture must contain one future item",
    )
    _require(
        any(entity["type"] == "creature" and entity["completeness"] == "complete" for entity in entities),
        "fixture must contain one complete creature",
    )
    _require(
        any(entity["type"] == "creature" and entity["completeness"] == "partial" for entity in entities),
        "fixture must contain one partial creature",
    )

    visible_1520 = _visible_entities(fixture, releases, "15.20")
    relations_1520 = _visible_relations(fixture, releases, "15.20", visible_1520)
    visible_1521 = _visible_entities(fixture, releases, "15.21")
    relations_1521 = _visible_relations(fixture, releases, "15.21", visible_1521)

    _require(
        visible_1520 == {"creature:fixture-rat", "item:fixture-sword"},
        f"unexpected 15.20 entity visibility: {sorted(visible_1520)}",
    )
    _require(
        relations_1520 == {"loot:fixture-rat:fixture-sword"},
        f"unexpected 15.20 relation visibility: {sorted(relations_1520)}",
    )
    _require(
        visible_1521
        == {
            "creature:fixture-rat",
            "item:fixture-sword",
            "item:future-fixture-shield",
        },
        f"unexpected 15.21 entity visibility: {sorted(visible_1521)}",
    )
    _require(
        relations_1521
        == {
            "loot:fixture-rat:fixture-sword",
            "loot:fixture-rat:fixture-sword:2",
        },
        f"unexpected 15.21 relation visibility: {sorted(relations_1521)}",
    )

    return {
        "15.20_entities": sorted(visible_1520),
        "15.20_relations": sorted(relations_1520),
        "15.21_entities": sorted(visible_1521),
        "15.21_relations": sorted(relations_1521),
    }


def _parse_args(argv: Iterable[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--schema", type=Path, required=True)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument(
        "--expected-schema-sha256",
        default=EXPECTED_SCHEMA_SHA256,
    )
    parser.add_argument(
        "--expected-fixture-sha256",
        default=EXPECTED_FIXTURE_SHA256,
    )
    return parser.parse_args(list(argv))


def main(argv: Iterable[str] = sys.argv[1:]) -> int:
    args = _parse_args(argv)
    try:
        schema_raw, schema = _load_json(args.schema)
        fixture_raw, fixture = _load_json(args.fixture)

        schema_sha = _sha256(schema_raw)
        fixture_sha = _sha256(fixture_raw)
        _require(
            schema_sha == args.expected_schema_sha256,
            f"schema SHA-256 mismatch: expected {args.expected_schema_sha256}, got {schema_sha}",
        )
        _require(
            fixture_sha == args.expected_fixture_sha256,
            f"fixture SHA-256 mismatch: expected {args.expected_fixture_sha256}, got {fixture_sha}",
        )

        _schema_validate(schema, fixture)
        visibility = _validate_fixture_semantics(fixture)
    except (ContractValidationError, OSError) as exc:
        print(f"game-catalog contract validation failed: {exc}", file=sys.stderr)
        return 1

    print(
        json.dumps(
            {
                "contract": EXPECTED_CONTRACT,
                "schema_version": EXPECTED_SCHEMA_VERSION,
                "schema_sha256": schema_sha,
                "fixture_sha256": fixture_sha,
                "visibility": visibility,
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
