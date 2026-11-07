# Admin

Staff authentication, confirmed MFA, current RBAC grants and durable audit protect
admin surfaces. See [admin analytics](src/analytics/README.md) for T10b transport,
storage, configuration and qualification.

Admin analytics ingestion and presentation default OFF. Enabling presentation
cannot bypass the accepted resync baseline gate: contract §§5.3 and 10.6 require
the Game-proposed manifest/control envelope and qualified coverage/absence proof.
Pages, commits and fresh watermarks alone never establish a baseline. Until that
separately accepted verifier exists and has accepted a baseline, all analytics
pages and JSON remain unavailable. Export has no route (§10.4).
