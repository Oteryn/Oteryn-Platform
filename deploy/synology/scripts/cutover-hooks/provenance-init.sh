#!/usr/bin/env bash
# Cutover hook OTERYN_CUTOVER_PROVENANCE_INIT_COMMAND (plan P6 step 4, design PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md D5
# step4-begin). Called by cutover-postgres.sh native-reenable, after its forward-only marker, as:
#
#   provenance-init.sh init <task_id> <close_run_id>
#
# With native admission still off it:
#   1. resolves the runtime environment, checks the evidence location and reads the current kid K and its
#      used.kids count (refuses before any write unless the count is 1);
#   2. fetches every anchor of the locked anchor issue, proves it can post there and that verify --anchors
#      passes, all before any evidence write;
#   3. opens a run (run-open with a pre-init observation), writes init-intent, anchors it and writes anchored;
#   4. runs `game-auth:native-evidence:init-provenance` (D1) in the platform service, observes (D2) and writes
#      the init record (a refused init writes begin-refused);
#   5. observes with --key-id=K and writes attempt-intent 1, anchors it and writes anchored.
# Every record except an anchored line is anchored before the next action. Any failure exits non-zero, so the
# cutover step fails and native admission stays off; the deploy that turns it on is the operator's next step.
#
# Inputs: the cutover step environment plus OTERYN_CUTOVER_ANCHOR_ISSUE (the locked anchor issue number),
# GITHUB_REPOSITORY and GH_TOKEN (issues: write). Output: key=value result lines only; never the environment
# file, a key, a token or an observation.
set -euo pipefail
umask 077

HOOK_NAME=provenance-init
# shellcheck source=deploy/synology/scripts/cutover-hooks/common.sh
source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/common.sh"

usage() {
    echo "Usage: provenance-init.sh init <task_id> <close_run_id>" >&2
    exit 1
}

