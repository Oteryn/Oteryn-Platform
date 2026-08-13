# Oteryn-v2 Game Enforcement Command / Result Contract

## Status

`ACCEPTED PLATFORM CONSUMER / ORCHESTRATION ARCHITECTURE CONTRACT — OTERYN-V2 TRANSPORT AND RUNTIME IMPLEMENTATION DEFERRED`

This contract defines the semantic boundary used when Oteryn Platform support or moderation workflows request an authoritative native game sanction or runtime enforcement action from Oteryn-v2.

It does **not** define or authorize transport, endpoints, IDL bytes, game persistence, runtime enforcement algorithms, anti-cheat policy, deployment, production activation or direct/shared SQL. It does not enable any sanction profile by itself.

## Authority split

Oteryn Platform owns:

- support cases, player/content/guild reports and moderator workflow;
- authenticated moderator authorization, confirmed-MFA and exact permission gates;
- Platform policy decision, evidence references, appeal workflow and user communication;
- stable enforcement-operation orchestration, delivery/reconciliation state, notification state and privacy-safe Platform audit;
- Platform-local restrictions that do not claim game-runtime effect.

Oteryn-v2 owns:

- canonical game subject identity and current authoritative subject state;
- whether a requested sanction profile is supported and applicable to the current game state;
- atomic sanction application, replacement, revocation and expiry in the game domain;
- active-session/runtime enforcement and the authoritative enforcement result or receipt;
- game-owned ordering/fencing required to prevent an older command from weakening a newer restrictive decision.

A Platform moderator decision authorizes one bounded **request**. Dispatch, queue acceptance, a Platform status or a notification never proves game enforcement. Only an authoritative game result with applicable identity and revision proves game-domain effect.

## Trust and identity

Every request is created server-side after current Platform authorization. Browser-provided account, character, sanction, operation, case or revision identifiers are untrusted references and never establish ownership or authority.

The semantic envelope preserves:

```text
GameEnforcementCommand
  operation_id
  command_family
  command_semantic_version
  policy_revision
  decision_revision
  decided_at
  actor_context_reference
  source_case_reference?
  target
    AccountId?
    CharacterId?
    WorldId / ChannelId?
    network_or_device_scope?       # only under a separately accepted privacy/security policy
  intent
    apply | replace | revoke | expire | reconcile
  sanction_profile
  scope
  effective_at
  expires_at?
  reason_category
  evidence_reference_set
  precondition_evidence?
  correlation_id
```

Exact wire names are deferred. Free-form private case bodies, reporter identity, moderator notes, raw network identifiers, credentials and secrets are not command payload or ordinary telemetry. Evidence references are opaque, least-privilege handles; the game service receives only fields required to execute and audit its bounded responsibility.

Canonical native `AccountId` and game-owned `CharacterId` are used where applicable. Legacy Canary numeric IDs remain inside compatibility adapters and cannot leak into the native contract as canonical identity.

## Command families and capability negotiation

The common envelope may support separately versioned profiles such as:

- deny new game admission for an account;
- disconnect or restrict an active account/character session;
- character-scoped or world/channel-scoped game restriction;
- revoke, replace or expire a prior game sanction;
- reconcile an earlier operation or authoritative sanction state.

IP, device, chat, trade, market, movement or other specialized restrictions are **not** implied. Each profile requires accepted product/security/privacy policy, typed scope, minimum data, game capability declaration, failure semantics and focused validation before activation.

Unsupported family/version/profile/scope fails closed before a privileged effect. Platform must not approximate an unsupported narrow sanction with a broader sanction.

## Stable operation and decision identity

`operation_id` identifies one semantic attempt. The producer records a semantic fingerprint on first acceptance.

- exact retries use the same operation identity and converge on the same state/result;
- a materially different payload under the same identity fails closed;
- timeout or response loss never causes Platform to mint a replacement operation while the first may still apply;
- a new moderator decision uses a new operation identity and a strictly newer `decision_revision` for the same authoritative sanction stream;
- appeal submission alone is not a revoke command; only an authorized appeal outcome creates a newer decision.

At-least-once delivery is assumed. Exactly-once network delivery is not.

## Ordering and restrictive fencing

Commands for the same authoritative target/sanction stream are ordered by a stable stream identity plus monotonic `decision_revision`; timestamps alone are not authority.

The game authority must reject or return the already-authoritative state for stale revisions. An older delayed `apply`, `replace`, `revoke` or `expire` cannot overwrite a newer decision. Restore from backup, replay or failover cannot lower the accepted high-water revision without an explicit disaster-recovery reconciliation procedure.

Restriction precedence is fail closed:

- a broader/newer active restriction is not weakened by an older narrow or revoke command;
- revoke/expire names the sanction or stream it changes and cannot remove unrelated restrictions;
- replacement is atomic from the game-domain perspective and does not create an unintended enforcement gap;
- expiry is evaluated by authoritative game policy/time and becomes observable through an authoritative result/projection, not inferred solely from a Platform clock.

## Authoritative result lifecycle

The game boundary returns or exposes a reconcilable state equivalent to:

```text
UNKNOWN_TO_GAME
ACCEPTED_PENDING
APPLIED
REPLACED
REVOKED
EXPIRED
REJECTED
```

`APPLIED`, `REPLACED`, `REVOKED` and `EXPIRED` are terminal for that operation identity and include:

- operation identity and semantic version;
- authoritative target/sanction stream identity;
- accepted decision revision and policy/capability revision;
- effective sanction state and bounded scope;
- authoritative result/receipt revision;
- enforcement time and, where applicable, expiry;
- enough non-secret correlation to reconcile Platform state.

