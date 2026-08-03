#!/usr/bin/env python3
"""Bounded Cloudflare WAF/Bot repair for the canonical Oteryn public hosts."""
from __future__ import annotations

import hashlib
import json
import os
import re
import sys
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

BASE = os.getenv("CLOUDFLARE_API_BASE_URL", "https://api.cloudflare.com/client/v4").rstrip("/")
TOKEN = os.getenv("CLOUDFLARE_API_TOKEN", "")
ZONE = os.getenv("CLOUDFLARE_ZONE_ID", "")
OUT = Path(os.getenv("CLOUDFLARE_PUBLIC_EDGE_REPAIR_OUT", "cloudflare-public-edge-repair"))
CONFIRMATION = os.getenv("CLOUDFLARE_EDGE_REPAIR_CONFIRMATION", "")
WWW = "oteryn.molehill.cloud"
GATEWAY = "gateway.molehill.cloud"
RULE_REF = "oteryn-public-edge-canonical-skip-v1"
RULE_DESCRIPTION_PREFIX = "Oteryn canonical public edge exception v1"
RULE_EXPRESSION = f'http.host in {{"{WWW}" "{GATEWAY}"}}'
RULE_ACTION_PARAMETERS = {"products": ["bic", "securityLevel"], "ruleset": "current"}
APPLY_CONFIRMATION = "APPLY-OTERYN-PUBLIC-EDGE-REPAIR"
ROLLBACK_CONFIRMATION = "ROLLBACK-OTERYN-PUBLIC-EDGE-REPAIR"
COUNTRY_FIELD = re.compile(r"\b(?:ip\.geoip\.country|ip\.src\.country)\b", re.IGNORECASE)
COUNTRY_NEGATION = re.compile(r"\b(?:ne|not\s+in)\b", re.IGNORECASE)
EXPECTED_CANDIDATE_ID = os.getenv(
    "CLOUDFLARE_EXPECTED_COUNTRY_RULE_ID", "e0f91939eb494d4490d975498a9a9724"
)
EXPECTED_CANDIDATE_HASH = os.getenv(
    "CLOUDFLARE_EXPECTED_COUNTRY_RULE_SHA256",
    "3f5a9e27f91d9cfe4fb6f77ede8c1e91997ef32a91a443cd1e6b61211ff13c45",
)


class RepairError(RuntimeError):
    pass


