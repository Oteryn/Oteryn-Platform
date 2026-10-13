# shellcheck shell=bash
# Attempt evidence flows for the cutover hooks (PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md D3 to D5). Sourced
# after common.sh, never executed. Call hook_evidence_setup, hook_workdir and hook_runtime_env first.
#
# Every record except an `anchored` line is anchored before the next action. Every refusal or failure exits
# non-zero through hook_fail, so the calling cutover step fails closed. Nothing here prints an observation,
# a key or a token; the hooks print key=value result lines only.

# The read-only D2 observations of step4-observe: two at least 5 s apart, repeated while the witness is busy
# or a session or prepared transaction is in flight, for at most this many rounds (about two minutes).
EV_OBSERVE_ROUNDS=12
EV_KID_PATTERN='^[A-Za-z0-9._-]{1,64}$'

# ev <out-file> <args...>: runs the evidence tool, keeps its JSON payload in a file, returns its exit status.
ev() {
    local out="$1"
    shift
    hook_evidence "$@" > "$out" 2> "$HOOK_WORK/evidence.err"
}

ev_error() {
    jq -r '[.error, .reason, .message] | map(select(. != null)) | join(": ") | if . == "" then "unknown" else . end' "$1" \
        2>/dev/null || echo unreadable
}

# ev_state <command> <args...>: read-only helpers over the evidence directory, built on the tool's own parser.
#   pending <dir> <anchors-file>  prints the anchor of every run whose last line is not anchored (D3 retry)
#   summary <dir>                 prints the runs, their unfinished init-intent and their attempts as JSON
ev_state() {
    python3 -I - "$SCRIPTS_DIR/attempt_evidence.py" "$@" <<'PY'
import importlib.util
import json
import sys

spec = importlib.util.spec_from_file_location("attempt_evidence", sys.argv[1])
ae = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ae)
cmd = sys.argv[2]
runs, _ = ae.load_runs(sys.argv[3])
if cmd == "pending":
    _, anchors = ae.load_anchors(sys.argv[4])
    known = list(anchors.values())
    for run in runs:
        if run.errors or not run.recs:
            continue
        anchor = ae.anchor_of(run.recs[-1], run.hashes[-1])
        if anchor is not None and anchor not in known:
            print(json.dumps(anchor, sort_keys=True, separators=(",", ":")))
elif cmd == "summary":
    A = ae.analyze(runs)
    out = {"store_id": A.S0, "runs": []}
    for run in runs:
        recs = run.recs
        last = recs[-1] if recs else {}
        intent_anchored = None
        if run.id in A.unfinished:
            i = max(n for n, r in enumerate(recs) if r["type"] == "init-intent")
            intent_anchored = i + 1 < len(recs) and recs[i + 1]["type"] == "anchored"
        out["runs"].append({
            "run_id": run.id,
            "errors": run.errors,
            "last_type": last.get("type"),
            "done": last.get("type") == "run-close" and last.get("done") is True,
            "closed": last.get("type") == "run-close",
            "unfinished_init": run.id in A.unfinished,
            "init_intent_anchored": intent_anchored,
            "attempts": [{"attempt": x["n"], "key_id": x["intent"].get("key_id"), "observed": x["obs"] is not None,
                          "classification": x["cls"]} for x in A.attempts if x["run"] is run],
        })
    print(json.dumps(out, sort_keys=True))
else:
    sys.exit(2)
PY
}

# ev_anchor_lines <payload-file>: posts the anchor of every line in .lines (open) or of the line (append).
ev_anchor_lines() {
    local anchor
    while IFS= read -r anchor; do
        [[ -n "$anchor" ]] || continue
        hook_post_anchor "$anchor" >/dev/null
    done < <(jq -c 'if has("lines") then .lines[].anchor else .anchor end // empty' "$1")
}

# ev_append_anchored <run> <intent-payload>: posts the intent's anchor, then writes its anchored line.
ev_append_anchored() {
    local run="$1" comment
    comment="$(hook_post_anchor "$(jq -c '.anchor' "$2")")"
    ev "$HOOK_WORK/anchored.json" append anchored --dir "$EVIDENCE_DIR" --run "$run" --comment-id "$comment" \
        || hook_fail "cannot write the anchored line ($(ev_error "$HOOK_WORK/anchored.json"))."
}

