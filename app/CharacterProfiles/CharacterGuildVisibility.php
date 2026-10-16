<?php

namespace App\CharacterProfiles;

use App\CharacterProfiles\Models\CharacterProfilePreference;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersAccountView;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersReadModel;
use App\Identity\Models\Identity;
use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;
use UnexpectedValueException;

/**
 * The owner's Platform preference as an upper bound over Game's `guild_visible`: the public guild
 * row is shown only when both allow it. Ownership is proven at serve time through the live
 * account-character read model (enabled consumer, fresh watermark at the highest epoch, valid
 * snapshot at that epoch). A native character that cannot be bound to exactly one live Platform
 * identity this way (disabled, stale, epoch-fenced, invalid, ambiguous or unreadable) fails closed:
 * the guild stays hidden. A bound character without a saved preference keeps the default (visible).
 */
final class CharacterGuildVisibility
{
    /** More candidate accounts than this for one character is treated as ambiguous. */
    private const MAX_CANDIDATE_ACCOUNTS = 3;

    public function __construct(private readonly NativeAccountCharactersReadModel $characters) {}

    public function allows(string $characterId): bool
    {
        try {
            $accountId = $this->liveOwnerAccount($characterId);
            if ($accountId === null) {
                return false;
            }

            $identities = Identity::query()->where('account_id', $accountId)->limit(2)->get();
            if ($identities->count() !== 1) {
                return false;
            }
            $identity = $identities->first();
            if (! $identity instanceof Identity || $identity->isTerminated() || $identity->disabled_at !== null) {
                return false;
            }

            $preference = CharacterProfilePreference::query()
                ->where('identity_id', $identity->id)
                ->where('character_id', $characterId)
                ->first();

            return $preference === null || $preference->show_guild;
        } catch (QueryException|UnexpectedValueException) {
            return false;
        }
    }

    /**
     * The single account whose live, current-epoch snapshot holds the character, or null. Leftover
     * rows of an account whose snapshot is invalid or at a fenced epoch do not count as ownership.
     */
    private function liveOwnerAccount(string $characterId): ?string
    {
        $candidates = DB::table('native_account_character_rows')
            ->where('character_id', $characterId)
            ->distinct()
            ->orderBy('account_id')
            ->limit(self::MAX_CANDIDATE_ACCOUNTS + 1)
            ->pluck('account_id')
            ->all();
        if ($candidates === [] || count($candidates) > self::MAX_CANDIDATE_ACCOUNTS) {
            return null;
        }

        $now = now()->getTimestamp();
        $owners = [];
        foreach ($candidates as $accountId) {
            if (! is_string($accountId)) {
                return null;
            }
            $view = $this->characters->viewForAccount($accountId, $now);
            if ($view->state === NativeAccountCharactersAccountView::UNAVAILABLE
                || $view->state === NativeAccountCharactersAccountView::STALE) {
                // The feed itself is not live: no ownership can be proven for anyone.
                return null;
            }
            if ($view->state !== NativeAccountCharactersAccountView::READY) {
                continue;
            }
            foreach ($view->characters as $character) {
                if (strcasecmp($character->characterId, $characterId) === 0) {
                    $owners[] = $accountId;
                    break;
                }
            }
        }

        return count($owners) === 1 ? $owners[0] : null;
    }
}