`ACCEPTED_PENDING` means responsibility was durably accepted, not that runtime effect completed. `UNKNOWN_TO_GAME` is safe only when the game authority can prove the operation was never accepted and cannot later commit.

Platform-local `AMBIGUOUS` is used after timeout, connection loss or response loss where acceptance/effect is unknown. Platform reconciles the same operation identity until an authoritative state is known or an operator-visible incident remains. It does not display or notify game enforcement as completed.

## Typed rejection and transport failures

Terminal rejection preserves categories at least equivalent to:

- invalid or unsupported command/version/profile/scope;
- service authority/authentication failure before acceptance;
- target not found or not applicable, with privacy-preserving disclosure;
- stale decision/precondition revision;
- target identity or ownership mismatch;
- lifecycle/session/topology conflict;
- operation identity conflict;
- policy/capability revision conflict;
- non-retryable internal failure where no effect can later commit.

Dependency unavailable, retryable internal failure and any failure after possible acceptance remain pending/ambiguous, not terminal rejection. User-facing messages map typed outcomes to bounded localized language and do not expose private subject existence, anti-abuse rules, internal topology or moderator evidence.

## Apply, revoke, expire and appeal semantics

- `apply` creates or activates the requested typed restriction only after game-owned applicability checks.
- `replace` atomically changes the named sanction stream under a newer decision revision.
- `revoke` is an explicit newer authorized decision and never follows automatically from an appeal submission.
- `expire` records/reconciles the authoritative end of a time-bounded sanction; it is idempotent and cannot affect another stream.
- an appeal is Platform workflow truth. Accepted appeal outcomes may create a revoke/replace request; rejected or pending appeals do not mutate game state.
- Platform communication state may be ahead of or behind game reconciliation but must label the distinction truthfully.

## Active sessions and admission

For profiles promising immediate runtime effect, successful application is not proven until the authoritative result states the required active-session action reached its terminal policy outcome. A database row, queued disconnect or admission-policy update alone is insufficient.

New-admission denial and active-session termination are distinct capabilities. If only new-admission denial is supported, Platform must not claim that an already-online session was disconnected. Game Login Ticket, pre-admission and final game admission contracts continue to own their respective authorization stages.

## Reconciliation and projections

Platform retains one operation ledger per semantic attempt and can query or consume authoritative results by `operation_id`. Reconciliation is bounded, observable and operator-recoverable.

A read projection may expose current authoritative game-sanction state to authorized Platform workflows, but it is derived and carries source revision, freshness, current-owner epoch and degraded/unavailable state. Stale or unavailable projection state cannot prove absence of a sanction, cannot authorize a broader action and cannot overwrite a newer receipt.

## Security, privacy and audit

- service-to-service authentication is least privilege, audience-bound, rotatable and separate from human moderator identity;
- Platform authorization is rechecked at decision time; game authority authenticates the calling service and validates supported command scope;
- MFA is an additional gate, never authorization by itself;
- evidence and reason data are minimized; reporter identity and private notes do not cross the boundary by default;
- logs, metrics and traces contain opaque operation/correlation identifiers, typed state/codes and safe revisions, not free-form evidence, tokens or raw network/device values;
- both systems retain tamper-evident or append-oriented audit sufficient to correlate who authorized the Platform decision and what authoritative game result occurred, without duplicating private content;
- retention/deletion in Platform support records cannot erase or silently revoke game-owned sanction truth; each authority applies its accepted legal/security retention policy.

## Legacy compatibility, rollout and rollback

Current `enforcement_records` remain Platform workflow/communication records. Existing Canary bans/account status remain Legacy Canary Compatibility authority and are not rewritten by this contract.

Activation requires:

1. separately authorized Oteryn-v2 producer implementation and accepted transport/IDL;
2. capability/version negotiation and deterministic shared fixtures;
3. least-privilege service identity and credential rotation/revocation proof;
4. idempotency, conflicting-reuse, ordering, stale-revision, ambiguity and reconciliation tests;
5. apply/replace/revoke/expire and active-session/admission negative-path E2E for each enabled profile;
6. privacy/redaction, retention and operator-recovery review;
7. shadow evidence that compares behavior without dual-writing authoritative sanctions;
8. an explicit per-profile cutover and rollback gate.

Rollback disables new native dispatch while preserving reconciliation of already accepted operations and authoritative game sanctions. It must not revert to an older decision revision, silently dual-write Canary and native authorities, or report a Platform record as game enforcement.

## Required contract tests

Shared producer/consumer fixtures and integration evidence cover at least:

1. exact retry returns the same operation/result;
2. conflicting reuse fails closed;
3. response loss reconciles the original operation;
4. stale apply/revoke/expire cannot override a newer decision;
5. atomic replace creates no unintended enforcement gap;
6. appeal submission causes no game mutation;
7. accepted appeal outcome creates a newer bounded revoke/replace operation;
8. unsupported narrow scope is not broadened silently;
9. target mismatch and private non-existence do not leak sensitive facts;
10. new-admission denial is not misreported as active-session disconnect;
11. projection staleness/unavailability is distinct from no sanction;
12. restore/failover preserves or safely re-establishes the decision high-water fence;
13. logs/audit/notifications exclude private evidence and credentials;
14. mixed-version rollout and rollback preserve one authoritative sanction stream.

## Explicitly deferred

- exact transport, endpoint, schema/IDL bytes and delivery infrastructure;
- game-side sanction model, persistence, locking, timers and runtime hooks;
- exact sanction catalogue, durations, evidence standards and moderator policy;
- IP/device or other high-risk identifying scopes;
- production identities, secrets, deployment and activation;
- external repository implementation.
