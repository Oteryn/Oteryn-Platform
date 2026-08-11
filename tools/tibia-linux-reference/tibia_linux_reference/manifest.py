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
SAFETY_FALSE_FIELDS = {
    "official_login_attempted",
    "official_service_contacted",
    "client_or_battleye_modified",
    "ptrace_debugger_hook_injection_used",
    "traffic_decrypted_replayed_altered_injected",
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
