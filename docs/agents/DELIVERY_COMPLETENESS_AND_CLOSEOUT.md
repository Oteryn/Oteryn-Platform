# Platform Delivery and Closeout Delta

Use the bound META organization policy for shared delivery, review and integration semantics. Platform adds these local requirements:

- verify the claimed Platform producer/consumer path and preserve root security, data, payment and compatibility invariants;
- inspect the complete candidate diff and run focused checks plus the required exact-head `platform-gate`;
- treat runtime E2E as `NOT_APPLICABLE` only with a concrete reason for a non-runtime change;
- keep related PRs intentional and resolve material findings;
- after verified integration, archive the task record, close the governing Issue when acceptance is complete and release ownership.

A PR, static validator or worker report alone does not prove delivery. Production and protected external operations remain separately authorized.
