#!/usr/bin/env bash
# Fresh Oracle trust bootstrap using the existing P6a record/anchor/fence code.
# This is a new Oracle run, never a fabricated Synology cutover or recovery.
set -euo pipefail
umask 077
action="${1:-inspect}"
case "$action" in inspect|preflight|initialize) ;; *) exit 2;; esac
HOOK_NAME=oracle-native-bootstrap
source /src/deploy/synology/scripts/cutover-hooks/common.sh
source /src/deploy/synology/scripts/cutover-hooks/evidence.sh
source /src/deploy/synology/scripts/lib.sh
hook_workdir
hook_runtime_env
W="$HOOK_WORK"
load_oteryn_env_file "$HOOK_ENV_FILE"
[[ $COMPOSE_PROJECT_NAME == oteryn-oracle && $APP_ENV == preproduction \
    && $APP_URL == https://oteryn.com && $OTERYN_NATIVE_ADMISSION_ENABLED == false \
    && $OTERYN_PLATFORM_FIRST_PUBLICATION_FENCE == required ]] \
    || hook_fail 'The fresh isolated Oracle identity, disabled ingress or required fence differs.'
EVIDENCE_DIR=/var/lib/oteryn-attempt-evidence
EVIDENCE_TOOL="$SCRIPTS_DIR/attempt_evidence.py"
ev_check_location
ev_key_id
hook_observe "$W/pre.json" --key-id="$EV_KEY_ID"
if [[ $action == inspect ]]; then
    jq -cn --slurpfile location "$W/location.json" --slurpfile trust "$W/pre.json" \
        '{location: $location[0], trust: $trust[0]}'
    exit 0
fi
[[ ${GITHUB_ACTIONS:-} == true && ${GITHUB_REPOSITORY:-} == Oteryn/Oteryn-Platform \
    && ${GITHUB_REF:-} == refs/heads/main && ${GITHUB_SHA:-} =~ ^[0-9a-f]{40}$ \
    && ${GITHUB_RUN_ID:-} =~ ^[1-9][0-9]*$ ]] || hook_fail 'Protected main Actions context required.'
hook_evidence_setup
hook_require_tools gh jq python3 docker
# Refuse every existing run: automatic recovery, floor changes and key rotation
# are outside this fresh-start command. A partial attempt needs forward recovery.
[[ -z $(find "$EVIDENCE_DIR" -mindepth 1 -maxdepth 1 -print -quit) ]] \
    || hook_fail 'Evidence already exists; retain it and reconcile forward through an explicit recovery.'
jq -e '.store.binding == "none" and .store.marker == null and .store.row == null
    and .floor.value == null and .profile.present == false and .key.in_history == false
    and .key.trusted == false and .fence_required == true
    and .database.in_flight.sessions == 0 and .database.in_flight.prepared == 0' "$W/pre.json" >/dev/null \
    || hook_fail 'This is not a pristine unwitnessed Oracle store.'
hook_fetch_anchors "$W/anchors.json"
hook_evidence verify --dir "$EVIDENCE_DIR" --anchors "$W/anchors.json" > "$W/verified.json"
if [[ $action == preflight ]]; then
    echo 'oracle_native_preflight=passed'
    exit 0
fi
ref="OTERYN-20261015-oracle-arm64-deployment/oracle-$GITHUB_RUN_ID"
# Prove the actual bot principal can post before the first local evidence write.
hook_post_comment "$(jq -cn --arg ref "$ref" --arg sha "$GITHUB_SHA" '{oteryn_oracle_preflight: $ref, source_sha: $sha}')" >/dev/null \
    || hook_fail 'The token cannot anchor as the trusted Actions bot; no local record was written.'
open_status=0
ev "$W/open.json" open --dir "$EVIDENCE_DIR" --anchors "$W/anchors.json" \
    --pre-init "@$W/pre.json" --location "@$W/location.json" --cutover-ref "$ref" || open_status=$?
run_id="$(jq -r '.run_id // empty' "$W/open.json")"
[[ -n $run_id ]] || hook_fail "No evidence run was opened ($(ev_error "$W/open.json"))."
printf 'oracle_attempt_run_id=%s\n' "$run_id"
HOOK_FAIL_HINT="Retain Oracle evidence run $run_id; no automatic retry or reset is permitted. Reconcile its actual last record forward."
ev_anchor_lines "$W/open.json"
((open_status == 0)) || hook_fail "The run was refused ($(ev_error "$W/open.json"))."
ev "$W/init-intent.json" append init-intent --dir "$EVIDENCE_DIR" --run "$run_id"
ev_append_anchored "$run_id" "$W/init-intent.json"
raw="$(hook_artisan game-auth:native-evidence:init-provenance)" || true
printf '%s\n' "$raw" | hook_last_json > "$W/init-result.json" \
    || hook_fail 'Provenance init produced no result; the anchored init-intent remains open.'
hook_observe "$W/post.json" --key-id="$EV_KEY_ID"
jq -c --slurpfile obs "$W/post.json" '{result, store_id, observation: $obs[0], late: false}' \
    "$W/init-result.json" > "$W/init-data.json"
ev_append init init "$run_id" --data "@$W/init-data.json" \
    || hook_fail "Provenance init was refused ($(ev_error "$W/init.json"))."
store_id="$(jq -r '.record.store_id' "$W/init.json")"
hook_observe "$W/intent-obs.json" --key-id="$EV_KEY_ID"
jq -c --arg kid "$EV_KEY_ID" --arg sid "$store_id" --argjson count "$EV_KID_COUNT" \
    '{observation: ., scope: .scope, key_id: $kid, store_id: $sid, kid_record_count: $count, anchor_issue_locked: true}' \
    "$W/intent-obs.json" > "$W/intent-data.json"
ev "$W/intent.json" append attempt-intent --dir "$EVIDENCE_DIR" --run "$run_id" --data "@$W/intent-data.json"
ev_append_anchored "$run_id" "$W/intent.json"
container="$(hook_compose ps -q platform)"
[[ -n $container ]] || hook_fail 'The Oracle Platform container is missing.'
fence="$(oteryn_first_publication_guard "$container")"
[[ -n $fence ]] || hook_fail 'The fresh Oracle store did not yield its required anchored fence.'
# Enable signing only for this privileged console invocation. HTTP native login
# stays off until Game assignment, projection and the real client path qualify.
publish_status=0
docker exec -e GAME_AUTH_NATIVE_ADMISSION_ENABLED=true "$container" /usr/local/bin/oteryn-platform-entrypoint \
    php artisan game-auth:native-admission:publish-trust --no-interaction \
    "--first-publication-fence=$fence" > "$W/publication.json" 2>&1 || publish_status=$?
printf 'publication_command_exit=%s\n' "$publish_status"
# Only actual settled observations classify the outcome; never synthesize a
# profile or retry a possibly committed publication from its exit status.
hook_artisan() {
    if [[ $1 == game-auth:native-admission:self-check ]]; then
        docker exec -e GAME_AUTH_NATIVE_ADMISSION_ENABLED=true "$container" /usr/local/bin/oteryn-platform-entrypoint \
            php artisan "$@" --no-interaction
    else
        hook_compose exec -T platform /usr/local/bin/oteryn-platform-entrypoint php artisan "$@" --no-interaction
    fi
}
ev_summary
ev_step4_observe "$run_id" false
