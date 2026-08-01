from __future__ import annotations

from typing import Mapping, Sequence

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


def validate_manifest(document: Mapping[str, object]) -> None:
    missing = REQUIRED_TOP_LEVEL - set(document)
    extra = set(document) - REQUIRED_TOP_LEVEL
    if missing or extra:
        raise HarnessError(f"manifest keys differ from schema; missing={sorted(missing)}, extra={sorted(extra)}")
    if document["schema_version"] != 1 or document["mode"] not in {"synthetic-dry-run", "official-component"}:
        raise HarnessError("manifest schema version or mode is invalid")
    leak_scan = document["leak_scan"]
    cleanup = document["cleanup"]
    network = document["network_denial"]
    if not isinstance(leak_scan, Mapping) or leak_scan.get("result") != "PASS":
        raise HarnessError("manifest leak scan must pass")
    if not isinstance(cleanup, Mapping) or cleanup.get("result") != "PASS":
        raise HarnessError("manifest cleanup must pass")
    if document["mode"] == "synthetic-dry-run":
        if not isinstance(network, Mapping) or network.get("proven") is not True:
            raise HarnessError("synthetic dry-run manifest must prove network denial")
    findings = document["findings"]
    if not isinstance(findings, Mapping) or set(findings) != {"PROVEN", "DERIVED", "UNKNOWN", "CONFLICT"}:
        raise HarnessError("manifest findings must use the four evidence classifications")
    for values in findings.values():
        if not isinstance(values, Sequence) or isinstance(values, (str, bytes)):
            raise HarnessError("manifest finding classes must be arrays")
