---
task_id: OTERYN-20260805-single-native-protocol-version
coordination_id: OTS-20260804-native-protocol-selection
status: completed
agent: "native protocol contract correction owner"
project_lane: native-gameplay-protocol
track: cross-repository-contract
created: 2026-08-05T08:45:00+02:00
completed: 2026-08-05T09:00:00+02:00
archived: 2026-08-05T09:00:00+02:00
product_pr: 527
product_head: "f333722f6c8c1feacb85534d10220d29d2f8e5f1"
product_merge: "c33ea790efe331219ab757a16613a6b6f1a3e265"
risk: medium
owned_paths: []
shared_path_lease: []
feature_scope:
  type: documentation
  user_facing: false
  backend_required: false
  frontend_required: false
  integration_required: false
  e2e_required: false
execution_mode: github-only
---

# Terminal result

The native Oteryn gameplay programme now has an accepted single-version rule:

```text
family: oteryn
native_protocol_version: 1
transport: tcp.tls13.protobuf.be32.v1
schema_revision/hash: exact canonical contract
capabilities: exact canonical list/digest
```

Native v1 has no current profile field, catalogue, table, enum, selector, ordering, fallback or user-facing chooser.

The existing disabled Platform/Game Gateway producer still contains transitional profile-oriented implementation and MUST remain disabled until the saved cross-repository completion prompt corrects it.

Future native variants remain possible only through a new reviewed ADR, contract/schema revision and cross-repository migration after a real incompatibility is proven. No unused placeholder profile field remains part of the target v1 design.

Canary compatibility profiles remain unchanged and isolated from the native protocol.

# Delivered paths

- `docs/architecture/adr/0011-single-native-protocol-version.md`;
- `docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_SINGLE_VERSION_AMENDMENT.md`;
- `docs/agents/prompts/OTS_NATIVE_PROTOCOL_SINGLE_VERSION_COMPLETION_AGENT.md`.

# Saved implementation programme

The completion prompt authorizes coordinated work in:

- `blakinio/Oteryn-Platform`;
- `blakinio/Otheryn`;
- `blakinio/otclient`.

It requires:

1. canonical contract/schema and correspondence correction;
2. safe removal of the native profile dimension from the disabled Platform/Gateway producer;
3. Otheryn Game Session v2, readiness, TLS/ASIO native producer and authoritative gameplay;
4. independent Rust `protocol-oteryn` using the merged Tokio transport;
5. exact bounded staging E2E, Canary regression, downgrade-negative tests and rollback rehearsal;
6. production remaining disabled unless separately authorized.

# Validation

Exact product head `f333722f6c8c1feacb85534d10220d29d2f8e5f1`:

- Agent Governance `30982801081`: PASS;
- CI `30982801065`: PASS;
- Game Auth Ticket Concurrency `30982801070`: PASS;
- Platform DB Outage Validation `30982801064`: PASS;
- Edge Security Emulation `30982801071`: PASS;
- Phase 7 Production-Like Validation `30982801067`: PASS;
- exact changed paths: four declared documentation/task/prompt files;
- independent contract/prompt consistency review: PASS;
- material findings: 0;
- unresolved review threads: 0;
- protected squash merge: `c33ea790efe331219ab757a16613a6b6f1a3e265`.

# E2E

`NOT_APPLICABLE` — this task delivered the binding architecture correction and reusable implementation prompt only. It did not change runtime, persistence, schema bytes, deployment or production enablement.

# Ownership release

All task ownership and shared-path leases are released.
