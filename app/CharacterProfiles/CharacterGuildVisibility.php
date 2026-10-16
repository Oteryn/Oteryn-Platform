<?php

namespace App\CharacterProfiles;

use App\CharacterProfiles\Models\CharacterProfilePreference;
use App\Identity\Models\Identity;
use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;

/**
 * The owner's Platform preference as an upper bound over Game's `guild_visible`: the public guild
 * row is shown only when both allow it. A native character that cannot be bound to a live Platform
 * identity through the account-character projection fails closed (the guild stays hidden); a bound
 * character without a saved preference keeps the default (visible).
 */
final class CharacterGuildVisibility
{
    public function allows(string $characterId): bool
    {
        try {
            $accountId = DB::table('native_account_character_rows')->where('character_id', $characterId)->value('account_id');
            if (! is_string($accountId)) {
                return false;
            }

            $identity = Identity::query()->where('account_id', $accountId)->first();
            if (! $identity instanceof Identity || $identity->isTerminated() || $identity->disabled_at !== null) {
                return false;
            }

            $preference = CharacterProfilePreference::query()
                ->where('identity_id', $identity->id)
                ->where('character_id', $characterId)
                ->first();

            return $preference === null || $preference->show_guild;
        } catch (QueryException) {
            return false;
        }
    }
}
