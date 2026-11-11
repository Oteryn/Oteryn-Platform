#!/usr/bin/env bash
# Cutover hook OTERYN_CUTOVER_PROVENANCE_INIT_COMMAND (plan P6 step 4, design PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md D5
# step4-begin). Called by cutover-postgres.sh native-reenable, after its forward-only marker, as:
#
#   provenance-init.sh init <task_id> <close_run_id>
#
# With native admission still off it:
#   1. resolves the runtime environment, checks the evidence location and reads the current kid K and its
#      used.kids count (refuses before any write unless the count is 1);
#   2. fetches every anchor of the locked anchor issue, retries the anchor of any earlier run whose last line
#      was never anchored (D3), proves it can post there and that verify --anchors passes, all before any
#      new evidence write;
#   3. opens a run (run-open with a pre-init observation), writes init-intent, anchors it and writes anchored;
#   4. runs `game-auth:native-evidence:init-provenance` (D1) in the platform service, observes (D2) and writes
#      the init record (a refused init writes begin-refused);
#   5. observes with --key-id=K and writes attempt-intent 1, anchors it and writes anchored.
# Every record except an anchored line is anchored before the next action. Any failure exits non-zero, so the
# cutover step fails and native admission stays off; the deploy that turns it on is the operator's next step.
#
# The deploy workflow runs `provenance-init.sh preflight <task_id> <close_run_id>` before native-reenable
# records its forward-only marker: steps 1 and 2 (location, kid, anchors with the D3 retry, verify --anchors,
# a write probe) without any evidence write, so a missing anchor issue or permission cannot strand the cutover.
#
# The recovery commands run through the deploy workflow's cutover evidence step, never by hand, because only
# the workflow token posts anchors as the trusted author:
#
#   provenance-init.sh close-init <task_id> <close_run_id> <run_id>   D5 step4-begin --close-init --run <id>
#       For a run without an unfinished init-intent it resumes a begin the runner left between two durable
#       lines (run-open only, an init without its attempt intent, an intent without its anchored line); see
#       resume_begin. Native admission must still be off for a resume that writes an intent.
#   provenance-init.sh observe <task_id> <close_run_id> <run_id>      D5 step4-observe --run <id> (late: true)
#   provenance-init.sh retry <task_id> <close_run_id> <run_id>        D5 step4-begin --retry: attempt-intent n+1
#       after a failed-before-witness observation, the last line of the run.
#   provenance-init.sh rebegin <task_id> <close_run_id> [<run_id>]    the begin of `init` again, in a new run, after
#       the newest run of this cutover ended in a non-permanent begin-refused or in init-late (native admission off).
#       Without a run id it is the begin of a cutover whose `init` failed before any run of it existed (a check,
#       the pre-init observation, the probe or open itself failed after the forward-only marker); it refuses, and
#       names the newest run, when a run of this cutover is in the evidence directory.
#
# Every failure of `init` from its first check on prints the next step (HOOK_FAIL_HINT): rebegin without a run
# id before run-open exists, then the run id the recovery steps take.
#
# All act only on a run whose run-open records cutover_ref "<task_id>/close-<close_run_id>", and only while the
# cutover state (OTERYN_CUTOVER_STATE_DIR, default $OTERYN_STATE_DIR/postgres-cutover) records native re-enable
# for that task and close run; otherwise they refuse before any anchor post or evidence write.
#
# Inputs: the cutover step environment plus OTERYN_CUTOVER_ANCHOR_ISSUE (the locked anchor issue number),
# GITHUB_REPOSITORY and GH_TOKEN (issues: write). Output: key=value result lines only; never the environment
# file, a key, a token or an observation.
set -euo pipefail
umask 077

HOOK_NAME=provenance-init
HOOK_SELF="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=deploy/synology/scripts/cutover-hooks/common.sh
source "$HOOK_SELF/common.sh"
# shellcheck source=deploy/synology/scripts/cutover-hooks/evidence.sh
source "$HOOK_SELF/evidence.sh"

usage() {
    echo "Usage: provenance-init.sh {init|preflight} <task_id> <close_run_id>" >&2
    echo "       provenance-init.sh {close-init|observe|retry|rebegin} <task_id> <close_run_id> <run_id>" >&2
    echo "       provenance-init.sh rebegin <task_id> <close_run_id>" >&2
    exit 1
}

