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
        if (! $authority instanceof stdClass) {
            throw new CharacterBootstrapIntentUnavailable('Character bootstrap-intent authority state is unavailable.');
        }
        CharacterBootstrapIntentContract::assertAuthorityState($authority, $payload);

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
