<?php

namespace App\GameAuth\CharacterBootstrapIntent;

use Illuminate\Support\Facades\DB;
use JsonException;
use stdClass;

final class CharacterBootstrapIntentReader
{
    /** @return array<string, int|string|array<string, string>>|null */
    public function current(string $operationId): ?array
    {
        $row = DB::table('character_bootstrap_intents')->where('operation_id', $operationId)->first();
        if (! $row instanceof stdClass) {
            return null;
        }
        $authority = DB::table('character_bootstrap_intent_authority')->where('id', 1)->first();
        if (! $authority instanceof stdClass
            || ! is_numeric($row->source_revision ?? null)
            || ! is_numeric($authority->last_source_revision ?? null)
            || (int) $row->source_revision < 1
            || (int) $row->source_revision > (int) $authority->last_source_revision) {
            throw new CharacterBootstrapIntentUnavailable('Character bootstrap-intent source ordering is invalid.');
        }
        $rowDecisionId = $row->issuer_decision_id ?? null;
        if ((int) $row->source_revision === (int) $authority->last_source_revision
            && (! is_string($rowDecisionId)
                || ! is_string($authority->last_issuer_decision_id ?? null)
                || ! hash_equals($rowDecisionId, $authority->last_issuer_decision_id))) {
            throw new CharacterBootstrapIntentUnavailable('Character bootstrap-intent decision identity conflicts at equal revision.');
        }
        if (! is_numeric($row->issued_at_source ?? null) || ! is_numeric($row->expires_at_source ?? null)) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap-intent source time is invalid.');
        }
        $issuedAt = (int) $row->issued_at_source;
        $expiresAt = (int) $row->expires_at_source;
        $now = now()->getTimestamp();
        if ($issuedAt < 0 || $issuedAt > $now || $expiresAt <= $issuedAt || $expiresAt - $issuedAt > 300) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap-intent source time is invalid.');
        }
        if ($expiresAt <= $now) {
            return null;
        }
        if (! is_string($row->intent_json ?? null)
            || strlen($row->intent_json) > CharacterBootstrapIntentContract::MAX_RESPONSE_BYTES
            || ! is_string($row->intent_sha256 ?? null)
            || ! hash_equals($row->intent_sha256, hash('sha256', $row->intent_json))) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap intent is invalid.');
        }
        try {
            $payload = json_decode($row->intent_json, true, 3, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap intent is invalid.', 0, $exception);
        }
        if (! is_array($payload) || array_is_list($payload)) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap intent is invalid.');
        }

        /** @var array<string, int|string|array<string, string>> $payload */
        return $payload;
    }
}
