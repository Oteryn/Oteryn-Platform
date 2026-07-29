#!/usr/bin/env python3
from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one anchor, found {count}")
    return text.replace(old, new, 1)


ledger_path = ROOT / "docs/testing/product-completeness-benchmark.json"
ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
capabilities = {item["id"]: item for item in ledger["capabilities"]}

contract_evidence = {
    "file": "docs/contracts/CHARACTER_PROFILE_PREFERENCES_CONTRACT.md",
    "marker": "Effective privacy",
    "level": "repository_contract",
}
feature_evidence = {
    "file": "tests/Feature/CharacterProfiles/CharacterProfilePreferenceTest.php",
    "marker": "test_owner_can_update_escaped_comment_visibility_and_main_character",
    "level": "feature_test",
}
browser_evidence = {
    "file": "scripts/acceptance/tests/community-data-acceptance.spec.mjs",
    "marker": "@portal-community complete rankings, privacy-aware profile, owner preferences, deaths, guild search, localization, resilience and responsive lifecycle",
    "level": "browser_acceptance",
}
concurrency_evidence = {
    "file": ".github/workflows/community-data-acceptance.yml",
    "marker": "Prove single-main-character concurrency on real MariaDB",
    "level": "integration_test",
}

updates = {
    "character.public-information": {
        "delivery_status": "implemented",
        "rationale": "Authenticated owners can store a bounded Platform-owned plain-text public comment after current read-only Canary ownership verification; public rendering is escaped and audited without updating players.comment.",
        "oteryn_evidence": [contract_evidence, feature_evidence, browser_evidence],
        "gap_issues": [],
    },
    "character.privacy": {
        "delivery_status": "implemented",
        "rationale": "Per-character controls independently narrow account association, status, guild, house, skills, deaths and kill statistics. Account-level privacy remains an upper bound and hidden sibling characters are excluded from related-character presentation.",
        "oteryn_evidence": [contract_evidence, feature_evidence, browser_evidence],
        "gap_issues": [],
    },
    "character.main-selection": {
        "delivery_status": "implemented",
        "rationale": "Oteryn adopts an optional one-main-character preference per Platform Identity. Identity-row locking serializes replacement, and real-MariaDB acceptance proves that two concurrent selections leave exactly one main row.",
        "oteryn_evidence": [contract_evidence, concurrency_evidence, browser_evidence],
        "gap_issues": [],
    },
}

for capability_id, values in updates.items():
    capability = capabilities.get(capability_id)
    if capability is None:
        raise RuntimeError(f"missing capability {capability_id}")
    capability.update(values)

profile = capabilities["character.public-profile-completeness"]
profile["rationale"] = "The profile presents the approved server-backed read model plus an escaped Platform-owned owner comment, effective per-character privacy, related-character filtering and optional main-character presentation without exposing private identifiers."
profile["oteryn_evidence"] = [contract_evidence, feature_evidence, browser_evidence]

linkage = capabilities["character.guild-house-account-linkage"]
linkage["rationale"] = "Guild/rank, house and related account characters are presented through explicit per-character visibility. Account association remains private unless both the account-level upper bound and the viewed/sibling character preferences permit disclosure."
linkage["oteryn_evidence"] = [contract_evidence, feature_evidence, browser_evidence]

