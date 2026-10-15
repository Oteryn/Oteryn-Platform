---
task_id: OTERYN-20261015-oracle-actions-runners
governing_issue: 1639
required_reads: []
search_first: []
optional_reads: []
---

# Oracle Actions runners

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1639.
The owner explicitly authorized moving both organization runners and deployment
workflows to Oracle, preserving direct oteryn.com and all migrated authority.
One writer: this Codex task, branch codex/fix-oracle-build-source-modes-20261015.
Game deployment coordination: Oteryn/Oteryn-Game#2217; no protocol changes.

## Acceptance criteria

- Runners stay in their repository-restricted groups with existing component labels.
- No runner receives a Docker socket, host secrets or general host shell.
- A root-owned component-bound forced command accepts only status or deployment
  of exact current protected main after its GitHub Actions aggregate gate passes.
- Preserve private environment, migrated storage, custody, content and the durable
  Game incarnation controller. Back up before migrations; fail closed on failure.
- Legacy NAS/Tunnel workflows cannot route jobs or overwrite Oracle configuration.
- Real protected workflow executions prove runner identity, ARM64 and readiness.
- Game daily physical and logical backups are verified and persist across reboot.

## Ownership

Owned: deploy/oracle/**, .github/workflows/oracle-operations.yml and this packet.
Dependencies: existing installed durable controller and coherent migration.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-15T22:00:00Z
head: 156c9f73ecef3cf34cd908562508dc5cceb79cce
branch: codex/fix-oracle-build-source-modes-20261015
pr: null
status: waiting
terminal_pr_policy: archive_pending
context_routes:
  - deployment
owned_paths:
  - deploy/oracle/**
  - docs/agents/tasks/archive/OTERYN-20261015-*
  - .github/workflows/stage-synology-deploy-runner-image.yml
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - .github/workflows/oracle-operations.yml
  - .github/workflows/ci.yml
  - docs/agents/tasks/active/OTERYN-20261015-oracle-actions-runners.md
proven:
  - New component runners registered offline in existing repository-restricted groups
  - Native Account repair merged via real Merge Queue and browser roster passed
derived: []
unknown:
  - Protected workflow deployment acceptance
  - Final native gameplay entry
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths: []
validation:
  - command: python3 deploy/oracle/test_actions_deploy.py
    result: PASS
    evidence: ten focused authority and recovery tests
  - command: python3 deploy/oracle/test_game_supervisor.py
    result: PASS
    evidence: six retained custody and reconciliation tests
blockers: []
next_action: archive delivered source packet after protected Oracle workflow and native entry acceptance
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository delivery
source_branch_evidence: verify after protected integration
```

The component key additionally requires its exact active protected-main workflow
run/attempt, deployment title/source SHA and expected in-progress runner job. This
preserves the existing environment-release gate; a status run cannot authorize
deployment. The owner approved one required independent Platform Codex review
for PR #1644 in this chat. Game review uses its standing repository grant.

CI repair: explicitly register Oracle Operations and retire the obsolete NAS runner-image
stager within the existing 54-workflow budget; no budget or gate is weakened.

Review finding 4288835355 accepted: add the negative Docker capability probe
before SSH, covering custom socket/context and DOCKER_HOST endpoints as well as
the canonical socket. Apply the same guard to the sibling Platform workflow.

Independent review findings 4288848796/8802/8812/8819/8821 accepted and repaired:
prove current branch protection, reauthorize after backups, persist durable Oracle
schema/release journals, refuse migration with a running old Game, and include
scheduler health plus expected image/incarnation checks. Eight focused authority
and recovery tests plus the actual workflow endpoint-guard regression pass.

Second review findings 4288929731 (edge configuration deployment) and 4288929744
(receipt drift) accepted and repaired. The candidate edge config is validated
before disruption, atomically installed, recreated to acquire the bind-file inode
when needed, and read back from the serving container. Receipt reuse now requires
fresh workflow/main authority and actual image/config equality. Ten focused
authority/recovery tests pass.

The existing CI workflow runs all three Oracle authority, supervisor and runner
boundary suites. The open CI proposal #1610 owns documentation only; this bounded
validation step does not overlap its files or change routing or required gates.

## Main-only operational qualification

PR1644 delivered source at b288effb76480ce58f55c834010814ddb0b961a6. Real merge_group
38821354548 and platform-gate116720563780 passed; protected-main readback matched.
The reviewed root adapter and component-constrained keys are installed on Oracle;
continuous runners remain stopped. Legacy NAS/Tunnel workflows are disabled.
Actual status dispatch exposed GitHub parser rejection of runner.name at job-env
scope (HTTP422, line35). Move this binding to the operation step env, where the
runner context is defined. No authority, environment protection or host command
changes. Resume the same operational task under Issue1639; preserve independent
review and exact qualification evidence from PR1644. Native server repair is
separately coordinated under Game2219.

The actual GitHub dispatch parser and actionlint1.7.7 reproduce the job-env
runner-context error on b288eff. Actionlint passes for both corrected sibling
workflows; the existing endpoint guard and ten adapter regressions also pass.

## Physical deployment incident and bounded repair

Platform1646 merged at156c9f73ecef3cf34cd908562508dc5cceb79cce through real
merge_group38823837786; the GitHub Oracle status run38824509629 passed on the
expected Oracle runner. Game2222 merged atb021969169d9cd695d5cfdf137b2ab228b9aaa42
through real merge_group38824307201. Both legacy workflow families are disabled.

The first deployment attempt stopped before effects because host Git was missing;
Git2.53.0 was installed. The next attempt38824952250 built its images and stopped
writers, then PHP refused to open artisan before loading the application. The
root operator umask077 had made checked-out public executable inputs0700 and
ordinary inputs0600; Docker COPY retained root-only permissions for www-data.

Operator recovery independently proved candidate artisan mode0700 and unreadable
under its actual image USER, the exact failure before framework entry, and all69
live Platform migration rows equal to the verified predeployment logical backup
20261015T234051Z. The retained previous schema contract matches the candidate.
The exact prior environment and original Oracle containers were restored. Public
login/gateway returned200, scheduler healthy, Game durable readiness passed.
Failed schema_unknown evidence is retained as a reconciled historical record.
Current release state explicitly has phase operator_reconciled and sha:null,
with independently verified schema and actual prior image IDs; it is not a source
qualification or success receipt for the failed candidate. No NAS writer restarted.

Repair: apply umask022 only to child Git clone/fetch/checkout processes for public
tracked build inputs, preserving the parent/private writer umask077. Add an
isolated image probe inheriting real USER/WORKDIR, without network/mounts/keys,
that requires readable artisan/bootstrap/autoloader before any backup or outage.
Real Git checkout RED reproduces0700; the preflight RED proves the old adapter
would disrupt first. Both GREEN plus all12 authority/recovery,6 controller and1
runner boundary tests pass. The PHP direct-source consumer was affected; compiled
Game/gateway binaries are built as root and retain normal build output modes.
No permission normalization touches runtime configuration, keys, custody or dumps.

Physical rollout remains open. This material host-execution repair requires fresh
exact-candidate CI and independent review before manually upgrading the root
adapter or attempting another Platform deployment.
