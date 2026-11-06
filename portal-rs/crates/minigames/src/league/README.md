# PLAT3-T94 — display-only league consumer

Base: Platform main `93f0aec532df11bd575ac3d2944a3b5587e3d585`.
Game authority: `3b49a20feec3ba29700bb4b2fa28fa9b539b79a2`, CANDIDATE rev 1.
Contract: `docs/contracts/OTERYN_GAME_WORLD_EVENT_OUTCOMES_AND_GUILD_SEASON_V1.md`.
Fixture provenance and exact hashes: `fixtures/SOURCE.md` and `tests.rs`.
This task record stays inside the owner's permitted module paths; no live
Platform Issue matching T94/PLAT3 was found during the targeted preflight.
Jira mapping/synchronization is unknown; no Jira write or PR creation is claimed.

## Delivered boundary

Strict canonical JSON with closed member order/types/absence lists, UUIDv7,
uint64 decimal counters, event windows/options/evidence, season bounds, checked
score sums and deterministic tie ordering. Unknown/duplicate/null/escaped members
and noncanonical bytes deny. Guild names temporarily accept only an ASCII subset
of PP's NFC/privacy domain; unsupported names deny the whole snapshot.

`League::ingest` is an internal trusted-adapter API, **not a network endpoint**.
It consumes only decided audit types 10/11 and their exact registered prost
payloads, bound to complete snapshot bytes and matching record/evidence hashes.
Types 6/7 remain their existing owners' responsibility. The separate snapshot
transport (including nonterminal records, withdrawals and watermarks) cannot be
mistaken for an audit decision. Those operations require the missing qualified
transport adapter and remain unavailable. No live feed is advertised.

Every write holds the module advisory transaction lock and the adapter's current
recovery fence. Receipt IDs and transaction ordinals are unique; duplicate bytes
are idempotent, receipt collisions quarantine serving, equal-pair different
content/gaps quarantine the subject. Quarantine has no automatic clearing route.
Question/season identity is immutable; correction binds the immediately prior
record and replaces the projection. Final records cannot change. Occurrence
non-reuse, final controls and receipt fingerprints survive P90D payload cleanup.
First freeze time must be authenticated by the adapter; retries cannot extend
history retention. Reads independently suppress final history at P90D and deny missing/expired signature audit or mismatched stored hashes.

## Activation and unresolved owner choices

All `Flags` default false. `OTERYN_LEAGUE_ENABLED=true` plus
`OTERYN_LEAGUE_PUBLIC_ENABLED=true` only mount `/pl/league` and `/en/league` and
navigation. The configured adapter remains `Unavailable`, with all approval and
ingest gates false, and responds with localized 503/no-store. No environment
setting upgrades it to source authority. The page router limits aggregate reads
to 120/minute per process; Askama escapes all data. There are no browser mutations,
so no CSRF-bearing forms or submission endpoints are introduced.

Trusted application composition must separately supply `Authority`, the approval
flags and the existing private TLS/ANL adapter. `Authority::fence` must lock a
nonrollback generation, accepted source namespace, complete enumeration of both
families/all retained subjects, correction lineage and fresh <=30s watermarks.
It must reject clock uncertainty above 1s, incomplete cuts, schema drift, epoch
alarms and missing evidence. `Authority::verify` must authenticate the purpose
identity, exact signature bytes/key, ANL envelope/payload hash, TransactionEventRef,
source evidence, source mappings, final-freeze time and recovery binding together.
The Ed25519 helper defines no new signed Game wire format: the candidate specifies
mTLS HTTP publishing and has no detached signature profile or signed fixture.
Missing verification stays unavailable; hashes never establish authority.

New epochs cannot update existing subjects in this implementation: complete
staging/enumeration, accepted nonrollback fence and recovery protocol remain an
adapter/owner seam. No epoch is minted on restart and stale cached state is not
used as a replacement. `expire` is a scheduler seam, not an enabled background
worker. It removes audit bytes at P90D from the immutable authenticated ANL timestamp and expired final snapshot bytes while
preserving minimal anti-replay/final-state authority controls.

Open owner questions remain gates: accepted source/advance windows/aggregation,
season calendar/challenge/scoring/eligibility/tombstone/correction policy,
immutable final freeze/repair, PKI and measured production limits. Each approval
flag defaults OFF; no product policy is selected here. **Predictions are always
closed**; there is no prediction window advertised, no scoring of submissions,
no stakes, fees, Wallet/Store operation, reward or gameplay advantage.

T88 implements daily public facts and explicitly supplies no T94 outcome source.
T89's production journal/cosmetic authority adapters remain unavailable and do
not establish guild scoring authority. Neither is used to infer eligibility.
G-12 producer/consumer interrupted-flow qualification and environment rollout
remain pending; this implementation cannot mark the gate production-ready.

## Validation

Tests cover exact pinned bytes/hashes, Ed25519 verification and tampering, strict
wire grammar, evidence absence, correction chains, fixed question fields, frozen
lineage, rank/score/coverage refusal, clock/watermark failure, disabled/503 HTTP,
pl/en responsive HTML, and PostgreSQL concurrent replay, durable restart,
recovery mismatch, final mutation quarantine and retention. PostgreSQL tests use
`PORTAL_TEST_DATABASE_URL`, isolated schemas and the complete migration chain.
An unset variable emits a skip notice and is not PostgreSQL qualification.

Required: `cargo fmt --all --check`,
`cargo clippy -q --workspace --all-targets -- -D warnings`,
`cargo test -q --workspace` from `portal-rs/` with the database harness configured.