(($# == 3)) && [[ "$1" == init ]] || usage
task_id="$2"
close_run_id="$3"
[[ "$task_id" =~ ^OTERYN-[0-9]{8}-[a-z0-9][a-z0-9-]{0,80}$ ]] || hook_fail "the task id is malformed."
[[ "$close_run_id" =~ ^[1-9][0-9]{0,18}$ ]] || hook_fail "the maintenance-close run id is malformed."

hook_require_tools docker gh jq python3 sed
hook_evidence_setup
KEY_SCRIPT="${OTERYN_CUTOVER_KEY_SCRIPT:-$SCRIPTS_DIR/generate-native-admission-key.sh}"
[[ -f "$EVIDENCE_TOOL" ]] || hook_fail "the attempt evidence tool is missing."
[[ -f "$KEY_SCRIPT" ]] || hook_fail "the native admission key tool is missing."
[[ -d "$EVIDENCE_DIR" ]] || hook_fail "the attempt evidence directory is missing."
hook_workdir
hook_runtime_env
W="$HOOK_WORK"

# evidence <out-file> <args...>: runs the tool, keeps its JSON payload in a file, and returns its exit status.
evidence() {
    local out="$1"
    shift
    hook_evidence "$@" > "$out" 2> "$W/evidence.err"
}

# anchor_lines <payload-file>: posts the anchor of every line in .lines (open) or of the line (append).
anchor_lines() {
    local anchor
    while IFS= read -r anchor; do
        [[ -n "$anchor" ]] || continue
        hook_post_anchor "$anchor" >/dev/null
    done < <(jq -c 'if has("lines") then .lines[].anchor else .anchor end // empty' "$1")
}

# append_anchored <run> <intent-payload>: posts the intent's anchor, then writes its anchored line.
append_anchored() {
    local run="$1" comment
    comment="$(hook_post_anchor "$(jq -c '.anchor' "$2")")"
    evidence "$W/anchored.json" append anchored --dir "$EVIDENCE_DIR" --run "$run" --comment-id "$comment" \
        || hook_fail "cannot write the anchored line ($(jq -r '.error // "unknown"' "$W/anchored.json"))."
}

# --- 1. Location, key id and kid history (read-only) -----------------------------------------------
project="$(hook_env_get COMPOSE_PROJECT_NAME)"
evidence "$W/location.json" check-location --dir "$EVIDENCE_DIR" --project "${project:-oteryn-staging}" \
    || hook_fail "the attempt evidence location check failed: $(jq -r '(.errors // [.error]) | join(",")' "$W/location.json" 2>/dev/null || echo unreadable)."

key_tool() {
    env -u PLATFORM_IMAGE OTERYN_ENV_FILE="$HOOK_ENV_FILE" COMPOSE_PROJECT_NAME="${project:-oteryn-staging}" \
        bash "$KEY_SCRIPT" "$@"
}
key_id="$(key_tool show | sed -n 's/^current_key_id=//p' | head -n 1)" || hook_fail "cannot read the current native admission key id."
[[ "$key_id" =~ ^[A-Za-z0-9._-]{1,64}$ ]] || hook_fail "the key tool reported no well-formed current key id."
kid_history="$(key_tool check-kid-history --kid "$key_id")" \
    || hook_fail "the used.kids history check for the current key failed (every count must be 1)."
kid_count="$(awk -v k="kid:$key_id" '$1 == "kid_history" && $2 == k { sub(/^count=/, "", $4); print $4 }' <<<"$kid_history")"
[[ "$kid_count" == 1 ]] || hook_fail "the current key has $kid_count used.kids records; exactly 1 is required."

# --- 2. Anchors and pre-init observation, before any evidence write --------------------------------
hook_observe "$W/pre.json" --key-id="$key_id"
hook_fetch_anchors "$W/anchors.json"
hook_post_comment "$(jq -cn --arg ref "$task_id/close-$close_run_id" '{oteryn_cutover_preflight: $ref}')" >/dev/null \
    || hook_fail "cannot post to the anchor issue as $TRUSTED_ANCHOR_AUTHOR_LOGIN (issues: write); nothing was written."
evidence "$W/verify.json" verify --dir "$EVIDENCE_DIR" --anchors "$W/anchors.json" \
    || hook_fail "verify --anchors fails over the evidence directory; an earlier line may need its anchor retried (D3). Nothing was written."

# --- 3. run-open, then init-intent ------------------------------------------------------------------
open_status=0
evidence "$W/open.json" open --dir "$EVIDENCE_DIR" --anchors "$W/anchors.json" --pre-init "@$W/pre.json" \
    --location "@$W/location.json" --cutover-ref "$task_id/close-$close_run_id" || open_status=$?
run_id="$(jq -r '.run_id // empty' "$W/open.json")"
if [[ -z "$run_id" ]]; then
    hook_fail "open wrote no run ($(jq -r '.error // "unknown"' "$W/open.json" 2>/dev/null || echo unreadable))."
fi
anchor_lines "$W/open.json"
((open_status == 0)) || hook_fail "the run was refused before the init: $(jq -r '.reason // .error' "$W/open.json") (run $run_id)."

evidence "$W/init-intent.json" append init-intent --dir "$EVIDENCE_DIR" --run "$run_id" \
    || hook_fail "cannot write init-intent ($(jq -r '.error // "unknown"' "$W/init-intent.json"))."
append_anchored "$run_id" "$W/init-intent.json"

# --- 4. D1 init, observation, init record ----------------------------------------------------------
init_raw="$(hook_artisan game-auth:native-evidence:init-provenance)" || true
printf '%s\n' "$init_raw" | hook_last_json > "$W/init-result.json" \
    || hook_fail "the provenance init printed no result; run $run_id holds an unfinished init-intent (D5 --close-init)."
for attempt in 1 2 3; do
    if (hook_observe "$W/post.json" --key-id="$key_id"); then
        break
    fi
    ((attempt < 3)) || hook_fail "no observation after the provenance init; run $run_id holds an unfinished init-intent (D5 --close-init)."
    sleep 5
done
jq -c --slurpfile obs "$W/post.json" '{result, store_id, observation: $obs[0]}' "$W/init-result.json" > "$W/init-data.json"
init_status=0
evidence "$W/init.json" append init --dir "$EVIDENCE_DIR" --run "$run_id" --data "@$W/init-data.json" || init_status=$?
if [[ "$(jq -r '.record.type // empty' "$W/init.json")" == "" ]]; then
    hook_fail "cannot write the init record ($(jq -r '.error // "unknown"' "$W/init.json")); run $run_id holds an unfinished init-intent (D5 --close-init)."
fi
anchor_lines "$W/init.json"
((init_status == 0)) || hook_fail "the provenance init was refused: $(jq -r '.reason // .error' "$W/init.json") (run $run_id)."
init_result="$(jq -r '.result' "$W/init-result.json")"
store_id="$(jq -r '.record.store_id' "$W/init.json")"

# --- 5. attempt-intent 1 ----------------------------------------------------------------------------
hook_observe "$W/intent-obs.json" --key-id="$key_id"
jq -c --arg kid "$key_id" --arg sid "$store_id" --argjson count "$kid_count" \
    '{observation: ., scope: .scope, key_id: $kid, store_id: $sid, kid_record_count: $count, anchor_issue_locked: true}' \
    "$W/intent-obs.json" > "$W/intent-data.json"
intent_status=0
evidence "$W/intent.json" append attempt-intent --dir "$EVIDENCE_DIR" --run "$run_id" --data "@$W/intent-data.json" \
    || intent_status=$?
[[ -n "$(jq -r '.record.type // empty' "$W/intent.json")" ]] \
    || hook_fail "cannot write attempt-intent ($(jq -r '.error // "unknown"' "$W/intent.json")); run $run_id."
if ((intent_status != 0)); then
    anchor_lines "$W/intent.json"
    hook_fail "the attempt intent was refused: $(jq -r '.reason // .error' "$W/intent.json") (run $run_id)."
fi
append_anchored "$run_id" "$W/intent.json"

printf 'provenance_init_result=%s\n' "$init_result"
printf 'witness_store_id=%s\n' "$store_id"
printf 'attempt_run_id=%s\n' "$run_id"
printf 'attempt_intent_line_sha256=%s\n' "$(jq -r '.line_sha256' "$W/intent.json")"