ledger["generated_at"] = datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
ledger_path.write_text(json.dumps(ledger, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

human_path = ROOT / "docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md"
human = human_path.read_text(encoding="utf-8")
human = replace_once(
    human,
    "- First Game Catalog scope closure: PR #303 (PR #272 delivery evidence)\n",
    "- First Game Catalog scope closure: PR #303 (PR #272 delivery evidence)\n- Character profile preferences candidate: PR #308 / Issue #307\n",
    "benchmark audit identity",
)
human = replace_once(human, "| Implemented | 20 |", "| Implemented | 23 |", "implemented count")
human = replace_once(human, "| Missing | 17 |", "| Missing | 14 |", "missing count")
human = replace_once(
    human,
    "**Oteryn must not claim benchmark product completeness while required partial or missing capabilities remain open.** The principal remaining required-gap tracker is #277. Commerce is intentionally planned rather than part of the current non-commercial launch boundary, but #278 is mandatory before any commercial activation. Structured spell/NPC/quest/achievement expansion is tracked by #301, while optional maps and hunt/discovery planning is tracked by #302.",
    "**Oteryn must not claim benchmark product completeness while required partial or missing capabilities remain open.** Issue #307 delivers the Platform-owned comment, character privacy and optional main-character portion of #277; deletion/restore, rename, transfer and authoritative achievements remain open. Commerce is intentionally planned rather than part of the current non-commercial launch boundary, but #278 is mandatory before any commercial activation. Structured spell/NPC/quest/achievement expansion is tracked by #301, while optional maps and hunt/discovery planning is tracked by #302.",
    "benchmark verdict",
)
human = replace_once(
    human,
    "| Account overview and Canary provisioning | Covered | ready, pending, recoverable, conflict, missing, retry, internal identifiers hidden |",
    "| Account overview and Canary provisioning | Covered | ready, pending, recoverable, conflict, missing, retry, internal identifiers hidden, per-character public-profile management and optional main-character state |",
    "route inventory",
)
old_character = """### Character management and public profile

Implemented subset:

- character creation with validation, quotas, ownership controls and public visibility;
- privacy-aware public character profile with approved comment, skills, guild/rank, house, deaths, kill statistics, related characters and status;
- Character Bazaar ownership transfer through escrow, which is not a world-transfer or general owner-management service.

Required gaps:

- editable public information/comment and moderation-safe rendering;
- character privacy controls;
- deletion grace period and restore;
- rename with history/cooldown and cross-surface consistency;

Planned/optional gaps:

- world or channel transfer;
- achievement selection;
- main-character selection.

Tracker: **#277** for owner-editable character mutations and character-specific policy. The approved public read model is delivered through PR #298.
"""
new_character = """### Character management and public profile

Implemented:

- character creation with validation, quotas, ownership controls and public visibility;
- privacy-aware public character profile with skills, guild/rank, house, deaths, kill statistics, related characters and status;
- authenticated owner-editable Platform comments with bounded plain-text validation, escaped rendering and audit;
- per-character visibility that can narrow account association, status, guild, house, skills, deaths and kill statistics while account-level privacy remains the upper bound;
- optional single main-character selection serialized by Identity-row locking and proven by a real-MariaDB concurrent race;
- Character Bazaar ownership transfer through escrow, which is not a world-transfer or general owner-management service.

Required gaps:

- deletion grace period and restore;
- rename with history/cooldown and cross-surface consistency.

Planned gaps:

- world or channel transfer;
- achievement selection after an authoritative source exists.

Issue **#307** delivers the Platform-owned profile-preference portion of **#277**. Parent #277 remains open only for the mutation/achievement lifecycles above; no Canary write is authorized by this slice.
"""
human = replace_once(human, old_character, new_character, "character benchmark section")
human = replace_once(
    human,
    "| Required character/profile completeness | #277 | Public profile editing/privacy, delete/restore, rename, linkage and transfer policy |",
    "| Required remaining character lifecycle | #277 | Delete/restore, rename, authoritative achievements and controlled world/channel transfer contracts; profile editing/privacy/main selection are delivered by #307 |",
    "gap backlog row",
)
human_path.write_text(human, encoding="utf-8")

project_path = ROOT / "docs/agents/PROJECT_STATE.md"
project = project_path.read_text(encoding="utf-8")
project = replace_once(
    project,
    "- character name search and privacy-aware server-backed character profiles;",
    "- character name search and privacy-aware server-backed character profiles;\n- authenticated Platform-owned character comments, per-character visibility controls and optional main-character selection;",
    "project delivered profile",
)
project = replace_once(
    project,
    "Still not benchmark-complete: owner-editable character information, character-level privacy, selected achievements, deletion/restore, rename, controlled transfer and optional main-character policy remain #277. Customer commerce remains #278. Structured authoritative spell/NPC/quest/achievement catalogues remain #301, while optional map/hunt/discovery decisions remain #302.",
    "Still not benchmark-complete: Issue #307 delivers owner-editable Platform comments, character-level privacy and optional main-character selection, while selected achievements, deletion/restore, rename and controlled transfer remain #277. Customer commerce remains #278. Structured authoritative spell/NPC/quest/achievement catalogues remain #301, while optional map/hunt/discovery decisions remain #302.",
    "project remaining profile",
)
project = replace_once(
    project,
    "The account-security fragment adds guest/authenticated EN/PL email, session, privacy, recovery-key and termination states. The Character Bazaar fragment adds public, authenticated and administrator marketplace surfaces. The community-data fragment adds highscore, profile, deaths, guild, localization, dependency-failure/recovery and responsive states.",
    "The account-security fragment adds guest/authenticated EN/PL email, session, privacy, recovery-key and termination states. The Character Bazaar fragment adds public, authenticated and administrator marketplace surfaces. The community-data and character-profile fragments add highscore, profile, owner-preference, main-character race, deaths, guild, localization, dependency-failure/recovery and responsive states.",
    "project route evidence",
)
project = replace_once(project, "- 20 implemented;", "- 23 implemented;", "project implemented count")
project = replace_once(project, "- 17 missing;", "- 14 missing;", "project missing count")
project = replace_once(
    project,
    "- #281 — first versioned item/weapon/creature/loot Game Catalog scope delivered by PR #272 and evidence ownership closed by PR #303.",
    "- #281 — first versioned item/weapon/creature/loot Game Catalog scope delivered by PR #272 and evidence ownership closed by PR #303;\n- #307 — Platform-owned character comments, per-character privacy and optional main-character selection in PR #308.",
    "project focused slices",
)
project = replace_once(
    project,
    "`OTERYN-20260729-game-catalog-first-scope-closeout` in PR #303 is reconciling Issue #281 to the first Game Catalog scope already delivered by PR #272 and assigning every deferred capability to #277, #301 or #302.",
    "`OTERYN-20260729-character-profile-preferences` in draft PR #308 delivers Issue #307, the Platform-owned profile-preference slice of parent #277, without Canary mutation.",
    "project active task",
) if "`OTERYN-20260729-game-catalog-first-scope-closeout`" in project else replace_once(
    project,
    "None. Issue #281 is closed and its closeout task is archived. Start the next benchmark slice only through a new active task and separate pull request.",
    "`OTERYN-20260729-character-profile-preferences` in draft PR #308 delivers Issue #307, the Platform-owned profile-preference slice of parent #277, without Canary mutation.",
    "project active task",
)
project = replace_once(
    project,
    "1. Deliver a Platform-owned #277 slice for per-character privacy and optional main-character preference without Canary mutation.",
    "1. Complete PR #308 / Issue #307 exact-head evidence and archive its task; keep parent #277 open for separately contracted mutation lifecycles.",
    "project sequence",
)
project += """

## Character profile preferences candidate

Draft PR #308 implements the Platform-owned Issue #307 slice: bounded escaped owner comments, per-character effective visibility, filtered related-character association, optional single main-character selection, audit events, real-MariaDB race evidence and EN/PL desktop/tablet/mobile browser acceptance. Canary remains read-only; rename, deletion, restore, transfer and selected achievements remain outside this contract, and no production claim is made.
"""
project_path.write_text(project, encoding="utf-8")

ownership_path = ROOT / "docs/architecture/DATA_OWNERSHIP.md"
ownership = ownership_path.read_text(encoding="utf-8")
if "## Character profile preference ownership" not in ownership:
    ownership += """

## Character profile preference ownership

Platform owns `character_profile_preferences`: the owner-authored public comment, per-character visibility flags and optional main-character selection. Canary remains the source of current character identity, account ownership and gameplay/profile facts. Every management write re-resolves the ready binding and reads the active Canary character before changing Platform state; stored player IDs never become ownership proof.

Account-level association and status flags are disclosure upper bounds. Per-character preferences may only narrow them. Platform comments are bounded plain text rendered escaped and do not update `players.comment`. Main-character replacement locks the Platform Identity row, writes atomically and is proven under concurrent real-MariaDB processes. This boundary authorizes no Canary rename, deletion, restore, transfer, achievement or generic player update.
"""
    ownership_path.write_text(ownership, encoding="utf-8")

for relative, heading, body in [
    (
        "docs/architecture/MODULE_CATALOG.md",
        "## Character Profile Preferences",
        """The CharacterProfiles module owns authenticated management of Platform-stored character comments, per-character public visibility and optional main-character selection. It verifies ownership from the immutable ready binding plus a fresh read-only Canary character lookup on every edit/update, records bounded audit events and projects effective privacy into PublicGameData. It does not mutate Canary or implement rename, delete, restore, transfer or achievements. Contract: `docs/contracts/CHARACTER_PROFILE_PREFERENCES_CONTRACT.md`.\n""",
    ),
    (
        "docs/architecture/SECURITY_ARCHITECTURE.md",
        "## Character profile preference security",
        """Character-profile management is authenticated and owner-scoped by server-resolved ready binding plus current read-only Canary ownership. Browser-supplied player/account identifiers do not authorize writes. Public comments are length-bounded, control-normalized and escaped at render time; audit events exclude comment content. Account privacy remains an upper bound, hidden sibling associations are filtered and generic dependency failures do not disclose SQL details. Main-character selection locks the Identity row and real-MariaDB race acceptance must leave exactly one main. No Canary mutation is authorized.\n""",
    ),
]:
    path = ROOT / relative
    text = path.read_text(encoding="utf-8")
    if heading not in text:
        text += f"\n\n{heading}\n\n{body}"
        path.write_text(text, encoding="utf-8")

print("Reconciled character profile preference benchmark and architecture documents.")