(($# >= 3)) || usage
command="$1"
task_id="$2"
close_run_id="$3"
run_id=""
case "$command" in
    init | preflight) (($# == 3)) || usage ;;
    close-init | observe | retry | rebegin)
        # rebegin alone may go without a run id: no run of this cutover exists yet.
        (($# == 4)) || [[ "$command" == rebegin && $# == 3 ]] || usage
        run_id="${4:-}"
        [[ -z "$run_id" || "$run_id" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]] \
            || hook_fail "the evidence run id is malformed."
        ;;
    *) usage ;;
esac
# pre_open_hint: the forward exit while no new run exists. `init` runs after cutover-postgres.sh recorded its
# forward-only marker, so a failure here must not need a run id that was never printed.
pre_open_hint() {
    HOOK_FAIL_HINT="no new evidence run was opened; fix the cause, keep native admission off and run the cutover evidence-rebegin step with evidence_run_id ${run_id:-left empty}."
}
[[ "$command" != init ]] || pre_open_hint
[[ "$task_id" =~ ^OTERYN-[0-9]{8}-[a-z0-9][a-z0-9-]{0,80}$ ]] || hook_fail "the task id is malformed."
[[ "$close_run_id" =~ ^[1-9][0-9]{0,18}$ ]] || hook_fail "the maintenance-close run id is malformed."
ev_bind_cutover "$task_id" "$close_run_id"

hook_require_tools docker gh jq python3 sed
hook_evidence_setup
[[ -f "$EVIDENCE_TOOL" ]] || hook_fail "the attempt evidence tool is missing."
[[ -d "$EVIDENCE_DIR" ]] || hook_fail "the attempt evidence directory is missing."
hook_workdir
hook_runtime_env
W="$HOOK_WORK"

# A recovery command acts only on a run of the cutover in progress: the cutover state must record native
# re-enable for this task and close run, and the run must record the same cutover_ref in its run-open. Both
# are checked before any anchor post or evidence write.
if [[ "$command" != init && "$command" != preflight ]]; then
    ev_require_cutover_state "$task_id" "$close_run_id"
fi
if [[ -n "$run_id" ]]; then
    ev_summary
    [[ -n "$(jq -r --arg r "$run_id" '.runs[] | select(.run_id == $r) | .run_id' "$W/summary.json")" ]] \
        || hook_fail "run $run_id has no file in the evidence directory."
    ev_require_ref "$run_id"
fi

# begin_checks: location, key id and kid history, then every anchor with the D3 retry and verify --anchors,
# all before any new evidence write.
begin_checks() {
    ev_check_location
    ev_key_id
    ev_verified_anchors "$W/anchors.json"
}

# run_d1 <observation-out>: the D1 init in the platform service, then a D2 observation with up to three tries.
run_d1() {
    local raw attempt
    raw="$(hook_artisan game-auth:native-evidence:init-provenance)" || true
    printf '%s\n' "$raw" | hook_last_json > "$W/init-result.json" \
        || hook_fail "the provenance init printed no result; run $run_id holds an unfinished init-intent (provenance-init.sh close-init)."
    for attempt in 1 2 3; do
        if (hook_observe "$1" --key-id="$EV_KEY_ID"); then
            return 0
        fi
        ((attempt < 3)) || hook_fail "no observation after the provenance init; run $run_id holds an unfinished init-intent (provenance-init.sh close-init)."
        sleep 5
    done
}

# write_init <late>: the init record from the D1 result and the post-D1 observation, anchored.
write_init() {
    jq -c --slurpfile obs "$W/post.json" --argjson late "$1" '{result, store_id, observation: $obs[0], late: $late}' \
        "$W/init-result.json" > "$W/init-data.json"
    ev_append init init "$run_id" --data "@$W/init-data.json" \
        || hook_fail "the provenance init was refused: $(ev_error "$W/init.json") (run $run_id)."
}

# refuse <reason>: an anchored begin-refused with the first observation, which closes the init-intent.
refuse() {
    jq -c --arg reason "$1" '{reason: $reason, observation: .}' "$W/first.json" > "$W/refused-data.json"
    ev_append refused begin-refused "$run_id" --data "@$W/refused-data.json" || true
    [[ "$(jq -r '.record.type' "$W/refused.json")" == begin-refused ]] || hook_fail "cannot write begin-refused; run $run_id."
}

# write_init_intent: init-intent after run-open, anchored with its anchored line.
write_init_intent() {
    ev "$W/init-intent.json" append init-intent --dir "$EVIDENCE_DIR" --run "$run_id" \
        || hook_fail "cannot write init-intent ($(ev_error "$W/init-intent.json"))."
    ev_append_anchored "$run_id" "$W/init-intent.json"
}

# write_attempt_intent <store_id>: observes with --key-id=K and writes attempt-intent 1, anchored with its
# anchored line. The tool checks continuity and every precondition; a refusal is an anchored begin-refused.
write_attempt_intent() {
    hook_observe "$W/intent-obs.json" --key-id="$EV_KEY_ID"
    jq -c --arg kid "$EV_KEY_ID" --arg sid "$1" --argjson count "$EV_KID_COUNT" \
        '{observation: ., scope: .scope, key_id: $kid, store_id: $sid, kid_record_count: $count, anchor_issue_locked: true}' \
        "$W/intent-obs.json" > "$W/intent-data.json"
    local intent_status=0
    ev "$W/intent.json" append attempt-intent --dir "$EVIDENCE_DIR" --run "$run_id" --data "@$W/intent-data.json" \
        || intent_status=$?
    [[ -n "$(jq -r '.record.type // empty' "$W/intent.json")" ]] \
        || hook_fail "cannot write attempt-intent ($(ev_error "$W/intent.json")); run $run_id."
    if ((intent_status != 0)); then
        ev_anchor_lines "$W/intent.json"
        hook_fail "the attempt intent was refused: $(jq -r '.reason // .error' "$W/intent.json") (run $run_id)."
    fi
    ev_append_anchored "$run_id" "$W/intent.json"
}

# probe: proves the job token can post to the anchor issue; the probe comment is not an anchor.
probe() {
    hook_post_comment "$(jq -cn --arg ref "$task_id/close-$close_run_id" '{oteryn_cutover_preflight: $ref}')" >/dev/null \
        || hook_fail "cannot post to the anchor issue as $TRUSTED_ANCHOR_AUTHOR_LOGIN (issues: write); nothing was written."
}

# resume_begin <run-state>: the runner died between two durable lines of `init` outside the init-intent window
# that D5 --close-init covers. The durable prefix is adopted and the begin goes on from the next line:
#   run-open only        open's own decision is retaken on the recorded pre-init observation (a refusal is
#                        written as begin-refused); then init-intent, D1, init and attempt-intent as in `init`.
#   init (late: false)   attempt-intent 1, anchored.
#   init (late: true)    the run-close with done: false of the init-late path.
#   attempt-intent       its anchored line (the anchor comment is reused if it was posted).
resume_begin() {
    local state="$1" last init_late store decision
    last="$(jq -r '.last_type' <<<"$state")"
    init_late="$(jq -r 'if .init == null then "" else (.init.late | tostring) end' <<<"$state")"
    store="$(jq -r '.init.store_id // empty' <<<"$state")"
    if [[ "$last" == run-open || "$last:$init_late" == init:false ]]; then
        # The begin runs before the deploy that turns native admission on, as in `init`.
        [[ "$(hook_env_get OTERYN_NATIVE_ADMISSION_ENABLED)" != true ]] \
            || hook_fail "native admission is on in the deployed environment; run $run_id cannot resume its begin. The owner decides."
    fi
    case "$last:$init_late" in
        run-open:)
            ev_state open-decision "$EVIDENCE_DIR" "$W/anchors.json" "$run_id" > "$W/open-decision.json" \
                || hook_fail "cannot retake the open decision for run $run_id."
            decision="$(jq -r '.reason // empty' "$W/open-decision.json")"
            if [[ -n "$decision" ]]; then
                jq -c '{reason, observation}' "$W/open-decision.json" > "$W/refused-data.json"
                ev_append refused begin-refused "$run_id" --data "@$W/refused-data.json" || true
                hook_fail "run $run_id: the open is refused ($decision); begin-refused is written. The owner decides."
            fi
            probe
            write_init_intent
            run_d1 "$W/post.json"
            write_init false
            store="$(jq -r '.record.store_id' "$W/init.json")"
            write_attempt_intent "$store"
            printf 'close_init_result=resumed-begin\nprovenance_init_result=%s\n' "$(jq -r '.result' "$W/init-result.json")"
            ;;
        init:false)
            probe
            write_attempt_intent "$store"
            printf 'close_init_result=resumed-attempt\n'
            ;;
        init:true)
            ev_close "$run_id" false
            printf 'close_init_result=init-late\n'
            ;;
        attempt-intent:*)
            ev_state last-anchor "$EVIDENCE_DIR" "$run_id" > "$W/intent.json" || hook_fail "cannot read run $run_id."
            ev_append_anchored "$run_id" "$W/intent.json"
            printf 'close_init_result=resumed-anchor\n'
            ;;
        *)
            hook_fail "run $run_id holds no unfinished init-intent and no resumable begin (last line $last); nothing to resume."
            ;;
    esac
    [[ -z "$store" ]] || printf 'witness_store_id=%s\n' "$store"
    printf 'attempt_run_id=%s\n' "$run_id"
}

