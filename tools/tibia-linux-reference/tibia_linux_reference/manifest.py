from __future__ import annotations

import json
from collections.abc import Mapping, Sequence
from pathlib import Path

from . import HarnessError


REQUIRED_TOP_LEVEL = {
    "schema_version",
    "session_id",
    "mode",
    "started_at",
    "finished_at",
    "duration_monotonic_ms",
    "environment",
    "client_identity",
    "credential_handling",
    "process_lifecycle",
    "window_lifecycle",
    "network_denial",
    "filesystem_inventory",
    "leak_scan",
    "cleanup",
    "safety",
    "findings",
}
SAFETY_FALSE_FIELDS = {
    "official_login_attempted",
    "official_service_contacted",
    "client_or_battleye_modified",
    "ptrace_debugger_hook_injection_used",
    "traffic_decrypted_replayed_altered_injected",
}
_SCHEMA_PATH = Path(__file__).resolve().parents[1] / "schema" / "session-manifest.schema.json"
_SCHEMA_METADATA_KEYS = {"$schema", "$id", "title", "description"}
_SCHEMA_VALIDATION_KEYS = {
    "type",
    "additionalProperties",
    "required",
    "properties",
    "const",
    "enum",
    "minLength",
    "minimum",
    "items",
}


def _json_equal(left: object, right: object) -> bool:
    try:
        return json.dumps(left, sort_keys=True, separators=(",", ":")) == json.dumps(
            right, sort_keys=True, separators=(",", ":")
        )
    except (TypeError, ValueError):
        return False


def _schema_type_matches(value: object, expected: str) -> bool:
    if expected == "object":
        return isinstance(value, Mapping)
    if expected == "array":
        return isinstance(value, list)
    if expected == "string":
        return isinstance(value, str)
    if expected == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    raise HarnessError(f"manifest schema uses unsupported type {expected!r}")


def _validate_schema_node(value: object, schema: Mapping[str, object], path: str) -> None:
    unsupported = set(schema) - _SCHEMA_METADATA_KEYS - _SCHEMA_VALIDATION_KEYS
    if unsupported:
        raise HarnessError(
            "manifest schema uses unsupported validation keyword(s): "
            + ", ".join(sorted(unsupported))
        )

    expected_type = schema.get("type")
    if expected_type is not None:
        if not isinstance(expected_type, str):
            raise HarnessError("manifest schema type declaration is invalid")
        if not _schema_type_matches(value, expected_type):
            raise HarnessError(f"manifest value at {path} does not match schema type {expected_type}")

    if "const" in schema and not _json_equal(value, schema["const"]):
        raise HarnessError(f"manifest value at {path} does not match schema const")

    if "enum" in schema:
        choices = schema["enum"]
        if not isinstance(choices, list):
            raise HarnessError("manifest schema enum declaration is invalid")
        if not any(_json_equal(value, choice) for choice in choices):
            raise HarnessError(f"manifest value at {path} is outside schema enum")

    if "minLength" in schema:
        minimum_length = schema["minLength"]
        if not isinstance(minimum_length, int) or isinstance(minimum_length, bool):
            raise HarnessError("manifest schema minLength declaration is invalid")
        if not isinstance(value, str) or len(value) < minimum_length:
            raise HarnessError(f"manifest value at {path} is shorter than schema minLength")

    if "minimum" in schema:
        minimum = schema["minimum"]
        if not isinstance(minimum, (int, float)) or isinstance(minimum, bool):
            raise HarnessError("manifest schema minimum declaration is invalid")
        if not isinstance(value, (int, float)) or isinstance(value, bool) or value < minimum:
            raise HarnessError(f"manifest value at {path} is below schema minimum")

    if isinstance(value, Mapping):
        properties = schema.get("properties", {})
        if not isinstance(properties, Mapping):
            raise HarnessError("manifest schema properties declaration is invalid")
        required = schema.get("required", [])
        if not isinstance(required, list) or not all(isinstance(item, str) for item in required):
            raise HarnessError("manifest schema required declaration is invalid")
        missing = [item for item in required if item not in value]
        if missing:
            raise HarnessError(f"manifest value at {path} misses required schema keys: {sorted(missing)}")
        if schema.get("additionalProperties") is False:
            extra = set(value) - set(properties)
            if extra:
                raise HarnessError(f"manifest value at {path} contains extra schema keys: {sorted(extra)}")
        additional = schema.get("additionalProperties")
        if additional not in (None, True, False):
            raise HarnessError("manifest schema additionalProperties declaration is unsupported")
        for key, child_schema in properties.items():
            if key not in value:
                continue
            if not isinstance(key, str) or not isinstance(child_schema, Mapping):
                raise HarnessError("manifest schema property declaration is invalid")
            _validate_schema_node(value[key], child_schema, f"{path}.{key}")

    if isinstance(value, list) and "items" in schema:
        item_schema = schema["items"]
        if not isinstance(item_schema, Mapping):
            raise HarnessError("manifest schema items declaration is invalid")
        for index, item in enumerate(value):
            _validate_schema_node(item, item_schema, f"{path}[{index}]")


