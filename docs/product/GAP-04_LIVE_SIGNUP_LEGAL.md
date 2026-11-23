# GAP-04 · Live signup legal editions

**Decision pending.** The operator must supply and approve the actual terms of
service and privacy policy in English and Polish, including the legal operator
details and each document's effective date. The tracked draft package is for
local and testing signup. Its text cannot qualify a live release.

## Publish and qualify

1. Obtain the operator's approval for both translations of each document.
   Choose a new immutable edition version for the terms and a new immutable
   edition version for the privacy policy. Use the same version for English
   and Polish of each document.
2. In the staff panel, open **Pages**. The live signup readiness section links
   to all four editors and shows the configured version, current version and
   publication status. Publish English first for each document, then review
   and publish the Polish translation against that English revision. Select
   **Approve this new legal edition as final** on each publish. Use the
   [existing host CLI](../../portal-rs/crates/content/src/pages/README.md)
   if publication is performed from the host.
3. Set `OTERYN_TERMS_VERSION` and `OTERYN_PRIVACY_VERSION` in the candidate
   live configuration to the exact approved versions. Run
   `oteryn-portal check-signup-legal` with that configuration and the target
   database. Activate signup only after the check passes. Staff can verify
   all four statuses in **Pages** after the service loads the configuration.
4. Open `/en/register` and `/pl/register`. Confirm each consent link opens the
   approved public edition in the same language and displays the configured
   version. In the qualification environment submit a fresh registration in
   each locale and verify that the resulting consent records reference the
   immutable terms and privacy editions with the accepted locale and version.
   A stale submitted version must return a validation error.

Live signup pauses in both languages if any required edition is unpublished,
out of date, inconsistent with the configured version, or lacks final approval.
The pause page links to the public policies. Revising English requires a fresh
review of the Polish translation. Version changes require a new check before
activation. The readiness section applies the live publication rules even when
a disposable profile permits testing with drafts.

This decision and the release qualification belong to the Platform operator.
There is no Game-side deliverable or data dependency.