if [[ "$command" == preflight ]]; then
    begin_checks
    probe
    printf 'provenance_preflight=passed\n'
    exit 0
fi

if [[ "$command" == close-init ]]; then
    # D5 step4-begin --close-init --run <id>: the first observation only picks the path.
    begin_checks
    ev_summary
    state="$(jq -c --arg r "$run_id" '.runs[] | select(.run_id == $r)' "$W/summary.json")"
    [[ -n "$state" ]] || hook_fail "run $run_id has no file in the evidence directory."
    ev_require_ref "$run_id"
    [[ "$(jq -r '.closed' <<<"$state")" != true ]] || hook_fail "run $run_id is closed; nothing to resume."
    if [[ "$(jq -r '.unfinished_init' <<<"$state")" != true ]]; then
        resume_begin "$state"
        exit 0
    fi
    intent_anchored="$(jq -r '.init_intent_anchored' <<<"$state")"
    hook_observe "$W/first.json" --key-id="$EV_KEY_ID"
    binding="$(jq -r '.store.binding // empty' "$W/first.json")"
    observed_store="$(jq -r 'def sid: (if type == "object" then .id else . end) | select(type == "string" and . != "");
        [(.store.marker | sid), (.store.row | sid)] | first // empty' "$W/first.json")"
    earlier_store="$(jq -r '.store_id // empty' "$W/summary.json")"
    case "$binding:$intent_anchored" in
        marker_only:true | match:true)
            run_d1 "$W/post.json"
            write_init true
            ev_close "$run_id" false
            printf 'close_init_result=init-late\nwitness_store_id=%s\nattempt_run_id=%s\n' "$(jq -r '.record.store_id' "$W/init.json")" "$run_id"
            ;;
        none:false)
            refuse init-not-run
            printf 'close_init_result=init-not-run\nattempt_run_id=%s\n' "$run_id"
            ;;
        marker_only:false | match:false)
            # D1 never ran in this run (it runs only after the anchored line); the store is the one an earlier
            # run's init recorded. Any other store id cannot be explained, which is init-lost.
            if [[ -n "$earlier_store" && "$observed_store" == "$earlier_store" ]]; then
                refuse init-not-run
                printf 'close_init_result=init-not-run\nattempt_run_id=%s\n' "$run_id"
            else
                refuse init-lost
                hook_fail "run $run_id: begin-refused init-lost (permanent for this evidence directory); the owner decides."
            fi
            ;;
        *)
            refuse init-lost
            hook_fail "run $run_id: begin-refused init-lost (permanent for this evidence directory); the owner decides."
            ;;
    esac
    exit 0
