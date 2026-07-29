<?php

namespace App\CharacterProfiles;

use App\Accounts\Models\IdentityCanaryAccount;
use App\Audit\SecurityEventRecorder;
use App\CharacterProfiles\Models\CharacterProfilePreference;
use App\Identity\Models\Identity;
use App\PublicGameData\CanaryGameDataRepository;
use Illuminate\Support\Facades\DB;
use stdClass;

final class CharacterProfilePreferenceService
{
    public function __construct(
        private readonly CanaryGameDataRepository $gameData,
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    /**
     * @return array{character: stdClass, preference: CharacterProfilePreference}
     */
    public function editState(Identity $identity, string $name): array
    {
        $character = $this->ownedActiveCharacter($identity, $name);
        $preference = CharacterProfilePreference::query()
            ->where('identity_id', $identity->id)
            ->where('canary_player_id', (int) $character->id)
            ->first();

        return [
            'character' => $character,
            'preference' => $preference ?? $this->newPreference($identity->id, (int) $character->id),
        ];
    }

    /**
     * @param  array{
     *     public_comment: string,
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
            $playerId = (int) $character->id;
            $preference = CharacterProfilePreference::query()
                ->where('identity_id', $lockedIdentity->id)
                ->where('canary_player_id', $playerId)
                ->lockForUpdate()
                ->first();
            $wasMain = $preference?->is_main_character ?? false;

            if ($attributes['is_main_character']) {
                CharacterProfilePreference::query()
                    ->where('identity_id', $lockedIdentity->id)
                    ->where('canary_player_id', '!=', $playerId)
                    ->where('is_main_character', true)
                    ->update(['is_main_character' => false, 'updated_at' => now()]);
            }

            $preference ??= $this->newPreference($lockedIdentity->id, $playerId);
            $preference->forceFill($attributes)->save();

            $this->securityEvents->recordCharacterProfilePreferencesUpdated($lockedIdentity->id);
            if ($attributes['is_main_character'] && ! $wasMain) {
                $this->securityEvents->recordMainCharacterSelected($lockedIdentity->id);
            }

            return $preference;
        }, 3);
    }

    /**
     * @return stdClass
     */
    private function ownedActiveCharacter(Identity $identity, string $name): stdClass
    {
        $binding = IdentityCanaryAccount::query()->whereKey($identity->id)->first();
        $accountId = $binding?->canary_account_id;
        if ($binding === null || ! $binding->isReady() || ! is_int($accountId)) {
            throw new CharacterProfileNotOwned;
        }

        $character = $this->gameData->findActiveCharacter($name);
        if (! $character instanceof stdClass || (int) $character->account_id !== $accountId) {
            throw new CharacterProfileNotOwned;
        }

        return $character;
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
