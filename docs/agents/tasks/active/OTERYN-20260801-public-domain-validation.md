---
task_id: OTERYN-20260801-public-domain-validation
required_reads:
  - AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - docs/testing/PRODUCTION_SMOKE_CHECKLIST.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
  - docs/agents/tasks/archive/OTERYN-20260801-public-https-proxy.md
search_first:
  - public hostname APP_URL proxy cookie cache generated URL
  - Game Gateway routes health readiness version login session
  - open tasks and PRs owning audit evidence or Synology deployment paths
optional_reads:
  - PR #381
  - PR #383
  - PR #385
  - Issue #91
---

# OTERYN-20260801-public-domain-validation

## Goal

Produce one sanitized evidence package that classifies launch-relevant public-hostname, HTTPS, reverse-proxy, generated-URL, cookie, caching and Game Gateway behavior for `https://oteryn.molehill.cloud` and `https://login.oteryn.molehill.cloud` without changing application implementation, deployment or external infrastructure.

## Acceptance criteria

- [ ] Record the current repository head and observed deployment identity without unsupported inference.
- [ ] Verify or explicitly classify both canonical hostname roles.
- [ ] Collect bounded direct or durable evidence for representative WWW hostname behavior.
- [ ] Discover actual Gateway endpoint contracts and collect bounded evidence or an exact blocker.
- [ ] Classify generated URLs, cookie attributes, sensitive-response caching and proxy HTTPS behavior.
- [ ] Prove password-recovery hostname/delivery or record the exact blocker.
- [ ] Record every finding with severity, confidence, evidence, impact, ownership and recommendation.
- [ ] Persist a compact final report and one draft evidence PR.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-validation.md
  - docs/agents/reports/OTERYN-20260801-public-domain-validation.md
modules:
  - public-web
  - identity
  - game-gateway
  - edge-transport
dependencies:
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - PR #381 route and surface inventory
  - PR #383 and PR #385 public HTTPS proxy evidence
  - Issue #91 production go-live gate
blockers:
  - direct external probes are unavailable from the current execution environment until another evidence path succeeds
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T07:19:32Z
session_id: chatgpt-20260801-public-domain-validator-001
policy_version: 2
phase: discovery_and_evidence
execution_mode: chat-github-connector
execution_reason: repository inspection, bounded documentation writes and evidence review do not require application implementation
context_pressure: medium
decomposition_decision: phased
head: 7dac56d3f3f4606be958c875f278edbe410e6b54
branch: audit/OTERYN-20260801-public-domain-validation
pr: none
status: investigating
context_routes:
  - agent-governance
  - security
  - auth-identity
  - api
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-validation.md
  - docs/agents/reports/OTERYN-20260801-public-domain-validation.md
proven:
  - Current default branch main is 7dac56d3f3f4606be958c875f278edbe410e6b54.
  - The task record, dedicated branch and PR did not exist before this session.
  - The canonical repository contract maps oteryn.molehill.cloud to Platform WWW on Synology loopback port 8000 and login.oteryn.molehill.cloud to Game Gateway on loopback port 8080.
  - PR #381 owns separate portal audit task, report and evidence paths and does not own this task's two documentation paths.
  - PR #335 owns a distinct Synology boot-repair path; this task performs no deployment-file edits.
  - Direct HTTP probes through web fetch failed and the sandbox resolver could not resolve either canonical hostname.
derived:
  - No current repository path ownership conflict prevents this documentation-only validation task.
unknown:
  - Exact currently deployed Platform SHA.
  - Exact currently deployed Gateway and Canary versions.
  - Current external DNS, TLS, redirect, header, cookie and route behavior for both hostnames.
  - Availability of a controlled identity and mailbox for login and password-recovery validation.
conflicts: []
first_failure:
  marker: external-probe-unavailable
  evidence: web fetch returned cache-miss failures and sandbox curl returned DNS resolution failures for both canonical hostnames
rejected_hypotheses:
  - Prompt-authoring deployment context is not accepted as current deployment identity without direct evidence.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-validation.md
validation:
  - command: GitHub repository, branch, task and PR preflight
    result: PASS
    evidence: main 7dac56d3f3f4606be958c875f278edbe410e6b54; branch/task/PR initially absent; open PR ownership inspected
  - command: direct HTTPS and HTTP fetch attempts for both canonical hostnames
    result: BLOCKED
    evidence: web fetch cache miss and sandbox DNS resolution failure
blockers:
  - Current tools cannot directly reach or resolve the two canonical public hostnames; live external behavior remains unproven unless existing exact-deployment artifacts provide equivalent evidence.
next_action: Discover effective Platform and Gateway hostname-dependent contracts and exact deployment evidence from repository source, PRs and workflow artifacts.
```

## Notes

Implementation, deployment, production mutation and external infrastructure changes are not authorized. Large logs, screenshots, traces and binaries remain outside Git.