fi

if [[ "$command" == retry ]]; then
    # D5 step4-begin --retry: the last attempt of the run is failed-before-witness (nothing was witnessed) and
    # the operator fixed the cause outside trust state. attempt-intent n+1 follows in the same run file after a
    # fresh observation, with the same preconditions, anchor and anchored line; a refusal is begin-refused.
    begin_checks
    ev_summary
    ev_require_ref "$run_id"
    state="$(jq -c --arg r "$run_id" '.runs[] | select(.run_id == $r)' "$W/summary.json")"
    [[ "$(jq -r '.closed' <<<"$state")" != true && "$(jq -r '.last_type' <<<"$state")" == attempt-observation \
        && "$(jq -r '.attempts | last | .classification' <<<"$state")" == failed-before-witness ]] \
        || hook_fail "the last line of run $run_id is no failed-before-witness observation; nothing to retry."
    store="$(jq -r '.init.store_id // empty' <<<"$state")"
    [[ -n "$store" ]] || hook_fail "run $run_id holds no init; nothing to retry."
    probe
    write_attempt_intent "$store"
    printf 'retry_result=intent\nattempt=%s\nwitness_store_id=%s\nattempt_run_id=%s\nattempt_intent_line_sha256=%s\n' \
        "$(jq -r '.record.attempt' "$W/intent.json")" "$store" "$run_id" "$(jq -r '.line_sha256' "$W/intent.json")"
    exit 0
fi

