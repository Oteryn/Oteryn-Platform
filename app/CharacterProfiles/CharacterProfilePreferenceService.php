<?php

namespace App\CharacterProfiles;

use App\CharacterProfiles\Models\CharacterProfilePreference;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersAccountView;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersReadModel;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharacterSummary;
use App\Identity\Models\Identity;
use App\Identity\Support\CanonicalAccountId;
use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;
use UnexpectedValueException;

/**
 * Owner preferences of the native public character page, keyed by the native CharacterId. Ownership
 * comes from the account-character projection of the identity's AccountId; Game owns the character
 * data and Platform stores only what the owner chose.
 */
final class CharacterProfilePreferenceService
{
    public function __construct(
        private readonly NativeAccountCharactersReadModel $characters,
        private readonly CharacterProfileEventRecorder $events,
    ) {}

    /**
     * @return array{character: NativeAccountCharacterSummary, preference: CharacterProfilePreference}
     *
     * @throws CharacterProfileNotOwned
     * @throws CharacterProfileUnavailable
     */
    public function editState(Identity $identity, string $name): array
    {
        $character = $this->ownedCharacter($identity, $name, false);

        try {
            $preference = CharacterProfilePreference::query()
                ->where('identity_id', $identity->id)
                ->where('character_id', $character->characterId)
                ->first();
        } catch (QueryException) {
            throw new CharacterProfileUnavailable;
        }

        return [
            'character' => $character,
            'preference' => $preference ?? $this->newPreference($identity->id, $character->characterId),
        ];
    }

    /**
     * @param  array{show_guild: bool, is_main_character: bool}  $attributes
     *
     * @throws CharacterProfileNotOwned
     * @throws CharacterProfileUnavailable
     */
    public function update(Identity $identity, string $name, array $attributes): CharacterProfilePreference
    {
        return DB::transaction(function () use ($identity, $name, $attributes): CharacterProfilePreference {
            $lockedIdentity = Identity::query()->lockForUpdate()->find($identity->id);
            if (! $lockedIdentity instanceof Identity) {
                throw new CharacterProfileNotOwned;
            }

            $character = $this->ownedCharacter($lockedIdentity, $name, true);

            $preference = CharacterProfilePreference::query()
                ->where('identity_id', $lockedIdentity->id)
                ->where('character_id', $character->characterId)
                ->lockForUpdate()
                ->first();
            $wasMain = $preference instanceof CharacterProfilePreference
                && $preference->is_main_character;

            if ($attributes['is_main_character']) {
                CharacterProfilePreference::query()
                    ->where('identity_id', $lockedIdentity->id)
                    ->where(fn ($query) => $query->whereNull('character_id')->orWhere('character_id', '!=', $character->characterId))
                    ->where('is_main_character', true)
                    ->update(['is_main_character' => false, 'updated_at' => now()]);
            }

            $preference ??= $this->newPreference($lockedIdentity->id, $character->characterId);
            $preference->forceFill($attributes)->save();

            $this->events->recordPreferencesUpdated($lockedIdentity->id);
            if ($attributes['is_main_character'] && ! $wasMain) {
                $this->events->recordMainCharacterSelected($lockedIdentity->id);
            }

            return $preference;
        }, 3);
    }

    private function ownedCharacter(Identity $identity, string $name, bool $lock): NativeAccountCharacterSummary
    {
        $accountId = $identity->getAttribute('account_id');
        if (! is_string($accountId) || ! CanonicalAccountId::isValid($accountId)
            || $identity->isTerminated() || $identity->disabled_at !== null) {
            throw new CharacterProfileNotOwned;
        }

        try {
            $view = $this->characters->viewForAccount($accountId, now()->getTimestamp(), $lock);
        } catch (QueryException|UnexpectedValueException) {
            throw new CharacterProfileUnavailable;
        }
        if ($view->state === NativeAccountCharactersAccountView::MISSING) {
            throw new CharacterProfileNotOwned;
        }
        if ($view->state !== NativeAccountCharactersAccountView::READY) {
            throw new CharacterProfileUnavailable;
        }

        // An exact name wins; a case-folded name resolves only when exactly one owned character
        // matches it, so case variants never read or write each other's preference.
        $folded = [];
        foreach ($view->characters as $character) {
            if ($character->name === $name) {
                return $character;
            }
            if (strtolower($character->name) === strtolower($name)) {
                $folded[] = $character;
            }
        }
        if (count($folded) !== 1) {
            throw new CharacterProfileNotOwned;
        }

        return $folded[0];
    }

    private function newPreference(int $identityId, string $characterId): CharacterProfilePreference
    {
        return new CharacterProfilePreference([
            'identity_id' => $identityId,
            'character_id' => $characterId,
            'canary_player_id' => null,
            'public_comment' => null,
            'show_account_association' => false,
            'show_status' => false,
            'show_guild' => true,
            'show_house' => true,
            'show_skills' => true,
            'show_deaths' => true,
            'show_kills' => true,
            'is_main_character' => false,
        ]);
    }
}
