<?php

namespace App\CharacterProfiles;

use App\Accounts\Models\IdentityCanaryAccount;
use App\CharacterProfiles\Models\CharacterProfilePreference;
use App\Identity\Models\Identity;
use App\PublicGameData\CanaryGameDataRepository;
use Illuminate\Support\Facades\DB;
use stdClass;

final class CharacterProfilePreferenceService
{
    public function __construct(
        private readonly CanaryGameDataRepository $gameData,
        private readonly CharacterProfileEventRecorder $events,
    ) {}

    /**
     * @return array{character: stdClass, preference: CharacterProfilePreference}
     */
    public function editState(Identity $identity, string $name): array
    {
        $character = $this->ownedActiveCharacter($identity, $name);
        $playerId = $this->positiveInteger($character->id);
        if ($playerId === null) {
            throw new CharacterProfileNotOwned;
        }

        $preference = CharacterProfilePreference::query()
            ->where('identity_id', $identity->id)
            ->where('canary_player_id', $playerId)
            ->first();

        return [
            'character' => $character,
            'preference' => $preference ?? $this->newPreference($identity->id, $playerId),
        ];
    }

    /**
     * @param  array{
     *     public_comment: string|null,
     *     show_account_association: bool,
     *     show_status: bool,
     *     show_guild: bool,
     *     show_house: bool,
     *     show_skills: bool,
     *     show_deaths: bool,
     *     show_kills: bool,
     *     is_main_character: bool
     * }  $attributes
     */
    public function update(Identity $identity, string $name, array $attributes): CharacterProfilePreference
    {
        return DB::transaction(function () use ($identity, $name, $attributes): CharacterProfilePreference {
            $lockedIdentity = Identity::query()->lockForUpdate()->find($identity->id);
            if (! $lockedIdentity instanceof Identity || $lockedIdentity->isTerminated()) {
                throw new CharacterProfileNotOwned;
            }

            $character = $this->ownedActiveCharacter($lockedIdentity, $name);
            $playerId = $this->positiveInteger($character->id);
            if ($playerId === null) {
                throw new CharacterProfileNotOwned;
            }

            $preference = CharacterProfilePreference::query()
                ->where('identity_id', $lockedIdentity->id)
                ->where('canary_player_id', $playerId)
                ->lockForUpdate()
                ->first();
            $wasMain = $preference instanceof CharacterProfilePreference
                && $preference->is_main_character;

            if ($attributes['is_main_character']) {
                CharacterProfilePreference::query()
                    ->where('identity_id', $lockedIdentity->id)
                    ->where('canary_player_id', '!=', $playerId)
                    ->where('is_main_character', true)
                    ->update(['is_main_character' => false, 'updated_at' => now()]);
            }

            $preference ??= $this->newPreference($lockedIdentity->id, $playerId);
            $preference->forceFill($attributes)->save();

            $this->events->recordPreferencesUpdated($lockedIdentity->id);
            if ($attributes['is_main_character'] && ! $wasMain) {
                $this->events->recordMainCharacterSelected($lockedIdentity->id);
            }

            return $preference;
        }, 3);
    }

    private function ownedActiveCharacter(Identity $identity, string $name): stdClass
    {
        $binding = IdentityCanaryAccount::query()->whereKey($identity->id)->first();
        $accountId = $binding?->canary_account_id;
        if ($binding === null || ! $binding->isReady() || ! is_int($accountId)) {
            throw new CharacterProfileNotOwned;
        }

        $character = $this->gameData->findActiveCharacter($name);
        if (! $character instanceof stdClass) {
            throw new CharacterProfileNotOwned;
        }

        $characterAccountId = $this->positiveInteger($character->account_id);
        $playerId = $this->positiveInteger($character->id);
        if ($characterAccountId !== $accountId || $playerId === null) {
            throw new CharacterProfileNotOwned;
        }

        return $character;
    }

    private function positiveInteger(mixed $value): ?int
    {
        if (is_int($value)) {
            return $value > 0 ? $value : null;
        }

        if (is_string($value) && ctype_digit($value)) {
            $parsed = (int) $value;

            return $parsed > 0 ? $parsed : null;
        }

        return null;
    }

    private function newPreference(int $identityId, int $playerId): CharacterProfilePreference
    {
        return new CharacterProfilePreference([
            'identity_id' => $identityId,
            'canary_player_id' => $playerId,
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
