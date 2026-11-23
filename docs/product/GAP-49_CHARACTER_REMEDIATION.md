# Staff character remediation qualification

Staff can open **Character remediation** in the Players console to review the five proposed corrections and their requirements. The view is available through the existing staff host, session, MFA and Players access checks, with `OTERYN_ADMIN_CONSOLE=true`. The console flag defaults to off.

The qualification view reads Platform policy metadata. It reports unavailable for the live target and revisions, Game previews and approvals where applicable, and Game receipts. The operation links select a policy profile. Every correction still awaits contract acceptance.

| Proposed operation | Risk | Game authority and required evidence |
| --- | --- | --- |
| `character.rescue.teleport` | R3 | Character Placement and the world owner verify location revision, stuck evidence, the allowlisted destination and policy revision, same-world/channel scope, live session generation and movement restrictions. |
| `character.session.disconnect` | R3 | Live session owner verifies channel, session ID, generation and revision, then terminates only that generation and invalidates its reconnect proofs under one fence. |
| `character.outfit.correct` | R2 | Character Appearance verifies content, appearance and entitlement revisions and eligibility for owned cosmetics. It commits a recoverable prior appearance snapshot with the correction. |
| `character.item.compensate` | R3 | Inventory verifies definition and inventory revisions, provenance, correction line, canonical loss revision, capacity and remaining loss allowance. |
| `character.item.confiscate` | R4 | Item Custody verifies exact instance, location and custody and inventory revisions, escrow state and a digest-bound approved preview. |

## Required before execution

- Game and Platform owners must accept the versioned operation, typed inputs, dry-run and result variants, refusal codes and authenticated receipt profile in the proposed contract. Game must supply the owning route and current eligible target, including the relevant revisions, and implement the fences in the table. [Proposed Game operations §§4–7](../contracts/PANEL_PROPOSED_GAME_OPERATIONS.md)
- Platform must bind each accepted profile to an explicit operation grant and current world scope, require a reason code and case reference, enforce atomic admission limits, and retain the requester and approver evidence. R3 and R4 require recent requester step-up and a dry-run path. R4 execution requires an independently scoped approver with recent step-up and approval bound to the exact preview and revisions.
- Game must recheck current authorization and reject a wrong world, stale revision or ineligible target at its effect boundary. Commit the effect, idempotency evidence and typed receipt together. Platform must verify the receipt signature and exact delivery/intent binding, persist the legal transition and audit before showing a Game outcome. Resolve uncertain outcomes by original operation ID.
- Keep delivery and execution disabled by default when an accepted profile is introduced. The qualification page provides no submission, outbox, receipt ingress or financial path.