# ev_append <name> <type> <run> [append args...]: appends one record and anchors it. A begin-refused result is
# anchored too; the caller reads .error from $HOOK_WORK/<name>.json. Fails when no line was written.
ev_append() {
    local name="$1" type="$2" run="$3" status=0
    shift 3
    ev "$HOOK_WORK/$name.json" append "$type" --dir "$EVIDENCE_DIR" --run "$run" "$@" || status=$?
    [[ -n "$(jq -r '.record.type // empty' "$HOOK_WORK/$name.json" 2>/dev/null)" ]] \
        || hook_fail "cannot write $type ($(ev_error "$HOOK_WORK/$name.json")); run $run."
    ev_anchor_lines "$HOOK_WORK/$name.json"
    return "$status"
}

# ev_retry_anchors <anchors-out>: D3 anchor retry, then a fresh fetch of every anchor into <anchors-out>. A run
# whose last line was never anchored (a post failed) gets that post retried first; an existing trusted comment
# with the same anchor is reused. Fails when a post still fails.
ev_retry_anchors() {
    local out="$1" anchor
    hook_fetch_anchors "$out"
    ev_state pending "$EVIDENCE_DIR" "$out" > "$HOOK_WORK/pending-anchors.txt" \
        || hook_fail "cannot read the attempt evidence directory."
    [[ -s "$HOOK_WORK/pending-anchors.txt" ]] || return 0
    while IFS= read -r anchor; do
        hook_post_anchor "$anchor" >/dev/null
        printf 'anchor_retried=%s/%s\n' "$(jq -r .run_id <<<"$anchor")" "$(jq -r .seq <<<"$anchor")"
    done < "$HOOK_WORK/pending-anchors.txt"
    hook_fetch_anchors "$out"
}

# ev_verified_anchors <anchors-out>: the D3 retry, then verify --anchors over the whole directory. Every
# summary a hook acts on is read only after this passes, so a deleted or rewritten anchor, or a run file with
# integrity errors, stops the hook before it writes or accepts anything.
ev_verified_anchors() {
    ev_retry_anchors "$1"
    ev "$HOOK_WORK/verify.json" verify --dir "$EVIDENCE_DIR" --anchors "$1" \
        || hook_fail "verify --anchors fails over the evidence directory ($(jq -r '.errors | first // "unknown"' "$HOOK_WORK/verify.json" 2>/dev/null)). Nothing was written; the owner decides."
}

ev_summary() {
    ev_state summary "$EVIDENCE_DIR" > "$HOOK_WORK/summary.json" || hook_fail "cannot read the attempt evidence directory."
}

ev_check_location() {
    EV_PROJECT="$(hook_env_get COMPOSE_PROJECT_NAME)"
    EV_PROJECT="${EV_PROJECT:-oteryn-staging}"
    ev "$HOOK_WORK/location.json" check-location --dir "$EVIDENCE_DIR" --project "$EV_PROJECT" \
        || hook_fail "the attempt evidence location check failed: $(jq -r '(.errors // [.error]) | join(",")' "$HOOK_WORK/location.json" 2>/dev/null || echo unreadable)."
}

# ev_key_id: the current kid K from the key tool's show action and its used.kids count, which must be 1 (D7).
# Sets EV_KEY_ID and EV_KID_COUNT. Call ev_check_location first.
ev_key_id() {
    local key_script="${OTERYN_CUTOVER_KEY_SCRIPT:-$SCRIPTS_DIR/generate-native-admission-key.sh}" history
    [[ -f "$key_script" ]] || hook_fail "the native admission key tool is missing."
    EV_KEY_ID="$(env -u PLATFORM_IMAGE OTERYN_ENV_FILE="$HOOK_ENV_FILE" COMPOSE_PROJECT_NAME="$EV_PROJECT" \
        bash "$key_script" show | sed -n 's/^current_key_id=//p' | head -n 1)" \
        || hook_fail "cannot read the current native admission key id."
    [[ "$EV_KEY_ID" =~ $EV_KID_PATTERN ]] || hook_fail "the key tool reported no well-formed current key id."
    history="$(env -u PLATFORM_IMAGE OTERYN_ENV_FILE="$HOOK_ENV_FILE" COMPOSE_PROJECT_NAME="$EV_PROJECT" \
        bash "$key_script" check-kid-history --kid "$EV_KEY_ID")" \
        || hook_fail "the used.kids history check for the current key failed (every count must be 1)."
    EV_KID_COUNT="$(awk -v k="kid:$EV_KEY_ID" '$1 == "kid_history" && $2 == k { sub(/^count=/, "", $4); print $4 }' <<<"$history")"
    [[ "$EV_KID_COUNT" == 1 ]] || hook_fail "the current key has $EV_KID_COUNT used.kids records; exactly 1 is required."
}

