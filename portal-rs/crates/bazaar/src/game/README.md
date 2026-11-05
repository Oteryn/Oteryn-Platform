# T58 G-7 preparation and qualification boundary

Authority: `docs/contracts/OTERYN_V2_BAZAAR_CONTRACT_PROPOSAL.md`, revision 2.
Read-only Game reference inspected at `857f15f376907c592bb28d854fccff18a15cdf87`:
`docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md`, especially §4.3,
§6.2 and §15. Laravel reference: `app/Marketplace/Native/BazaarOwnershipTransfers.php`.
Neither dependency task branch existed on origin at task start; the base already
contains the T57 public Accounts/Evidence interfaces and Character command API.

All nine proposed immutable command shapes are validated before local staging.
The caller supplies a transaction containing its authorized saga decision; commit
or roll back that transaction as a unit. Preparation and game flags default off.
Preparation can record an outbox while game dispatch stays off. No worker sends
these records, and no timeout, status projection or watermark authorizes money,
asset delivery, ownership changes, automatic release or a second attempt.

The six proposed routes return empty 503 responses with `Cache-Control: no-store`.
They are exported for future private-listener composition and are not mounted on
the public web listener. There is deliberately no Bazaar receipt endpoint. The
T57 Evidence implementation returns false until receipt qualification is possible.

Restore integration is a transaction-local `Outbox::reopen` seam for the existing
authenticated restore coordinator, not another restore endpoint. It raises floors
and reopens affected operations without rewriting their bytes or source sequence.
Projection ingestion shares that lock, orders unsigned revisions numerically,
retains same-revision conflict digests and never lets duplicate completeness
renew freshness. Raw projections remain internal, with no public serving API.
A watermark's receipt is not evidence of complete resync; all sale authority stays
closed. Quarantined results must be committed to retain their diagnostic digest.

## Unresolved contract decisions: deny until accepted

- The proposal explicitly defers the representation of Bazaar result data within
  the single CHAR_CMD receipt and supplies no shared receipt fixture. No facts
  member, alternate receipt schema, reason-code mapping or success is invented.
- The proposed contract-version allocation and Game capability-advertisement
  mechanism remain unaccepted. `dispatch_available` therefore stays false even
  if a composing application sets both local flags true.
- Lock-to-transfer interlock, escrow account allocation, destination custody,
  bank holds and incoming-credit headroom remain Game acceptance dependencies.
  No transfer is submitted through the existing Character API before a committed
  bind can be verified. Ownership cuts must use Character/projection revisions,
  epoch and recovery generation from the existing transfer receipt.
- The existing Bazaar core rounds gold commission down; this proposal requires
  ceil(gross * 250 / 10000). T58 validates the latter and never enables gold
  settlement through the former. Reconcile this public API before activation.
- Snapshot key provisioning/revocation and transfer-surviving optional tabs need
  qualification. Verification takes an explicitly supplied trusted public key,
  expected lock binding and exact kid. It accepts the minimal golden profile;
  non-null optional tabs and unknown blessing catalogs deny, without truncation.
- Attempts after the first require an accepted durable terminal refusal; staging
  cannot manufacture that evidence. Reopened rows stay paused for reconciliation.
- Production freshness, retention, resync completion and privacy-floor adapters
  must be wired and qualified before any publication or dispatch enablement.

No new SQL query macros were added; runtime parameterized queries preserve
`SQLX_OFFLINE=true` builds without new query-cache files.

## Validation limitation in the supplied base

Workspace formatting and Clippy with warnings denied pass. With the PostgreSQL
URL configured, the exact workspace test command reaches the existing
`admission::tests::db_gateway_client_calls_real_admission_listener` test and fails:
Rustls cannot choose its process-level CryptoProvider when both provider features
are enabled. The same failure reproduces on untouched base
`3b05567d53bb02080035f0e5ce518b22ef8d8119`. Selecting a provider in the gateway
client belongs outside T58's allowed files. This remains an acceptance blocker,
not an ignored success. The two T58 PostgreSQL tests exercise staged outbox
replay/refusal/conflict/restore and projection ordering/quarantine/watermark/epochs.

The diagnostic `cargo test --locked --no-fail-fast` run completed with 320 passed,
14 failed and 1 pre-existing ignored test. All 14 failures report that same
Rustls provider-selection panic in four admission/gateway test targets. All 14
Bazaar PostgreSQL tests passed, including both new T58 tests. No tests were
changed or disabled to work around this upstream acceptance failure.
