<?php

namespace App\GameAuth\CharacterBootstrapIntent;

use Illuminate\Support\Facades\DB;
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
        $payload = CharacterBootstrapIntentContract::decodeStored($row);

        $authority = DB::table('character_bootstrap_intent_authority')->where('id', 1)->first();
        $lastRevision = filter_var(
            $authority instanceof stdClass ? ($authority->last_source_revision ?? null) : null,
            FILTER_VALIDATE_INT,
            ['options' => ['min_range' => 0]],
        );
        $sourceRevision = (int) $payload['source_revision'];
        if (! is_int($lastRevision) || $sourceRevision > $lastRevision) {
            throw new CharacterBootstrapIntentUnavailable('Character bootstrap-intent source ordering is invalid.');
        }
        if ($sourceRevision === $lastRevision
            && (! is_string($authority->last_issuer_decision_id ?? null)
                || ! hash_equals($payload['issuer_decision_id'], $authority->last_issuer_decision_id))) {
            throw new CharacterBootstrapIntentUnavailable('Character bootstrap-intent decision identity conflicts at equal revision.');
        }

        $issuedAt = (int) $payload['issued_at_source'];
        $expiresAt = (int) $payload['expires_at_source'];
        $now = now()->getTimestamp();
        if ($issuedAt > $now) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap-intent source time is invalid.');
        }
        if ($expiresAt <= $now) {
            return null;
        }

        return $payload;
    }
}
