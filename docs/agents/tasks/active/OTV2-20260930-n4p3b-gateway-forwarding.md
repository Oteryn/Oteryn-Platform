---
task_id: OTV2-20260930-n4p3b-gateway-forwarding
governing_issue: 1419
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
search_first:
  - services/game-gateway/internal/httpapi
  - app/GameAuth/NativeLogin
optional_reads: []
---

# OTV2-20260930-n4p3b-gateway-forwarding

## Goal

Governing GitHub Issue: #1419 (https://github.com/Oteryn/Oteryn-Platform/issues/1419), delivery item 3 (Gateway). Coordination: Oteryn/Oteryn-Game#162 (lane N4-P). Owner authority: D160 (Platform writes for N4-P, testing/preproduction only), D164, D171, D172.

N4P-3b, split from N4P-3 (#1427, merged): the Go Game Gateway (`services/game-gateway`) validates, rate-limits and forwards `POST /v1/login` with `protocol_version: 2` once to the private issuer `POST /internal/v1/game-auth/native-admissions` (contract §3.1, §3.2, §3.3, §6.2 step 5, §10, §11), default off. The Canary path is unchanged and the Gateway gets no database or signing credential.

Out of scope: production config, secrets and enablement; issuer PHP changes; `.github/workflows`; Game repository writes; the joint E2E with Game (#1419 item 5).

## Acceptance criteria

- [x] `GATEWAY_NATIVE_LOGIN_ENABLED` (strict `true`/`false`, default off); off keeps the previous `400 invalid_request` answer for protocol 2; protocol 1 keeps the Canary path either way.
- [x] Exact §3.1 validation with the issuer's rules; canonical re-encoded body forwarded once with the Gateway service credential; no redirects; bounded timeout (`GATEWAY_NATIVE_ADMISSION_TIMEOUT`, default 5 s, at most 8 s).
- [x] Exact §3.2 success validation against the forwarded request; §11.1 error body with the canonical `attempt_ref` echo; §11.2 status/class mapping; SECURITY_TERMINAL collapse; Retry-After on rate limits.
- [x] §6.2 step 5: unsent request is NATIVE_LOGIN_UNAVAILABLE; written request with no valid answer is ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED.
- [x] §10 Gateway limits: 30/min per source address; 6 in total per (`attempt_ref`, ticket hash); bounded table fails closed.
- [x] No ticket, grant or credential in logs or `%v` output. Go unit tests plus a Gateway-level journey test against an issuer double.

## Ownership

```yaml
owned_paths:
  - services/game-gateway/**
  - docs/agents/tasks/active/OTV2-20260930-n4p3b-gateway-forwarding.md
modules:
  - services/game-gateway
dependencies:
  - "#1427 N4P-3 native admission issuer (merged f880cd7)"
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-30T23:59:00Z
head: UNKNOWN
branch: claude/n4p3b-gateway-forwarding
pr: none
status: validating
context_routes:
  - auth-identity
  - security
owned_paths:
  - services/game-gateway/**
proven:
  - "Issuer wire at 5d4883a: NativeAdmissionWire/NativeAdmissionRequest rules (ticket [\\x21-\\x7e]{1,256}, client_build 1..64, alpn 1..255 visible ASCII, major/profile 0..65535 ints, platform windows, 1..4 transports) are mirrored by the Gateway"
  - "The issuer echoes attempt_ref from the raw body and collapses SECURITY_TERMINAL codes; the Gateway forwards a canonical body so the echo is the forwarded attempt_ref"
derived:
  - "A transport failure after the request was written can follow an issuer commit, so it maps to ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED (the same request returns the committed grant)"
unknown:
  - U14 Gateway to issuer mTLS; U10 rate-limit tuning; trusted-proxy source address when the Gateway sits behind a proxy
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Forwarding the raw client bytes rejected; Go and PHP JSON parsers differ on edge cases (invalid UTF-8), a canonical re-encode makes both sides parse identical values
changed_paths:
  - services/game-gateway/internal/nativelogin/ (new)
  - services/game-gateway/internal/httpapi/server.go
  - services/game-gateway/internal/httpapi/server_test.go
  - services/game-gateway/internal/config/config.go
  - services/game-gateway/internal/config/config_test.go
  - services/game-gateway/cmd/game-gateway/main.go
  - services/game-gateway/internal/e2e/native_login_test.go
  - services/game-gateway/README.md
  - docs/agents/tasks/active/OTV2-20260930-n4p3b-gateway-forwarding.md
validation:
  - command: gofmt -l . && go vet ./... && go test -race -count=1 ./... && go build -trimpath ./cmd/game-gateway (services/game-gateway, Go 1.24.7)
    result: PASS
    evidence: local
  - command: joint issuer<->gateway test against the real Laravel issuer
    result: NOT_APPLICABLE
    evidence: the repository has no harness that runs the PHP issuer and the Go Gateway together; the Go journey test uses an issuer double with the issuer's wire; the real joint E2E is #1419 item 5
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: every native switch is default-off and enablement is testing/preproduction only under separate authority; joint E2E with Game is #1419 item 5
blockers:
  - none
next_action: exact-head CI green; hand back to the control plane (Oteryn/Oteryn-Game#162)
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: pending
```

## Notes

The public error body uses the existing Gateway no-store header set (`no-store, no-cache, must-revalidate, private`), a superset of the contract's `no-store, private`. A query string or a body above the 16 KiB public read limit keeps the legacy `invalid_request` answer because the branch is chosen from the parsed body.
