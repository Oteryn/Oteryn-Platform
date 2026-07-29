<?php

namespace App\PublicGameData;

use App\Accounts\Models\IdentityCanaryAccount;
use App\CharacterProfiles\Models\CharacterProfilePreference;
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
     *     status: array{online: bool, last_login: CarbonImmutable|null, last_logout: CarbonImmutable|null}|null,
     *     visibility: array{guild: bool, house: bool, skills: bool, deaths: bool, kills: bool},
     *     is_main_character: bool
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
        $playerId = $this->positiveInteger($record->id);
        if ($playerId === null) {
            return null;
        }

        $identity = $this->identityForCanaryAccount($record->account_id);
        $preference = null;
        /** @var Collection<int|string, CharacterProfilePreference> $preferencesByPlayer */
        $preferencesByPlayer = collect();
        /** @var Collection<int, stdClass> $relatedCharacters */
        $relatedCharacters = collect();
        $accountAssociationPublic = false;
        $status = null;

        if ($identity !== null && ! $identity->isTerminated()) {
            $preferencesByPlayer = CharacterProfilePreference::query()
                ->where('identity_id', $identity->id)
                ->get()
                ->keyBy('canary_player_id');
            $candidatePreference = $preferencesByPlayer->get($playerId);
            $preference = $candidatePreference instanceof CharacterProfilePreference ? $candidatePreference : null;

            $showAccountAssociation = ! $preference instanceof CharacterProfilePreference
                || $preference->show_account_association;
            $accountAssociationPublic = $identity->public_account_association && $showAccountAssociation;

            if ($accountAssociationPublic) {
                $relatedCharacters = $this->gameData->activeCharactersForAccount($record->account_id)
                    ->filter(function (stdClass $character) use ($playerId): bool {
                        $candidatePlayerId = $this->positiveInteger($character->id);

                        return $candidatePlayerId !== null && $candidatePlayerId !== $playerId;
                    })
                    ->filter(function (stdClass $character) use ($preferencesByPlayer): bool {
                        $candidatePlayerId = $this->positiveInteger($character->id);
                        if ($candidatePlayerId === null) {
                            return false;
                        }

                        $siblingPreference = $preferencesByPlayer->get($candidatePlayerId);

                        return ! $siblingPreference instanceof CharacterProfilePreference
                            || $siblingPreference->show_account_association;
                    })
                    ->take(CommunityDataPolicy::profileRelatedCharacterLimit())
                    ->values();
            }

            $showStatus = ! $preference instanceof CharacterProfilePreference || $preference->show_status;
            if ($identity->public_status_visible && $showStatus) {
                $status = [
                    'online' => $this->gameData->isCharacterOnline($playerId),
                    'last_login' => $this->timestamp($record->lastlogin),
                    'last_logout' => $this->timestamp($record->lastlogout),
                ];
            }
        }

        $visibility = [
            'guild' => ! $preference instanceof CharacterProfilePreference || $preference->show_guild,
            'house' => ! $preference instanceof CharacterProfilePreference || $preference->show_house,
            'skills' => ! $preference instanceof CharacterProfilePreference || $preference->show_skills,
            'deaths' => ! $preference instanceof CharacterProfilePreference || $preference->show_deaths,
            'kills' => ! $preference instanceof CharacterProfilePreference || $preference->show_kills,
        ];
        $comment = $preference instanceof CharacterProfilePreference
            ? trim((string) $preference->public_comment)
            : trim($record->comment);

        /** @var object{name: string, size: int}|null $house */
        $house = $visibility['house'] ? $this->gameData->houseForPlayer($playerId) : null;
        /** @var Collection<int, stdClass> $emptyEvents */
        $emptyEvents = collect();
        $deaths = $visibility['deaths']
            ? $this->gameData->deathsForPlayer($playerId, CommunityDataPolicy::profileDeathLimit())
            : $emptyEvents;
        $kills = $visibility['kills']
            ? $this->gameData->killSummary($record->name, CommunityDataPolicy::profileRecentKillLimit())
            : ['count' => 0, 'recent' => $emptyEvents];

        return [
            'character' => [
                'name' => $record->name,
                'level' => $record->level,
                'vocation' => $record->vocation,
                'magic_level' => $record->maglevel,
                'comment' => $comment,
                'boss_points' => $record->boss_points,
                'guild_name' => $visibility['guild'] && is_string($record->guild_name) && $record->guild_name !== '' ? $record->guild_name : null,
                'guild_rank' => $visibility['guild'] && is_string($record->guild_rank) && $record->guild_rank !== '' ? $record->guild_rank : null,
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
            'deaths' => $deaths,
            'kills' => $kills,
            'related_characters' => $relatedCharacters,
            'account_association_public' => $accountAssociationPublic,
            'status' => $status,
            'visibility' => $visibility,
            'is_main_character' => $preference instanceof CharacterProfilePreference
                && $preference->is_main_character,
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

    private function timestamp(int $epoch): ?CarbonImmutable
    {
        return $epoch > 0 ? CarbonImmutable::createFromTimestampUTC($epoch) : null;
    }
}