# ev_observe_any <out-file> [--key-id=K]: one D2 observation whatever its result (observed, witness_busy, ...).
# Fails only when the command printed no JSON object.
ev_observe_any() {
    local out="$1" raw
    shift
    raw="$(hook_artisan game-auth:native-evidence:observe-trust --trust=fresh "$@")" || true
    printf '%s\n' "$raw" | hook_last_json > "$out"
}

# ev_settled <observation-file>: observed, with every in_flight count zero (the tool's in_flight_zero).
ev_settled() {
    jq -e 'def zero: if type == "number" then . == 0 elif type == "object" then length > 0 and all(.[]; zero) else false end;
        .result == "observed" and (.database.in_flight | zero)' "$1" >/dev/null 2>&1
}

ev_attempt_key_id() {
    jq -r --arg r "$1" '.runs[] | select(.run_id == $r) | .attempts | last | .key_id // empty' "$HOOK_WORK/summary.json"
}

# ev_close <run> <done> [classification]: writes run-close and anchors it.
ev_close() {
    local data
    data="$(jq -cn --argjson finished "$2" --arg cls "${3:-}" '{done: $finished} + (if $cls == "" then {} else {classification: $cls} end)')"
    ev_append close run-close "$1" --data "$data" || hook_fail "cannot write run-close ($(ev_error "$HOOK_WORK/close.json")); run $1."
}

# ev_self_check <run> <observation-file> <late>: runs the read-only signing self-check
# (game-auth:native-admission:self-check, which writes no trust row and no witness file), writes the
# self-check record with its result and anchors it. A passing check closes the run with done: true.
ev_self_check() {
    local run="$1" obs="$2" late="$3" raw status=0 extra=()
    raw="$(hook_artisan game-auth:native-admission:self-check)" || status=$?
    printf '%s\n' "$raw" | hook_last_json > "$HOOK_WORK/self-check-line.json" || printf 'null\n' > "$HOOK_WORK/self-check-line.json"
    jq -cn --argjson status "$status" --argjson late "$late" --slurpfile line "$HOOK_WORK/self-check-line.json" \
        --slurpfile obs "$obs" \
        '{passed: ($status == 0 and ($line[0] | type == "object" and .result == "ok")), exit_status: $status,
          result_line: $line[0], observation: $obs[0], late: $late}' > "$HOOK_WORK/self-check-data.json"
    [[ "$late" == true ]] && extra=(--anchors "$HOOK_WORK/anchors.json")
    if ! ev "$HOOK_WORK/self-check.json" append self-check --dir "$EVIDENCE_DIR" --run "$run" \
        --data "@$HOOK_WORK/self-check-data.json" "${extra[@]}"; then
        hook_fail "the self-check record was refused ($(ev_error "$HOOK_WORK/self-check.json")$(jq -r 'if .classification then ", observation classifies \(.classification)" else "" end' "$HOOK_WORK/self-check.json" 2>/dev/null)); nothing was written to run $run. The owner decides."
    fi
    ev_anchor_lines "$HOOK_WORK/self-check.json"
    if [[ "$(jq -r '.record.passed' "$HOOK_WORK/self-check.json")" != true ]]; then
        hook_fail "the signing self-check failed (exit $status); run $run stays open. Fix the cause and run native-smoke again, which rechecks (D5 step4-recheck)."
    fi
    ev_close "$run" true
    printf 'attempt_result=committed\nself_check=passed\n'
}

