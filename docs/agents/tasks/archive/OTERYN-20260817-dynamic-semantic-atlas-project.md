---
task_id: OTERYN-20260817-dynamic-semantic-atlas-project
repository: blakinio/Oteryn-Platform
mode: architecture
status: completed
closed_at_utc: 2026-08-17T22:00:38Z
pr: 1139
validated_head: e2a5746d08a725225b604b56667d94f057662490
merge_sha: 93873c57c6d640676005841309e6da6d552ad867
programme_alias: DYN-ATLAS
---

# OTERYN-20260817-dynamic-semantic-atlas-project — COMPLETED

## Terminal result

PR #1139 was marked Ready after the normal merge gate passed and squash-merged to protected `main` as `93873c57c6d640676005841309e6da6d552ad867`.

The merge establishes the documentation baseline for the Dynamic Semantic Atlas programme without changing runtime, production, Game repository state or deployment configuration.

Canonical programme artifacts created by the task:

- `docs/architecture/oteryn-dynamic-semantic-atlas.md`;
- `docs/maps/oteryn-dynamic-semantic-atlas-program.md`;
- `docs/maps/oteryn-dynamic-semantic-atlas-execution-prompt.md`.

## Accepted programme invariants

- The existing main Atlas is evolved rather than replaced by a parallel map product.
- Atlas is a derived semantic projection/read model and never a second canonical World/Content authority.
- Oteryn-Game/current Oteryn-v2 lineage remains canonical World/Content, exporter and mutable gameplay authority.
- Platform `PlayerCompanion` remains the owner of personalized hunt guidance, player progress/planning and recommendations.
- Target capability families include World Map, World Knowledge, Hunt Intelligence, Player Progress/Bounty Companion and Authorized Live Atlas.
- NPC/shop/monster/public-loot knowledge is projection-driven and Game allowlist-controlled.
- Exact live player placement is privacy/consent controlled and not public by default.
- A future bounty/task web mutation requires an explicit authenticated Game command contract; Game Gateway is not repurposed as a generic gameplay API.
- Svelte 5 + TypeScript + PixiJS 8 + WebGL2 is the preferred DYN-ATLAS-001 browser proof candidate, not an irreversible technology freeze.
- FlatBuffers, Protobuf, JSON/JSONL, compression, permanent chunk size/floor packing and canonical coordinate/floor/stack semantics remain evidence-gated.
- DYN-ATLAS-001 remains a bounded static Semantic Thais Z7 proof; live state, bounty mutation, full map conversion, editor writes and raster retirement are explicitly out of scope.

## Upstream evidence preserved

Read-only Game evidence was pinned to `blakinio/Oteryn-v2@5577f6fc7c1f7ddef482f0f7b08039047704e36b`.

The programme preserves the current Oteryn-v2 `OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1` boundary and its physical-profile readiness `EVIDENCE_GAP`. No Platform document promotes the nonbinding ANL-02 candidate to accepted Game authority.

## Final exact-head validation

```yaml
validated_head: e2a5746d08a725225b604b56667d94f057662490
changed_files: 4
changed_path_scope: PASS
full_diff_self_review: PASS
ci:
  run_number: 7453
  result: PASS
agent_governance:
  run_number: 7076
  result: PASS
native_protocol_contract:
  run_number: 605
  result: PASS
native_protocol_contract_audits:
  run_number: 605
  result: PASS
platform_db_outage_validation:
  run_number: 5617
  result: PASS
game_auth_ticket_concurrency:
  run_number: 5188
  result: PASS
edge_security_emulation:
  run_number: 4111
  result: PASS
phase_7_production_like_validation:
  run_number: 5690
  result: PASS
runtime_browser_e2e: NOT_APPLICABLE_DOCS_ONLY
review_submissions_before_merge: 0
review_threads_before_merge: 0
pr_comments_before_merge: 0
```

Protected `main` was `fcafc20bc9705ca92256fdddc7433bcc3d191c40` at the final pre-merge drift check and became `93873c57c6d640676005841309e6da6d552ad867` after squash merge.

## Merge evidence

```yaml
pr: 1139
ready_before_merge: true
expected_head_merge_guard: e2a5746d08a725225b604b56667d94f057662490
merge_method: squash
merge_sha: 93873c57c6d640676005841309e6da6d552ad867
main_verified_after_merge: 93873c57c6d640676005841309e6da6d552ad867
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR #1139 merged successfully through the ordinary protected same-repository path.
source_branch_evidence: direct GitHub ref lookup after merge returned 404 for refs/heads/docs/oteryn-20260817-dynamic-semantic-atlas.
```

## Remaining evidence gates

These are not blockers for the completed documentation task; they are prerequisites for later implementation/freeze decisions:

- canonical Game spatial/coordinate/floor/stack/anchor profile v1;
- first executable Game -> Atlas physical profile and production resource limits;
- exact future Oteryn-Atlas implementation repository/authority if physical extraction is not yet available;
- Game public allowlists for NPC/loot/interaction detail;
- accepted analytics producer coverage for decision-grade HuntAdvisor metrics;
- Atlas live-state/privacy contract;
- web/PlayerCompanion -> Game gameplay-command contract if bounty mutations are enabled.

## Next programme action

When an authorized target Atlas implementation repository and required Game coordinate/export evidence are available, start a separate controlled implementation task using `DYN-ATLAS-001 — Semantic Thais Z7 Proof execution prompt`.

## Lifecycle closeout

This archive record replaces `docs/agents/tasks/active/OTERYN-20260817-dynamic-semantic-atlas-project.md` and releases its Platform documentation path ownership. Cross-repository implementation remains separately authorized work.
