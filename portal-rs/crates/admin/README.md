# Admin

Staff authentication, confirmed MFA, current RBAC grants and durable audit protect
admin surfaces. See [admin analytics](src/analytics/README.md) for T10b transport,
storage, configuration and qualification.

Admin analytics ingestion and presentation default OFF. Enabling presentation
cannot bypass the accepted resync baseline gate: contract §§5.3 and 10.6 require
the Game-proposed manifest/control envelope and qualified coverage/absence proof.
No fence row is created until the resync envelope/baseline is accepted; missing
evidence remains fail-closed. Pages, commits and fresh watermarks alone never establish a baseline. Until that
separately accepted verifier exists and has accepted a baseline, all analytics
pages and JSON remain unavailable. Export has no route (§10.4).

## Platform audit evidence

Staff with a current MFA-backed session, `admin.access` and `audit.view` can
download a verified completed UTC day from **Audit evidence** in the staff panel.
The portal accepts the last 90 completed days, with a maximum of 25,000 entries
per day. The export window does not delete source audit records or seals.
Existing audit rows are captured by the migration using their UTC occurrence
dates. New events use their UTC ingestion day by default. Explicitly dated events
can join their original day until it is being sealed, and join the current open
day if they arrive later. The original occurrence timestamp remains in the export.

The first successful export seals the day. Subsequent exports verify against that
same seal. The portal logs the day, version, count and head under
`oteryn_admin::audit_anchor` after every successful export. Keep
`oteryn_admin::audit_anchor=info` enabled in `RUST_LOG`. For independent retention,
export each completed day and preserve its JSON and matching anchor in an
access-controlled archive and log destination outside the audit database.
Limit access to the archive because exports contain staff and request metadata.
The export exposes digests of the before and after values. Existing records with
no account actor are identified as system events.

To verify a preserved export, find its matching version and UTC date in the
independently retained anchor log. For `oteryn-audit-evidence-v1`, check that the
entry count matches the anchor. Verify that sequences start at 1 and increment by
1, and that record IDs are strictly increasing. The first previous hash is
the lower-case hex SHA-256 of the UTF-8 bytes of the version, a newline, and the
ISO UTC date. At each entry, hash the previous lower-case hex hash, a newline,
and the RFC 8785 canonical JSON of the `record` object. Check the previous hash
and computed hash against the entry and continue with that computed hash.
Compare the final hash with both the export head and the retained anchor.
For an empty day, compare the initial hash with both heads.
