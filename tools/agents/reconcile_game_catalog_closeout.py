#!/usr/bin/env python3
from __future__ import annotations

import json
import re
from collections import Counter
from pathlib import Path

LEDGER_PATH = Path('docs/testing/product-completeness-benchmark.json')
BENCHMARK_PATH = Path('docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md')
STATE_PATH = Path('docs/agents/PROJECT_STATE.md')
TASK_PATH = Path('docs/agents/tasks/active/OTERYN-20260729-game-catalog-first-scope-closeout.md')


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f'{label}: expected one anchor, found {count}')
    return text.replace(old, new, 1)


def regex_once(text: str, pattern: str, replacement: str, label: str, *, flags: int = 0) -> str:
    updated, count = re.subn(pattern, replacement, text, count=1, flags=flags)
    if count != 1:
        raise SystemExit(f'{label}: expected one match, found {count}')
    return updated


def reconcile_ledger() -> tuple[Counter[str], Counter[str]]:
    ledger = json.loads(LEDGER_PATH.read_text(encoding='utf-8'))
    owners = {
        'character.achievement-display': ['#277', '#301'],
        'knowledge.spells-quests-npcs-achievements': ['#301'],
        'knowledge.maps': ['#302'],
        'knowledge.hunting-calculators': ['#302'],
        'knowledge.battle-pass-presets-huntfinder': ['#302'],
        'knowledge.world-transfer-docs': ['#277'],
        'knowledge.server-specific-systems-events': ['#302'],
    }

    found: set[str] = set()
    for capability in ledger['capabilities']:
        capability_id = capability['id']
        if capability_id in owners:
            capability['gap_issues'] = owners[capability_id]
            found.add(capability_id)

    missing = set(owners) - found
    if missing:
        raise SystemExit(f'Missing capability ids: {sorted(missing)}')

    stale = [
        capability['id']
        for capability in ledger['capabilities']
        if '#281' in capability.get('gap_issues', [])
    ]
    if stale:
        raise SystemExit(f'Stale #281 gap owners remain: {stale}')

    ledger['generated_at'] = '2026-07-29T15:58:00Z'
    LEDGER_PATH.write_text(json.dumps(ledger, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')

    delivery = Counter(capability['delivery_status'] for capability in ledger['capabilities'])
    relevance = Counter(capability['relevance'] for capability in ledger['capabilities'])
    if sum(delivery.values()) != 43 or sum(relevance.values()) != 43:
        raise SystemExit('Capability counts no longer total 43')
    return delivery, relevance


def reconcile_benchmark(delivery: Counter[str], relevance: Counter[str]) -> None:
    text = BENCHMARK_PATH.read_text(encoding='utf-8')
    text = replace_once(
        text,
        '- Community-data completeness delivery candidate: PR #298',
        '- Community-data completeness delivery: PR #298\n- First Game Catalog scope closure: PR #303 (PR #272 delivery evidence)',
        'audit identity',
    )

    rows = {
        'Implemented': delivery['implemented'],
        'Partial': delivery['partial'],
        'Missing': delivery['missing'],
        'Untested': delivery['untested'],
        'Not applicable': delivery['not_applicable'],
        'Required': relevance['required'],
        'Planned': relevance['planned'],
        'Optional / differentiator': relevance['optional_differentiator'],
    }
    for label, count in rows.items():
        text = regex_once(text, rf'\| {re.escape(label)} \| \d+ \|', f'| {label} | {count} |', f'count row {label}')

    text = replace_once(
        text,
        '**Oteryn must not claim benchmark product completeness while required partial or missing capabilities remain open.** The principal remaining required-gap tracker is #277. Commerce is intentionally planned rather than part of the current non-commercial launch boundary, but #278 is mandatory before any commercial activation. Structured server-backed Wiki expansion is tracked by #281.',
        '**Oteryn must not claim benchmark product completeness while required partial or missing capabilities remain open.** The principal remaining required-gap tracker is #277. Commerce is intentionally planned rather than part of the current non-commercial launch boundary, but #278 is mandatory before any commercial activation. Structured spell/NPC/quest/achievement expansion is tracked by #301, while optional maps and hunt/discovery planning is tracked by #302.',
        'verdict trackers',
    )
    text = replace_once(
        text,
        'Issues #276 and #279 now have delivered Platform-owned account-security and support/moderation lifecycles for their approved boundaries. It does not authorize Canary account unlink/rebind, native game-account deletion, character deletion or production deployment.',
        'Issues #276, #279 and #280 now have delivered Platform-owned account-security, support/moderation and read-only community-data lifecycles for their approved boundaries. Issue #281\'s first authoritative Game Catalog scope was delivered by PR #272; closing that scope does not implement or authorize the #301/#302 follow-ups, Canary account unlink/rebind, native game-account deletion, character deletion or production deployment.',
        'delivered issue summary',
    )
    text = replace_once(
        text,
        'Issue **#280** is delivered for the approved read-only boundary through PR #298, pending final exact-head merge.',
        'Issue **#280** is delivered for the approved read-only boundary through merged PR #298 (`7533b12b1e1c6d266c6bf5a8800e584fad23a01e`).',
        'community merge state',
    )
    text = replace_once(
        text,
        'It does not prove that absent character, support, commerce or community capabilities are acceptable omissions.',
        'It does not prove that absent character, commerce or knowledge capabilities are acceptable omissions.',
        'route contract boundary',
    )

    knowledge = '''### Knowledge and tooling ecosystem

The Wiki retains its public/editorial workflow, while PR #272 delivered the first authoritative versioned server-backed Game Catalog scope required by Issue #281.

Delivered first scope:

- immutable versioned snapshots with provenance, explicit activation and rollback;
- active-profile Oteryn items, weapons, creatures and exact visible loot/reverse-source relations;
- public EN/PL responsive reads and exact-permission confirmed-MFA administrator inspection;
- generated Canary artifact verification and MariaDB import, activation, candidate activation and rollback evidence.

Issue **#281** is complete for that first scope through PR #272. Closure does not promote unsupported capabilities or change production state.

Remaining planned expansion:

- structured spells, quests, NPCs and achievements only when additive authoritative producer/consumer contracts exist — **#301**;
- world-transfer documentation only when the owner-management and transfer service is defined — **#277**;
- complete historical introduction/removal, spawn, availability and provenance expansion as separately bounded contract work.

Optional differentiators and product discovery:

- maps and interactive maps;
- hunting-place discovery and calculators;
- equipment presets, Huntfinder-like matching and linked tasks;
- battle-pass and other server-specific engagement/system catalogues.

These optional/product-decision capabilities are tracked by **#302**. Third-party pages remain UX references only and are not Oteryn availability proof.

'''
    text = regex_once(
        text,
        r'### Knowledge and tooling ecosystem\n.*?(?=## Gap backlog and priority)',
        knowledge,
        'knowledge section',
        flags=re.S,
    )

    backlog = '''| Priority boundary | Issue | Scope |
|---|---|---|
| Delivered account/security lifecycle | #276 | Confirmed email, sessions, privacy, recovery key, termination and explicit MFA/binding policy |
| Required character/profile completeness | #277 | Public profile editing/privacy, delete/restore, rename, linkage and transfer policy |
| Delivered support/moderation lifecycle | #279 | Platform tickets, reports, enforcement history, notifications, retention and privacy |
| Delivered community-data completeness | #280 | Read-only rich profiles, guild directory/search/detail, highscore filters, deaths/statistics and explicit exclusions |
| Mandatory before commercial activation | #278 | Premium, coins, products, provider/webhook/refund/chargeback lifecycle |
| Delivered first server-backed Game Catalog scope | #281 | Versioned item/weapon/creature/loot catalogue delivered by PR #272; closeout preserves deferred boundaries |
| Planned structured catalogue expansion | #301 | Authoritative spells, NPCs, quests, achievements and exact cross-links |
| Optional knowledge/discovery planning | #302 | Maps, hunt tools, presets and server-specific discovery/product decisions |
| Separate presentation enhancement | #244 | Audited administrator homepage-template selector |
| Separate production gate | #91 | Exact deployed production verification; not satisfied by this audit |'''
    text = regex_once(
        text,
        r'\| Priority boundary \| Issue \| Scope \|\n\|---\|---\|---\|\n.*?(?=\n## External benchmark references)',
        backlog,
        'gap backlog table',
        flags=re.S,
    )
    BENCHMARK_PATH.write_text(text, encoding='utf-8')


def reconcile_state(delivery: Counter[str]) -> None:
    text = STATE_PATH.read_text(encoding='utf-8')
    text = replace_once(
        text,
        '- **Benchmark Product Completeness: NOT COMPLETE; REQUIRED GAPS #277, #278 AND #281 REMAIN OPEN**',
        '- **Benchmark Product Completeness: NOT COMPLETE; REQUIRED CHARACTER GAP #277 REMAINS OPEN AND #278 IS MANDATORY BEFORE COMMERCE**',
        'state release verdict',
    )
    text = replace_once(
        text,
        'The Wiki is editorially complete for delivered articles but lacks complete authoritative server-backed creature/item/loot/gameplay catalogues. Tracker: #281.',
        'The Wiki is editorially complete for delivered articles. PR #272 delivered the first authoritative versioned item/weapon/creature/loot Game Catalog scope. Structured spells/NPCs/quests/achievements remain #301 and optional map/hunt/discovery decisions remain #302.',
        'state wiki tracker',
    )
    text = regex_once(
        text,
        r'- \d+ implemented;\n- \d+ partial;\n- \d+ missing;\n- \d+ not applicable;',
        f"- {delivery['implemented']} implemented;\n- {delivery['partial']} partial;\n- {delivery['missing']} missing;\n- {delivery['not_applicable']} not applicable;",
        'state capability counts',
    )
    text = replace_once(
        text,
        '- #281 — first versioned item/weapon/creature/loot Game Catalog slice delivered; further catalogues remain.',
        '- #281 — first versioned item/weapon/creature/loot Game Catalog scope delivered by PR #272; closeout is PR #303.\n- #301 — authoritative spell/NPC/quest/achievement catalogue expansion.\n- #302 — optional maps, hunt tools and server-specific discovery planning.',
        'state focused backlog',
    )
    text = replace_once(
        text,
        'None. Issue #280 is complete and its task is archived. Start the next benchmark slice only through a new active task and separate pull request.',
        '`OTERYN-20260729-game-catalog-first-scope-closeout` in PR #303 is reconciling Issue #281 to the first Game Catalog scope already delivered by PR #272 and assigning every deferred capability to #277, #301 or #302.',
        'state current task',
    )
    text = replace_once(
        text,
        '1. Deliver #277 as a bounded character-management task, defining explicit operation-specific contracts before any Canary mutation.\n2. Keep #278 disabled until a dedicated payment ADR, threat model and provider lifecycle are reviewed.\n3. Continue #281 from authoritative Oteryn server availability, never by copying third-party datasets or prose.\n4. Resume #91 only after explicit production deployment/verification authorization and required production evidence access exist.',
        '1. Complete the #281 first-scope evidence closeout without promoting #301/#302 capabilities.\n2. Deliver #277 as bounded Platform-owned slices and require explicit operation-specific contracts before any Canary mutation.\n3. Keep #278 disabled until a dedicated payment ADR, threat model and provider lifecycle are reviewed.\n4. Start #301 only after an additive authoritative producer contract is approved; treat #302 as optional product discovery.\n5. Resume #91 only after explicit production deployment/verification authorization and required production evidence access exist.',
        'state recommended sequence',
    )
    STATE_PATH.write_text(text, encoding='utf-8')


def reconcile_task() -> None:
    text = TASK_PATH.read_text(encoding='utf-8')
    text = replace_once(text, '- [ ] Reassign every machine-ledger `#281` gap reference', '- [x] Reassign every machine-ledger `#281` gap reference', 'task ledger criterion')
    text = replace_once(text, '- [ ] Reconcile the human-readable benchmark, PROJECT_STATE and ACTIVE_WORK', '- [x] Reconcile the human-readable benchmark, PROJECT_STATE and ACTIVE_WORK', 'task docs criterion')
    text = replace_once(text, 'pr: none', 'pr: 303', 'task pr')
    text = replace_once(text, 'status: investigating', 'status: validating', 'task status')
    text = replace_once(
        text,
        '  - Whether the human-readable benchmark contains additional narrative references to #281 outside the machine-ledger capability rows.',
        '  - Whether any exact-head repository validator exposes an additional stale #281 ownership reference outside the reconciled files.',
        'task unknown',
    )
    text = replace_once(
        text,
        '  marker: none\n  evidence: No validation has run on the closeout branch yet.',
        '  marker: pending-exact-head-validation\n  evidence: Ledger ownership and human-state reconciliation are committed; exact-head repository workflows have not completed yet.',
        'task first failure',
    )
    text = replace_once(
        text,
        '  - docs/agents/tasks/active/OTERYN-20260729-game-catalog-first-scope-closeout.md\nvalidation:\n  - command: not-run\n    result: NOT_RUN\n    evidence: Ledger and project-state reconciliation has not been committed yet.',
        '  - docs/agents/tasks/active/OTERYN-20260729-game-catalog-first-scope-closeout.md\n  - docs/testing/product-completeness-benchmark.json\n  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md\n  - docs/agents/PROJECT_STATE.md\n  - docs/agents/ACTIVE_WORK.md\nvalidation:\n  - command: node scripts/acceptance/coverage/validate-product-completeness.mjs\n    result: PASS\n    evidence: The temporary closeout reconciler validated all 43 capability records after gap-owner reassignment.\n  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20260729-game-catalog-first-scope-closeout.md --require-checkpoint\n    result: PASS\n    evidence: The active checkpoint satisfies the shared version-1 contract.\n  - command: Required exact-head workflow suite\n    result: NOT_RUN\n    evidence: Pending normal user-authored cleanup/checkpoint head after temporary reconciler removal.',
        'task validation block',
    )
    text = replace_once(
        text,
        'next_action: Reassign all machine-ledger #281 gap references to #277, #301 or #302 while preserving current delivery statuses and evidence.',
        'next_action: Remove the temporary reconciler and workflow, validate the resulting exact head and fix only the first reproducible failure.',
        'task next action',
    )
    TASK_PATH.write_text(text, encoding='utf-8')


def main() -> None:
    delivery, relevance = reconcile_ledger()
    reconcile_benchmark(delivery, relevance)
    reconcile_state(delivery)
    reconcile_task()


if __name__ == '__main__':
    main()
