# ADR 0017: Platform-owned support and moderation boundary

- Status: Accepted
- Date: 2026-07-29

## Context

Oteryn needs authenticated tickets, bounded reports, moderator queues and account-visible enforcement history. Canary ban mutation and support file uploads do not have approved safe contracts.

## Decision

Oteryn Platform owns additive support-ticket, report, enforcement and notification-delivery records. User ownership is resolved from authenticated Identity state. Privileged mutations require confirmed MFA plus exact operation permissions, use deterministic locking/version checks and append bounded audit metadata.

Platform enforcement is informational/workflow state and does not mutate Canary bans or account status. Attachments remain disabled. Notification delivery occurs after the domain transaction and records failure without rolling back the committed lifecycle transition. Retention uses a supported configurable prune/anonymize command.

## Consequences

- support and moderation can ship without cross-repository writes;
- private user/moderator content has explicit presentation, audit and retention boundaries;
- Canary enforcement synchronization and attachments require separate ADRs/contracts and security review;
- repository/staging evidence cannot establish production deployment.
