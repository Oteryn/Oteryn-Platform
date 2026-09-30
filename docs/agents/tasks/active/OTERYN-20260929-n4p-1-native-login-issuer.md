---
task_id: OTERYN-20260929-n4p-1-native-login-issuer
governing_issue: 1419
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md
search_first:
  - app/GameAuth/NativeEvidence/NativeSigningTrustRegistry.php
  - app/GameAuth/Tickets redemption and IssueGameLoginTicket
optional_reads: []
---

# OTERYN-20260929-n4p-1-native-login-issuer

## Goal

Governing GitHub Issue: #1419 (https://github.com/Oteryn/Oteryn-Platform/issues/1419) — canonical lifecycle authority for this task. Coordination: Oteryn/Oteryn-Game#162; owner authority Q14 (comment 5899892092) and the accepted decision package Q21 (comment 5900403086). Contract: `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` from PR #1420 (read at head 892d640d; not yet merged at start).

N4P-1, testing/preproduction only. The dispatch size rule applies: hand-written logic above ~500 lines is split at a real seam. This packet delivers the first part, **N4P-1a: FND-04 grant issuer and trust-registry enforcement** (contract §8, §9, §16 signer tests). The native login endpoint (native ticket redemption to AccountId, `attempt_ref` transaction, projection-store Character check, Registry route selection with default-deny native login policy, error mapping, rate limit on `(attempt_ref, ticket hash)`) is **N4P-1b**, a separate packet and PR.

## Acceptance criteria

- [x] Ed25519 JWS `oteryn-pre-admission-v1`: exact header, exactly the §8.2 claims in contract order, TTL 5..30 s (default 20 s, U4), 32-byte CSPRNG `jti`, size bounds.
- [x] Private key only from an injected absolute secret file path (owner-only mode, canonical base64url seed); current plus optional retiring key; no key in repository, config values, fixtures or logs; test keys generated in tests.
- [x] Issuer self-check before signing (cached at most 5 s): switch on, `testing`/`preproduction` only, `kid` trusted in the fixed fresh scope and bound to the loaded public key; otherwise `NativeAdmissionUnavailable` (`NATIVE_LOGIN_UNAVAILABLE`).
- [x] Deterministic re-sign of a stored signing input returns a byte-identical token.
- [x] `NativeSigningTrustRegistry` fresh scope: re-keying a `kid` refused, third trusted key refused, revoked `kid` never re-trusted.
- [ ] N4P-1b native login endpoint (separate PR).
- [ ] Dedicated process pool under its own Unix user serving only the issuer route (deployment; excluded from this slice, lands with N4P-1b qualification).

## Ownership

```yaml
owned_paths:
  - app/GameAuth/NativeAdmission/**
  - app/GameAuth/NativeEvidence/NativeSigningTrustRegistry.php
  - config/game-auth.php
  - tests/Feature/GameAuth/NativeAdmission/**
  - docs/agents/tasks/active/OTERYN-20260929-n4p-1-native-login-issuer.md
modules:
  - GameAuth/NativeAdmission
  - GameAuth/NativeEvidence
dependencies:
  - PR #1420 contract candidate (head 892d640d)
blockers:
  - none
cross_repository_tasks:
  - none
```

Overlap: `OTERYN-20260912-platform-native-evidence-hardening` (#1388, PR #1389 merged) lists `app/GameAuth/NativeEvidence/**` and `config/game-auth.php`; it is idle awaiting separate qualification authority and has no open PR. This change to the registry is additive and scoped to the fresh admission scope.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-30T03:00:00Z
head: f7af6b3
branch: claude/n4p-1-native-login-issuer
pr: 1421
status: validating
context_routes:
  - auth-identity
  - security
owned_paths:
  - app/GameAuth/NativeAdmission/**
  - app/GameAuth/NativeEvidence/NativeSigningTrustRegistry.php
  - config/game-auth.php
  - tests/Feature/GameAuth/NativeAdmission/**
proven:
  - NativeSigningTrustRegistry previously allowed re-keying a kid by key revision and had no trusted-key ceiling
  - libsodium detached Ed25519 signatures are deterministic, so stored signing input plus kid re-signs byte-identically
  - Review 5901672391 at 1ce1d614 found publishTrustedKey could re-trust or re-key a kid of an earlier profile version; the repair refuses any kid already used in an earlier version
  - Validation regexes anchored with a bare $ accepted a trailing newline; the repair anchors every regex in owned paths with \A and \z
  - Re-review of f7af6b3 found resign accepted a canonical payload with iat far in the future; repair round 2 refuses iat > now and requires exp - now >= 1 s on one millisecond clock
  - var_export and array casts of the keyring exposed the secret-key array; round 2 keeps the keys in a redacting holder reachable only through a closure (residual risk, Reflection on the closure, documented in the class doc)
derived:
  - Self-check cache is per issuer instance and bounded to 5 s, so a revocation stops signing within 5 s
unknown:
  - FND-04 section 5 exact string grammar; the issuer uses a conservative subset pending pinned Game verifier fixtures (contract section 16)
  - U4 TTL default, U9 production custody, U13 audit retention
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Reading trust through NativeEvidenceSource rejected; it persists observations and advances high-water, so a read-only registry lookup is used
  - Editing the shared CanonicalAccountId and NativeEvidenceContract::assertKeyId regexes rejected in this packet (outside owned paths); the issuer, keyring, context and registry apply local strict checks instead
changed_paths:
  - app/GameAuth/NativeAdmission/IssuedNativeAdmissionGrant.php
  - app/GameAuth/NativeAdmission/NativeAdmissionGrantContext.php
  - app/GameAuth/NativeAdmission/NativeAdmissionGrantIssuer.php
  - app/GameAuth/NativeAdmission/NativeAdmissionKeyring.php
  - app/GameAuth/NativeAdmission/NativeAdmissionSecretKeys.php
  - app/GameAuth/NativeAdmission/NativeAdmissionUnavailable.php
  - app/GameAuth/NativeEvidence/NativeSigningTrustRegistry.php
  - config/game-auth.php
  - tests/Feature/GameAuth/NativeAdmission/NativeAdmissionGrantIssuerTest.php
  - docs/agents/tasks/active/OTERYN-20260929-n4p-1-native-login-issuer.md
validation:
  - command: vendor/bin/phpunit, vendor/bin/phpstan analyse, vendor/bin/pint --test at 4d6edf6
    result: PASS
    evidence: 686 tests, 24 skipped, one pre-existing unrelated notice; phpstan and pint clean; superseded by the review repair
  - command: review repair round 1 (findings 1, 2, 4-7 of review 5901672391) at f7af6b3 and round 2 (re-review of f7af6b3; future iat, 1 s remaining, posix fail closed, secret holder); php -l on changed PHP files and a stubbed issuer/keyring harness
    result: PASS
    evidence: local harness 50/50 plus a posix_geteuid-disabled run that refuses to load the key; composer install blocked (dependency downloads from github.com refused by the session proxy), so PHPUnit, PHPStan and Pint run in exact-head CI
  - command: python tools/agents/checkpoint.py, documentation_ia.py, source_branch_closeout.py, git diff --check
    result: PASS
    evidence: local; PHP 8.4.19 locally while composer requires 8.5, exact-head CI is authoritative
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: no route or actor path in this slice; the issuer is reachable only from N4P-1b
blockers:
  - none
next_action: control plane freezes the round-2 PR #1421 head, runs exact-head CI and a fresh review; then N4P-1b (native login endpoint) in its own packet
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: pending
```