if [[ "$command" == rebegin ]]; then
    # A begin of this cutover ended without an attempt that can go on: a begin-refused that is not permanent
    # (a refused open, init or intent, init-not-run) or the run-close of init-late. cutover-postgres.sh never
    # reruns native-reenable after its forward-only marker, so this step takes the begin again in a new run,
    # exactly as `init` does (open decides on the whole directory). Only the newest run of this cutover
    # qualifies, and native admission must still be off. Without a run id, no run of this cutover may exist:
    # `init` failed before open wrote one.
    [[ "$(hook_env_get OTERYN_NATIVE_ADMISSION_ENABLED)" != true ]] \
        || hook_fail "native admission is on in the deployed environment; a new begin is refused. The owner decides."
    begin_checks
    ev_summary
    if [[ -z "$run_id" ]]; then
        newest="$(jq -r --arg ref "$EV_CUTOVER_REF" '[.runs[] | select(.cutover_ref == $ref)] | sort_by(.opened_at // "")
            | last // empty | "\(.run_id) (last line \(.last_type))"' "$W/summary.json")"
        [[ -z "$newest" ]] \
            || hook_fail "the evidence directory holds run $newest of this cutover; run the evidence step for that run id. Nothing was written."
        printf 'rebegin_of=none\n'
    else
        ev_require_ref "$run_id"
        state="$(jq -c --arg r "$run_id" '.runs[] | select(.run_id == $r)' "$W/summary.json")"
        [[ "$(jq -r --arg r "$run_id" --arg ref "$EV_CUTOVER_REF" '.runs as $all | ($all[] | select(.run_id == $r) | .opened_at) as $at
            | [$all[] | select(.cutover_ref == $ref and .run_id != $r and (.opened_at // "") >= ($at // ""))] | length' "$W/summary.json")" == 0 ]] \
            || hook_fail "run $run_id is not the newest run of this cutover; name the newest one. Nothing was written."
        if [[ "$(jq -r '.last_type' <<<"$state")" == begin-refused ]]; then
            [[ "$(jq -r '.last_permanent' <<<"$state")" != true ]] \
                || hook_fail "run $run_id ended in a permanent begin-refused ($(jq -r '.last_reason' <<<"$state")); the owner decides. Nothing was written."
        else
            [[ "$(jq -r '.closed and (.done | not) and .init != null and .init.late == true' <<<"$state")" == true ]] \
                || hook_fail "run $run_id did not end in a begin that may be taken again (last line $(jq -r '.last_type' <<<"$state")). Nothing was written."
        fi
        printf 'rebegin_of=%s\n' "$run_id"
    fi
    pre_open_hint
    command=init
fi

if [[ "$command" == observe ]]; then
    # D5 step4-observe --run <id>: the missing observation of an unclosed attempt, with late: true.
    ev_verified_anchors "$W/anchors.json"
    ev_summary
    ev_require_ref "$run_id"
    [[ "$(jq -r --arg r "$run_id" '.runs[] | select(.run_id == $r) | .attempts | last | .observed' "$W/summary.json")" == false ]] \
        || hook_fail "run $run_id has no attempt waiting for its observation."
    ev_step4_observe "$run_id" true
    exit 0
fi

# --- init: 1 and 2. Location, key, anchors (with the D3 retry), a write probe; nothing written before ---
begin_checks
hook_observe "$W/pre.json" --key-id="$EV_KEY_ID"
probe

# --- 3. run-open, then init-intent ------------------------------------------------------------------
open_status=0
ev "$W/open.json" open --dir "$EVIDENCE_DIR" --anchors "$W/anchors.json" --pre-init "@$W/pre.json" \
    --location "@$W/location.json" --cutover-ref "$task_id/close-$close_run_id" || open_status=$?
run_id="$(jq -r '.run_id // empty' "$W/open.json")"
[[ -n "$run_id" ]] || hook_fail "open wrote no run ($(ev_error "$W/open.json"))."
# From here on run_id is the run every recovery step takes; each failure prints it.
printf 'attempt_run_id=%s\n' "$run_id"
HOOK_FAIL_HINT="evidence run $run_id of this cutover holds the begin; run the cutover evidence step its last line calls for (evidence-close-init, or evidence-rebegin after a begin-refused) with evidence_run_id $run_id."
ev_anchor_lines "$W/open.json"
((open_status == 0)) || hook_fail "the run was refused before the init: $(jq -r '.reason // .error' "$W/open.json") (run $run_id)."

write_init_intent

# --- 4. D1 init, observation, init record ----------------------------------------------------------
run_d1 "$W/post.json"
write_init false
init_result="$(jq -r '.result' "$W/init-result.json")"
store_id="$(jq -r '.record.store_id' "$W/init.json")"

# --- 5. attempt-intent 1 ----------------------------------------------------------------------------
write_attempt_intent "$store_id"

printf 'provenance_init_result=%s\n' "$init_result"
printf 'witness_store_id=%s\n' "$store_id"
printf 'attempt_intent_line_sha256=%s\n' "$(jq -r '.line_sha256' "$W/intent.json")"