def sha256(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def api(method: str, path: str, body: dict[str, Any] | None = None) -> dict[str, Any]:
    if not TOKEN:
        raise RepairError("CLOUDFLARE_API_TOKEN is missing")
    data = None
    headers = {
        "Authorization": f"Bearer {TOKEN}",
        "Accept": "application/json",
    }
    if body is not None:
        data = json.dumps(body, separators=(",", ":")).encode("utf-8")
        headers["Content-Type"] = "application/json"
    request = urllib.request.Request(BASE + path, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            status = response.status
            raw = response.read(2_000_000)
    except urllib.error.HTTPError as exc:
        status = exc.code
        raw = exc.read(2_000_000)
    except Exception as exc:  # pragma: no cover - network boundary
        raise RepairError(f"Cloudflare transport error: {type(exc).__name__}") from exc

    if not raw and 200 <= status < 300:
        return {"success": True, "result": None, "status": status}
    try:
        payload = json.loads(raw)
    except Exception as exc:
        raise RepairError(f"Cloudflare returned non-JSON HTTP {status}") from exc
    if not (200 <= status < 300 and payload.get("success") is True):
        codes = [str(item.get("code")) for item in payload.get("errors", []) if isinstance(item, dict)]
        code_text = ",".join(codes[:5]) or "unknown"
        raise RepairError(f"Cloudflare {method} {path} failed: HTTP {status}; codes={code_text}")
    return {"success": True, "result": payload.get("result"), "status": status}


def verify_token() -> None:
    path = "/user/tokens/verify" if not TOKEN.startswith("cfat_") else "/tokens/verify"
    result = api("GET", path).get("result")
    if not isinstance(result, dict) or result.get("status") != "active":
        raise RepairError("Cloudflare token is not proven active")


def get_ruleset() -> dict[str, Any]:
    result = api("GET", f"/zones/{ZONE}/rulesets/phases/http_request_firewall_custom/entrypoint").get("result")
    if not isinstance(result, dict) or not isinstance(result.get("rules"), list):
        raise RepairError("custom WAF entrypoint ruleset is unavailable")
    if result.get("phase") != "http_request_firewall_custom":
        raise RepairError("unexpected WAF ruleset phase")
    return result


def get_bot() -> dict[str, Any]:
    result = api("GET", f"/zones/{ZONE}/bot_management").get("result")
    if not isinstance(result, dict) or not isinstance(result.get("fight_mode"), bool):
        raise RepairError("Bot Management configuration is unavailable")
    return result


def is_broad_country_block(rule: dict[str, Any]) -> bool:
    expression = str(rule.get("expression", ""))
    return (
        rule.get("enabled", True) is True
        and rule.get("action") == "block"
        and "http.host" not in expression.lower()
        and COUNTRY_FIELD.search(expression) is not None
        and COUNTRY_NEGATION.search(expression) is not None
    )


def repair_rules(ruleset: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        item
        for item in ruleset.get("rules", [])
        if isinstance(item, dict) and item.get("ref") == RULE_REF
    ]


def candidate_rules(ruleset: dict[str, Any]) -> list[dict[str, Any]]:
    return [item for item in ruleset.get("rules", []) if isinstance(item, dict) and is_broad_country_block(item)]


def baseline_from_description(description: str) -> bool | None:
    match = re.fullmatch(re.escape(RULE_DESCRIPTION_PREFIX) + r" \[bot-baseline:(on|off)\]", description)
    if not match:
        return None
    return match.group(1) == "on"


def exact_repair_rule(rule: dict[str, Any]) -> tuple[bool, bool | None]:
    baseline = baseline_from_description(str(rule.get("description", "")))
    exact = (
        rule.get("action") == "skip"
        and rule.get("enabled", True) is True
        and rule.get("expression") == RULE_EXPRESSION
        and rule.get("action_parameters") == RULE_ACTION_PARAMETERS
        and rule.get("logging", {"enabled": True}) == {"enabled": True}
        and baseline is not None
    )
    return exact, baseline


def rule_index(ruleset: dict[str, Any], rule_id: str) -> int:
    for index, item in enumerate(ruleset.get("rules", [])):
        if isinstance(item, dict) and item.get("id") == rule_id:
            return index
    return -1


def inspect_state() -> dict[str, Any]:
    ruleset = get_ruleset()
    candidates = candidate_rules(ruleset)
    repairs = repair_rules(ruleset)
    bot = get_bot()
    candidate_hashes = [sha256(str(item.get("expression", ""))) for item in candidates]
    repair_state = "absent"
    baseline: bool | None = None
    before_candidate = False
    repair_first = False
    repair_exact = False
    repair_index = -1
    repair_hash: str | None = None
    if len(repairs) > 1:
        repair_state = "ambiguous"
    elif len(repairs) == 1:
        repair_exact, baseline = exact_repair_rule(repairs[0])
        repair_hash = sha256(str(repairs[0].get("expression", "")))
        repair_index = rule_index(ruleset, str(repairs[0].get("id", "")))
        repair_first = repair_index == 0
        if len(candidates) == 1:
            candidate_index = rule_index(ruleset, str(candidates[0].get("id", "")))
            before_candidate = 0 <= repair_index < candidate_index
        if not repair_exact:
            repair_state = "drift"
        elif not before_candidate:
            repair_state = "wrong_order"
        elif not repair_first:
            repair_state = "shadowed_by_earlier_rules"
        else:
            repair_state = "current"
    desired = (
        len(candidates) == 1
        and repair_state == "current"
        and repair_first
        and before_candidate
        and bot.get("fight_mode") is False
    )
    return {
        "ruleset": ruleset,
        "bot": bot,
        "candidates": candidates,
        "repairs": repairs,
        "candidate_expression_hashes": candidate_hashes,
        "repair_expression_hash": repair_hash,
        "repair_state": repair_state,
        "repair_exact": repair_exact,
        "repair_index": repair_index,
        "repair_first": repair_first,
        "repair_before_candidate": before_candidate,
        "bot_baseline": baseline,
        "desired_state": desired,
    }


def require_unambiguous(state: dict[str, Any], *, allow_absent: bool) -> None:
    if len(state["candidates"]) != 1:
        raise RepairError(f"expected exactly one broad country block candidate; found {len(state['candidates'])}")
    candidate = state["candidates"][0]
    if candidate.get("id") != EXPECTED_CANDIDATE_ID:
        raise RepairError("broad country block candidate ID does not match the audited rule")
    if state.get("candidate_expression_hashes") != [EXPECTED_CANDIDATE_HASH]:
        raise RepairError("broad country block expression hash does not match the audited rule")
    if len(state["repairs"]) > 1:
        raise RepairError("multiple Oteryn repair rules exist")
    if not allow_absent and len(state["repairs"]) != 1:
        raise RepairError("Oteryn repair rule is absent")
    if len(state["repairs"]) == 1:
        if not state.get("repair_exact"):
            raise RepairError("Oteryn repair rule is not exact/current")
        if not state.get("repair_before_candidate"):
            raise RepairError("Oteryn repair rule is not ordered before the broad block rule")


def created_rule_from_response(result: Any, bot_baseline: bool) -> dict[str, Any]:
    """Normalize Cloudflare's ruleset-shaped create response and legacy direct-rule fixtures."""
    if not isinstance(result, dict):
        raise RepairError("Cloudflare did not return a ruleset or created rule")

    if result.get("ref") == RULE_REF:
        matches = [result]
    else:
        rules = result.get("rules")
        if not isinstance(rules, list):
            raise RepairError("Cloudflare create response did not contain a rules list")
        matches = [item for item in rules if isinstance(item, dict) and item.get("ref") == RULE_REF]

    if len(matches) != 1:
        raise RepairError(f"Cloudflare create response contained {len(matches)} matching repair rules")
    rule = matches[0]
    exact, baseline = exact_repair_rule(rule)
    if not exact or baseline != bot_baseline:
        raise RepairError("Cloudflare returned a non-exact created repair rule")
    if not rule.get("id"):
        raise RepairError("Cloudflare created repair rule has no identifier")
    return rule


def create_repair_rule(state: dict[str, Any], bot_baseline: bool) -> str:
    ruleset_id = str(state["ruleset"].get("id", ""))
    if not ruleset_id:
        raise RepairError("ruleset identifier is missing")
    description = f"{RULE_DESCRIPTION_PREFIX} [bot-baseline:{'on' if bot_baseline else 'off'}]"
    body = {
        "action": "skip",
        "action_parameters": RULE_ACTION_PARAMETERS,
        "description": description,
        "enabled": True,
        "expression": RULE_EXPRESSION,
        "logging": {"enabled": True},
        "position": {"before": ""},
        "ref": RULE_REF,
    }
    result = api("POST", f"/zones/{ZONE}/rulesets/{ruleset_id}/rules", body).get("result")
    return str(created_rule_from_response(result, bot_baseline)["id"])


def move_repair_rule(state: dict[str, Any], position: dict[str, Any]) -> None:
    require_unambiguous(state, allow_absent=False)
    ruleset_id = str(state["ruleset"].get("id", ""))
    rule_id = str(state["repairs"][0].get("id", ""))
    if not ruleset_id or not rule_id:
        raise RepairError("repair rule identifiers are missing")
    api("PATCH", f"/zones/{ZONE}/rulesets/{ruleset_id}/rules/{rule_id}", {"position": position})


def delete_repair_rule(state: dict[str, Any]) -> None:
    require_unambiguous(state, allow_absent=False)
    rule = state["repairs"][0]
    exact, _ = exact_repair_rule(rule)
    if not exact:
        raise RepairError("refusing to delete a drifted repair rule")
    ruleset_id = str(state["ruleset"].get("id", ""))
    rule_id = str(rule.get("id", ""))
    if not ruleset_id or not rule_id:
        raise RepairError("repair rule identifiers are missing")
    api("DELETE", f"/zones/{ZONE}/rulesets/{ruleset_id}/rules/{rule_id}")


def set_fight_mode(value: bool) -> None:
    api("PUT", f"/zones/{ZONE}/bot_management", {"fight_mode": value})


def rollback_partial(
    created_rule: bool,
    moved_rule: bool,
    original_next_id: str | None,
    bot_changed: bool,
    baseline: bool,
) -> list[str]:
    actions: list[str] = []
    errors: list[str] = []
    if bot_changed:
        try:
            set_fight_mode(baseline)
            actions.append("bot_restored")
        except Exception as exc:  # pragma: no cover - emergency path
            errors.append(f"bot rollback failed: {type(exc).__name__}")
    if created_rule:
        try:
            current = inspect_state()
            if current["repairs"]:
                delete_repair_rule(current)
                actions.append("waf_rule_deleted")
            else:
                actions.append("waf_rule_already_absent")
        except Exception as exc:  # pragma: no cover - emergency path
            errors.append(f"WAF rollback failed: {type(exc).__name__}")
    elif moved_rule:
        try:
            current = inspect_state()
            position = {"before": original_next_id} if original_next_id else {"after": ""}
            move_repair_rule(current, position)
            actions.append("waf_rule_position_restored")
        except Exception as exc:  # pragma: no cover - emergency path
            errors.append(f"WAF order rollback failed: {type(exc).__name__}")
    if errors:
        raise RepairError("; ".join(errors))
    return actions


def apply() -> tuple[dict[str, Any], list[str]]:
    if CONFIRMATION != APPLY_CONFIRMATION:
        raise RepairError("apply confirmation is invalid")
    state = inspect_state()
    require_unambiguous(state, allow_absent=True)
    if state["desired_state"]:
        return state, []

    baseline = bool(state["bot"].get("fight_mode"))
    repair_absent_before = not state["repairs"]
    created_rule = False
    moved_rule = False
    original_next_id: str | None = None
    bot_changed = False
    mutations: list[str] = []
    try:
        if repair_absent_before:
            try:
                create_repair_rule(state, baseline)
                created_rule = True
                mutations.append("waf_skip_rule_created")
            except Exception:
                observed = inspect_state()
                if (
                    len(observed["repairs"]) == 1
                    and observed.get("repair_exact")
                    and observed.get("repair_first")
                    and observed.get("repair_before_candidate")
                ):
                    created_rule = True
                raise
        elif not state.get("repair_first"):
            repair_index = int(state.get("repair_index", -1))
            rules = state["ruleset"].get("rules", [])
            if repair_index < 0 or not isinstance(rules, list):
                raise RepairError("repair rule position is unavailable")
            if repair_index + 1 < len(rules) and isinstance(rules[repair_index + 1], dict):
                original_next_id = str(rules[repair_index + 1].get("id", "")) or None
            move_repair_rule(state, {"before": ""})
            moved_rule = True
            mutations.append("waf_skip_rule_moved_first")

        state = inspect_state()
        require_unambiguous(state, allow_absent=False)
        if not state.get("repair_first"):
            raise RepairError("repair rule is not the first rule in the custom ruleset")
        if state["bot"].get("fight_mode") is True:
            set_fight_mode(False)
            bot_changed = True
            mutations.append("bot_fight_mode_disabled")
        final = inspect_state()
        if not final["desired_state"]:
            raise RepairError("post-write verification did not reach the exact desired state")
        return final, mutations
    except Exception as exc:
        rollback_actions = rollback_partial(
            created_rule,
            moved_rule,
            original_next_id,
            bot_changed,
            baseline,
        )
        rollback_text = ",".join(rollback_actions) or "none"
        raise RepairError(f"apply failed and partial changes were rolled back ({rollback_text}): {exc}") from exc


def rollback() -> tuple[dict[str, Any], list[str]]:
    if CONFIRMATION != ROLLBACK_CONFIRMATION:
        raise RepairError("rollback confirmation is invalid")
    state = inspect_state()
    require_unambiguous(state, allow_absent=False)
    baseline = state["bot_baseline"]
    if baseline is None:
        raise RepairError("repair rule does not contain a recognized Bot Fight Mode baseline")
    mutations: list[str] = []
    current_bot = bool(state["bot"].get("fight_mode"))
    if current_bot != baseline:
        set_fight_mode(baseline)
        mutations.append("bot_fight_mode_restored")
    delete_repair_rule(inspect_state())
    mutations.append("waf_skip_rule_deleted")
    final = inspect_state()
    if final["repairs"]:
        raise RepairError("rollback verification found a remaining repair rule")
    if bool(final["bot"].get("fight_mode")) != baseline:
        raise RepairError("rollback verification found the wrong Bot Fight Mode state")
    return final, mutations


def sanitized(mode: str, state: dict[str, Any], mutations: list[str], status: str) -> dict[str, Any]:
    return {
        "schema_version": 2,
        "observed_at_utc": datetime.now(timezone.utc).isoformat(),
        "classification": "CLOUDFLARE_PUBLIC_EDGE_REPAIR",
        "operation_status": status,
        "mode": mode,
        "canonical_hosts": [WWW, GATEWAY],
        "candidate_count": len(state.get("candidates", [])),
        "candidate_expression_hashes": state.get("candidate_expression_hashes", []),
        "repair_rule_count": len(state.get("repairs", [])),
        "repair_state": state.get("repair_state", "unknown"),
        "repair_exact": state.get("repair_exact", False),
        "repair_index": state.get("repair_index", -1),
        "repair_first": state.get("repair_first", False),
        "repair_before_candidate": state.get("repair_before_candidate", False),
        "repair_expression_hash": state.get("repair_expression_hash"),
        "bot_fight_mode": state.get("bot", {}).get("fight_mode"),
        "bot_baseline": state.get("bot_baseline"),
        "desired_state": state.get("desired_state", False),
        "mutations": mutations,
        "mutation": "none" if not mutations else ",".join(mutations),
        "secrets_emitted": False,
        "expressions_emitted": False,
    }


def emit(evidence: dict[str, Any]) -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "evidence.json").write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    lines = [
        "# Cloudflare Oteryn public-edge repair",
        "",
        f"Observed at: `{evidence['observed_at_utc']}`",
        f"Mode: `{evidence['mode']}`",
        f"Status: `{evidence['operation_status']}`",
        f"Repair state: `{evidence['repair_state']}`",
        f"Repair first: `{evidence['repair_first']}`",
        f"Bot Fight Mode: `{evidence['bot_fight_mode']}`",
        f"Desired state: `{evidence['desired_state']}`",
        f"Mutation: `{evidence['mutation']}`",
        "",
        "No token, rule expression, country literal or raw API response is emitted.",
    ]
    (OUT / "summary.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    for key in (
        "operation_status",
        "mode",
        "candidate_count",
        "repair_rule_count",
        "repair_state",
        "repair_first",
        "repair_before_candidate",
        "bot_fight_mode",
        "desired_state",
        "mutation",
    ):
        value = evidence.get(key)
        if isinstance(value, bool):
            value = "true" if value else "false"
        print(f"{key}={value}")


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] not in {"audit", "apply", "rollback"}:
        raise SystemExit("usage: cloudflare-oteryn-public-edge-repair.py audit|apply|rollback")
    if not re.fullmatch(r"[0-9a-fA-F]{32}", ZONE):
        raise SystemExit("ERROR: invalid CLOUDFLARE_ZONE_ID")
    if not re.fullmatch(r"[0-9a-fA-F]{32}", EXPECTED_CANDIDATE_ID):
        raise SystemExit("ERROR: invalid expected country rule ID")
    if not re.fullmatch(r"[0-9a-fA-F]{64}", EXPECTED_CANDIDATE_HASH):
        raise SystemExit("ERROR: invalid expected country rule SHA-256")
    mode = sys.argv[1]
    try:
        verify_token()
        if mode == "audit":
            state = inspect_state()
            require_unambiguous(state, allow_absent=True)
            mutations: list[str] = []
        elif mode == "apply":
            state, mutations = apply()
        else:
            state, mutations = rollback()
        evidence = sanitized(mode, state, mutations, "success")
        emit(evidence)
    except RepairError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc


if __name__ == "__main__":
    main()