# ev_step4_observe <run> <late>: D5 step4-observe for the run's open attempt.
ev_step4_observe() {
    local run="$1" late="$2" key_id round cls
    key_id="$(ev_attempt_key_id "$run")"
    [[ "$key_id" =~ $EV_KID_PATTERN ]] || hook_fail "the open attempt of run $run names no well-formed key id."
    for ((round = 1; round <= EV_OBSERVE_ROUNDS; round++)); do
        ev_observe_any "$HOOK_WORK/o1.json" --key-id="$key_id" \
            || hook_fail "the trust observation printed no JSON; the attempt of run $run stays unobserved."
        sleep 5
        ev_observe_any "$HOOK_WORK/o2.json" --key-id="$key_id" \
            || hook_fail "the trust observation printed no JSON; the attempt of run $run stays unobserved."
        if ev_settled "$HOOK_WORK/o1.json" && ev_settled "$HOOK_WORK/o2.json"; then
            break
        fi
        ((round == EV_OBSERVE_ROUNDS)) || sleep 5
    done
    jq -cn --argjson late "$late" --slurpfile o1 "$HOOK_WORK/o1.json" --slurpfile o2 "$HOOK_WORK/o2.json" \
        '{deploy_exit: null, observation_1: $o1[0], observation_2: $o2[0], late: $late}' > "$HOOK_WORK/observation-data.json"
    ev "$HOOK_WORK/observation.json" append attempt-observation --dir "$EVIDENCE_DIR" --run "$run" \
        --data "@$HOOK_WORK/observation-data.json" \
        || hook_fail "cannot write attempt-observation ($(ev_error "$HOOK_WORK/observation.json")); the attempt of run $run stays unobserved."
    ev_anchor_lines "$HOOK_WORK/observation.json"
    hook_fetch_anchors "$HOOK_WORK/anchors.json"
    ev "$HOOK_WORK/classify.json" classify --dir "$EVIDENCE_DIR" --anchors "$HOOK_WORK/anchors.json" || true
    [[ "$(jq -r '.ok' "$HOOK_WORK/classify.json" 2>/dev/null)" == true ]] \
        || hook_fail "verify --anchors fails after the observation of run $run; nothing more is written. The owner decides."
    cls="$(jq -r '.record.classification // "unknown"' "$HOOK_WORK/observation.json")"
    printf 'attempt_run_id=%s\nattempt_classification=%s\n' "$run" "$cls"
    case "$cls" in
        committed)
            ev_self_check "$run" "$HOOK_WORK/o2.json" false
            ;;
        failed-before-witness)
            hook_fail "the attempt of run $run failed before the witness. Fix the cause outside trust state; a retry (D5 step4-begin --retry, intent n+1) is not implemented in the hooks, so run $run stays open and the owner decides."
            ;;
        *)
            ev_close "$run" false "$cls"
            hook_fail "the attempt of run $run is classified $cls; the run is closed with done: false and the cutover stops for the owner."
            ;;
    esac
}

# ev_step4_recheck <run>: D5 step4-recheck. A fresh observation, the read-only self-check, then self-check with
# late: true. The tool refuses the record, and nothing is written, unless verify --anchors passes and the
# observation still classifies committed against the attempt's intent.
ev_step4_recheck() {
    local run="$1" key_id
    key_id="$(ev_attempt_key_id "$run")"
    [[ "$key_id" =~ $EV_KID_PATTERN ]] || hook_fail "the committed attempt of run $run names no well-formed key id."
    hook_observe "$HOOK_WORK/recheck.json" --key-id="$key_id"
    printf 'attempt_run_id=%s\nattempt_classification=committed\n' "$run"
    ev_self_check "$run" "$HOOK_WORK/recheck.json" true
}

# ev_step4_after_deploy: what native-smoke runs. It closes the attempt that the deploy just used:
# step4-observe for the one unobserved attempt, step4-recheck for a committed attempt still waiting for a
# passing self-check, nothing for an attempt already closed with done: true. Anything else stops.
ev_step4_after_deploy() {
    local unobserved pending finished
    ev_verified_anchors "$HOOK_WORK/anchors.json"
    ev_summary
    unobserved="$(jq -r '[.runs[] | select(any(.attempts[]; .observed == false)) | .run_id] | join(" ")' "$HOOK_WORK/summary.json")"
    pending="$(jq -r '[.runs[] | select(any(.attempts[]; .classification == "committed") and (.closed | not)) | .run_id] | join(" ")' "$HOOK_WORK/summary.json")"
    finished="$(jq -r '[.runs[] | select(any(.attempts[]; .classification == "committed") and .done) | .run_id] | join(" ")' "$HOOK_WORK/summary.json")"
    if [[ -n "$unobserved" ]]; then
        [[ "$unobserved" != *" "* && -z "$pending$finished" ]] || hook_fail "the evidence directory holds more than one open or committed attempt; the owner decides."
        ev_step4_observe "$unobserved" false
    elif [[ -n "$pending" ]]; then
        [[ "$pending" != *" "* && -z "$finished" ]] || hook_fail "the evidence directory holds more than one committed attempt; the owner decides."
        ev_step4_recheck "$pending"
    elif [[ -n "$finished" && "$finished" != *" "* ]]; then
        printf 'attempt_run_id=%s\nattempt_result=committed\n' "$finished"
    else
        hook_fail "no attempt waits for its observation and none is committed; native-reenable (step 4) has not recorded an attempt. The owner decides."
    fi
}