def _validate_committed_schema(document: Mapping[str, object]) -> None:
    try:
        schema_document = json.loads(_SCHEMA_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise HarnessError("committed manifest schema could not be loaded") from error
    if not isinstance(schema_document, Mapping):
        raise HarnessError("committed manifest schema must be a JSON object")
    if schema_document.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        raise HarnessError("committed manifest schema draft is not the approved 2020-12 contract")
    _validate_schema_node(document, schema_document, "$")


def validate_manifest(document: Mapping[str, object]) -> None:
    _validate_committed_schema(document)

    missing = REQUIRED_TOP_LEVEL - set(document)
    extra = set(document) - REQUIRED_TOP_LEVEL
    if missing or extra:
        raise HarnessError(f"manifest keys differ from schema; missing={sorted(missing)}, extra={sorted(extra)}")
    if document["schema_version"] != 1 or document["mode"] not in {"synthetic-dry-run", "official-component"}:
        raise HarnessError("manifest schema version or mode is invalid")

    leak_scan = document["leak_scan"]
    cleanup = document["cleanup"]
    network = document["network_denial"]
    safety = document["safety"]
    credentials = document["credential_handling"]

    if not isinstance(leak_scan, Mapping) or leak_scan.get("result") != "PASS":
        raise HarnessError("manifest leak scan must pass")
    if not isinstance(cleanup, Mapping) or cleanup.get("result") != "PASS":
        raise HarnessError("manifest cleanup must pass")
    if not isinstance(network, Mapping) or network.get("proven") is not True:
        raise HarnessError("manifest must prove network denial")
    if network.get("raw_capture_created") is not False or network.get("official_endpoint_contacted") is not False:
        raise HarnessError("manifest network safety fields must remain false")
    if not isinstance(safety, Mapping) or set(safety) != SAFETY_FALSE_FIELDS:
        raise HarnessError("manifest safety fields differ from the fail-closed contract")
    if any(safety.get(field) is not False for field in SAFETY_FALSE_FIELDS):
        raise HarnessError("manifest safety fields must remain false")
    if not isinstance(credentials, Mapping):
        raise HarnessError("manifest credential handling must be an object")
    if credentials.get("arguments_contained_values") is not False:
        raise HarnessError("manifest must prove credential values were absent from arguments")
    if credentials.get("environment_contained_values") is not False:
        raise HarnessError("manifest must prove credential values were absent from ordinary environment variables")
    if credentials.get("retained") is not False:
        raise HarnessError("manifest must prove credential values were not retained")

    findings = document["findings"]
    if not isinstance(findings, Mapping) or set(findings) != {"PROVEN", "DERIVED", "UNKNOWN", "CONFLICT"}:
        raise HarnessError("manifest findings must use the four evidence classifications")
    for values in findings.values():
        if not isinstance(values, Sequence) or isinstance(values, (str, bytes)):
            raise HarnessError("manifest finding classes must be arrays")
