<?php

namespace App\PublicGameData;

use App\Accounts\Models\IdentityCanaryAccount;
use App\Identity\Models\Identity;
use Carbon\CarbonImmutable;
use Illuminate\Support\Collection;
use stdClass;

final class PublicCharacterProfileService
{
    public function __construct(private readonly CanaryGameDataRepository $gameData) {}

    /**
     * @return array{
     *     character: array<string, mixed>,
     *     house: array{name: string, size: int}|null,
     *     deaths: Collection<int, stdClass>,
     *     kills: array{count: int, recent: Collection<int, stdClass>},
     *     related_characters: Collection<int, stdClass>,
     *     account_association_public: bool,
     *     status: array{online: bool, last_login: CarbonImmutable|null, last_logout: CarbonImmutable|null}|null
     * }|null
     */
    public function find(string $name): ?array
    {
        $record = $this->gameData->findActiveCharacter($name);

        if ($record === null) {
            return null;
        }

        /**
         * @var object{
         *     id: int,
         *     name: string,
         *     account_id: int,
         *     level: int,
         *     vocation: int,
         *     maglevel: int,
         *     lastlogin: int,
         *     lastlogout: int,
         *     comment: string,
         *     boss_points: int,
         *     skill_fist: int,
         *     skill_club: int,
         *     skill_sword: int,
         *     skill_axe: int,
         *     skill_dist: int,
         *     skill_shielding: int,
         *     skill_fishing: int,
         *     guild_name: mixed,
         *     guild_rank: mixed
         * } $record
         */
        $identity = $this->identityForCanaryAccount($record->account_id);
        /** @var Collection<int, stdClass> $relatedCharacters */
        $relatedCharacters = collect();
        $accountAssociationPublic = false;
        $status = null;

        if ($identity !== null && ! $identity->isTerminated()) {
            $accountAssociationPublic = $identity->public_account_association;

            if ($accountAssociationPublic) {
                $relatedCharacters = $this->gameData->publicCharactersForAccount(
                    $record->account_id,
                    $record->id,
                    CommunityDataPolicy::profileRelatedCharacterLimit(),
                );
            }

            if ($identity->public_status_visible) {
                $status = [
                    'online' => $this->gameData->isCharacterOnline($record->id),
                    'last_login' => $this->timestamp($record->lastlogin),
                    'last_logout' => $this->timestamp($record->lastlogout),
                ];
            }
        }

        $house = $this->gameData->houseForPlayer($record->id);
        if ($house !== null) {
            /** @var object{name: string, size: int} $house */
        }

        return [
            'character' => [
                'name' => $record->name,
                'level' => $record->level,
                'vocation' => $record->vocation,
                'magic_level' => $record->maglevel,
                'comment' => trim($record->comment),
                'boss_points' => $record->boss_points,
                'guild_name' => is_string($record->guild_name) && $record->guild_name !== '' ? $record->guild_name : null,
                'guild_rank' => is_string($record->guild_rank) && $record->guild_rank !== '' ? $record->guild_rank : null,
                'skills' => [
                    'fist' => $record->skill_fist,
                    'club' => $record->skill_club,
                    'sword' => $record->skill_sword,
                    'axe' => $record->skill_axe,
                    'distance' => $record->skill_dist,
                    'shielding' => $record->skill_shielding,
                    'fishing' => $record->skill_fishing,
                ],
            ],
            'house' => $house === null ? null : [
                'name' => $house->name,
                'size' => $house->size,
            ],
            'deaths' => $this->gameData->deathsForPlayer(
                $record->id,
                CommunityDataPolicy::profileDeathLimit(),
            ),
            'kills' => $this->gameData->killSummary(
                $record->name,
                CommunityDataPolicy::profileRecentKillLimit(),
            ),
            'related_characters' => $relatedCharacters,
            'account_association_public' => $accountAssociationPublic,
            'status' => $status,
        ];
    }

    private function identityForCanaryAccount(int $accountId): ?Identity
    {
        $binding = IdentityCanaryAccount::query()
            ->where('canary_account_id', $accountId)
            ->where('status', IdentityCanaryAccount::STATUS_READY)
            ->first();

        if ($binding === null) {
            return null;
        }

        return Identity::query()->find($binding->identity_id);
    }

    private function timestamp(int $epoch): ?CarbonImmutable
    {
        return $epoch > 0 ? CarbonImmutable::createFromTimestampUTC($epoch) : null;
    }
}
