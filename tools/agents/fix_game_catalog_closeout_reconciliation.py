#!/usr/bin/env python3
from __future__ import annotations

import json
import re
from collections import Counter
from pathlib import Path

ledger = json.loads(Path('docs/testing/product-completeness-benchmark.json').read_text(encoding='utf-8'))
delivery = Counter(capability['delivery_status'] for capability in ledger['capabilities'])
relevance = Counter(capability['relevance'] for capability in ledger['capabilities'])

if sum(delivery.values()) != 43 or sum(relevance.values()) != 43:
    raise SystemExit('Capability counts must total 43')
if any('#281' in capability.get('gap_issues', []) for capability in ledger['capabilities']):
    raise SystemExit('Machine ledger still contains #281 as a remaining gap owner')

benchmark_path = Path('docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md')
benchmark = benchmark_path.read_text(encoding='utf-8')

delivery_table = f'''| Delivery status | Count |
|---|---:|
| Implemented | {delivery['implemented']} |
| Partial | {delivery['partial']} |
| Missing | {delivery['missing']} |
| Untested | {delivery['untested']} |
| Not applicable | {delivery['not_applicable']} |'''
benchmark, count = re.subn(
    r'\| Delivery status \| Count \|\n\|---\|---:\|\n.*?(?=\n\nRelevance classification:)',
    delivery_table,
    benchmark,
    count=1,
    flags=re.S,
)
if count != 1:
    raise SystemExit(f'Delivery table replacement count: {count}')

relevance_table = f'''| Relevance | Count |
|---|---:|
| Required | {relevance['required']} |
| Planned | {relevance['planned']} |
| Optional / differentiator | {relevance['optional_differentiator']} |
| Not applicable | {relevance['not_applicable']} |'''
benchmark, count = re.subn(
    r'\| Relevance \| Count \|\n\|---\|---:\|\n.*?(?=\n\n\*\*Oteryn)',
    relevance_table,
    benchmark,
    count=1,
    flags=re.S,
)
if count != 1:
    raise SystemExit(f'Relevance table replacement count: {count}')
benchmark_path.write_text(benchmark, encoding='utf-8')

state_path = Path('docs/agents/PROJECT_STATE.md')
state = state_path.read_text(encoding='utf-8')

old_tracker = 'Still not benchmark-complete: owner-editable character information, character-level privacy, selected achievements, deletion/restore, rename, controlled transfer and optional main-character policy remain #277. Customer commerce remains #278, and further authoritative server-backed catalogues remain #281.'
new_tracker = 'Still not benchmark-complete: owner-editable character information, character-level privacy, selected achievements, deletion/restore, rename, controlled transfer and optional main-character policy remain #277. Customer commerce remains #278. Structured authoritative spell/NPC/quest/achievement catalogues remain #301, while optional map/hunt/discovery decisions remain #302.'
if old_tracker in state:
    state = state.replace(old_tracker, new_tracker, 1)
elif new_tracker not in state:
    raise SystemExit('Project-state remaining tracker anchor not found')

count_block = (
    f"- {delivery['implemented']} implemented;\n"
    f"- {delivery['partial']} partial;\n"
    f"- {delivery['missing']} missing;\n"
    f"- {delivery['not_applicable']} not applicable;\n"
    f"- {relevance['required']} required, {relevance['planned']} planned, "
    f"{relevance['optional_differentiator']} optional/differentiator and {relevance['not_applicable']} not applicable."
)
state, count = re.subn(
    r'- \d+ implemented;\n- \d+ partial;\n- \d+ missing;\n- \d+ not applicable;\n- \d+ required, \d+ planned, \d+ optional/differentiator and \d+ not applicable\.',
    count_block,
    state,
    count=1,
)
if count != 1:
    raise SystemExit(f'Project-state count block replacement count: {count}')

old_slices = '''Completed focused slices:

- #276 — Platform-owned account security and lifecycle, merged in PR #283;
- #279 — Platform-owned support and moderation lifecycle, merged in PR #293;
- #280 — read-only community statistics and guild discovery with privacy-aware profiles, merged in PR #298.

Open focused backlog:

- #277 — character management and public profiles;
- #278 — premium, coins and entitlement commerce;
- #281 — first versioned item/weapon/creature/loot Game Catalog scope delivered by PR #272; closeout is PR #303.
- #301 — authoritative spell/NPC/quest/achievement catalogue expansion.
- #302 — optional maps, hunt tools and server-specific discovery planning.'''
new_slices = '''Completed focused slices:

- #276 — Platform-owned account security and lifecycle, merged in PR #283;
- #279 — Platform-owned support and moderation lifecycle, merged in PR #293;
- #280 — read-only community statistics and guild discovery with privacy-aware profiles, merged in PR #298;
- #281 — first versioned item/weapon/creature/loot Game Catalog scope delivered by PR #272; evidence closeout is PR #303.

Open focused backlog:

- #277 — character management and public profiles;
- #278 — premium, coins and entitlement commerce;
- #301 — authoritative spell/NPC/quest/achievement catalogue expansion;
- #302 — optional maps, hunt tools and server-specific discovery planning.'''
if old_slices in state:
    state = state.replace(old_slices, new_slices, 1)
elif new_slices not in state:
    raise SystemExit('Project-state focused-slice block not found')

state_path.write_text(state, encoding='utf-8')
