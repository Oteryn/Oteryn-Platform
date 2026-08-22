# Organization Terminal Branch Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a reusable, exact-head terminal branch lifecycle workflow from Platform and adopt it safely in META, Game and Atlas with repository-local authority.

**Architecture:** Platform owns the tested lifecycle implementation and reusable workflow. Each caller pins the reusable workflow to an immutable merged Platform SHA and supplies only its local policy/ADR and event wrapper, so write tokens remain repository-local.

**Tech Stack:** GitHub Actions, Python 3.12, GitHub REST/git exact-head deletion controls.

**Spec:** `docs/superpowers/specs/2026-08-23-organization-terminal-branch-lifecycle-design.md`

## Global Constraints

- Never use an organization-wide destructive token.
- Destructive calls operate only on `github.repository` with that repository's `GITHUB_TOKEN`.
- `pull_request_target` write execution checks out trusted `main`, never PR code.
- Deletion is exact-head and fail-closed; no age/prefix heuristic deletion.
- Callers pin Platform workflow/tool source to an immutable merged SHA.

---

### Task 1: Platform reusable workflow contract

**Files:**
- Create: `.github/workflows/terminal-branch-lifecycle-reusable.yml`
- Create: `docs/contracts/ORGANIZATION_TERMINAL_BRANCH_LIFECYCLE.md`
- Create: `tools/agents/test_terminal_branch_reusable.py`
- Modify: `.github/workflows/terminal-branch-lifecycle.yml`

**Interfaces:**
- Consumes: existing `tools/agents/branch_lifecycle.py`, `terminal_branch_cleanup.py`, and `terminal_branch_approval.py`.
- Produces: reusable `workflow_call` operations `read`, `close`, and `apply` with inputs `platform_ref`, `policy_path`, and `approval_path`.

- [ ] **Step 1: Write the failing reusable-workflow contract test**

Create a stdlib `unittest` that first requires the reusable workflow file to exist, then asserts operation dispatch, immutable Platform checkout, caller-root execution, trusted-main close/apply checkout, and permission separation.

- [ ] **Step 2: Run test to verify RED**

Run: `python3 tools/agents/test_terminal_branch_reusable.py`
Expected: FAIL because `.github/workflows/terminal-branch-lifecycle-reusable.yml` is absent.

- [ ] **Step 3: Implement the reusable workflow and durable caller contract**

Use pinned `actions/checkout` / `actions/setup-python` / `actions/upload-artifact`, checkout Platform tooling into `.oteryn-branch-lifecycle` with `persist-credentials: false`, and pass `--root "$GITHUB_WORKSPACE"` plus the caller-local policy path to the existing lifecycle scripts.

- [ ] **Step 4: Run focused tests**

Run:
`python3 tools/agents/test_terminal_branch_reusable.py && python3 tools/agents/test_terminal_branch_cleanup.py && python3 tools/agents/test_terminal_branch_guarded.py && python3 tools/agents/test_terminal_branch_approval.py`
Expected: PASS.

- [ ] **Step 5: Commit and open draft PR**

Commit message: `ci(branches): add reusable terminal lifecycle workflow`.

### Task 2: Merge Platform implementation

**Files:**
- Modify task checkpoint/archive only as required by repository closeout.

**Interfaces:**
- Consumes: exact Platform PR head from Task 1.
- Produces: immutable merged Platform SHA used by all caller repositories.

- [ ] **Step 1: Run exact-head self-review and required CI**

Inspect changed paths/full diff; require current Platform `classify-changes` and `test` gates plus the terminal lifecycle validation workflow.

- [ ] **Step 2: Squash merge**

Merge only after the exact head is green and review/thread hygiene is clear.

### Task 3: Adopt in META, Game and Atlas

**Files per caller:**
- Create: `.github/workflows/terminal-branch-lifecycle.yml`
- Create: `docs/agents/BRANCH_LIFECYCLE_POLICY.json`
- Create: `docs/architecture/adr/<repo-specific>-terminal-branch-lifecycle.md`

**Interfaces:**
- Consumes: Platform merged SHA from Task 2.
- Produces: repository-local scheduled/read, close-event, and reviewed-apply branch lifecycle controls.

- [ ] **Step 1: Create caller issue/branch and exact local policy/ADR**

Policy retains protected `main` and references the local ADR.

- [ ] **Step 2: Add thin caller workflow pinned twice to the same Platform SHA**

`uses:` and `with.platform_ref` must both equal the exact merged Platform SHA. Read calls get read permissions; close/apply calls get caller-local `contents: write` only.

- [ ] **Step 3: Validate caller adoption**

Run/inspect the caller's normal required CI and the lifecycle read job. Verify no runtime/deployment paths changed.

- [ ] **Step 4: Squash merge each caller PR**

Merge independently when exact-head gates pass.

### Task 4: Post-merge read-only inventory and closeout

**Files:**
- Archive Platform task record after provider rollout is proven.

**Interfaces:**
- Consumes: merged caller workflows.
- Produces: organization branch-hygiene inventory without automatic heuristic deletion.

- [ ] **Step 1: Verify scheduled/manual inventory capability in each caller**

Confirm the workflow exists on merged `main` and uses the expected immutable Platform SHA.

- [ ] **Step 2: Record historical orphan cleanup as a separate reviewed action**

Do not delete ambiguous old branches as part of rollout. Generate reviewable manifests first; only exact reviewed candidates may be applied later.

- [ ] **Step 3: Close issues and verify source branch disposition**

Close provider issues, archive the Platform task record, and verify merged task branches are removed by repository branch lifecycle/merge settings.
