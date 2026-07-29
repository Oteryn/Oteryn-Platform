<?php

namespace App\PublicGameData;

use App\Accounts\Models\IdentityCanaryAccount;
use App\Identity\Models\Identity;
use Carbon\CarbonImmutable;
use Illuminate\Support\Collection;

final class PublicCharacterProfileService
{
    public function __construct(private readonly CanaryGameDataRepository $gameData) {}

    /**
     * @return array{
     *     character: array<string, mixed>,
     *     house: array<string, mixed>|null,
     *     deaths: Collection<int, object>,
     *     kills: array{count: int, recent: Collection<int, object>},
     *     related_characters: Collection<int, object>,
     *     status: array{online: bool, last_login: CarbonImmutable|null, last_logout: CarbonImmutable|null}|null
     * }|null
     */
    public function find(string $name): ?array
    {
        $record = $this->gameData->findActiveCharacter($name);

        if ($record === null) {
            return null;
        }

        $identity = $this->identityForCanaryAccount((int) $record->account_id);
        $relatedCharacters = collect();
        $status = null;

        if ($identity !== null && ! $identity->isTerminated()) {
            if ($identity->public_account_association) {
                $relatedCharacters = $this->gameData->publicCharactersForAccount(
                    (int) $record->account_id,
                    (int) $record->id,
                    CommunityDataPolicy::profileRelatedCharacterLimit(),
                );
            }

            if ($identity->public_status_visible) {
                $status = [
                    'online' => $this->gameData->isCharacterOnline((int) $record->id),
                    'last_login' => $this->timestamp((int) $record->lastlogin),
                    'last_logout' => $this->timestamp((int) $record->lastlogout),
                ];
            }
        }

        $house = $this->gameData->houseForPlayer((int) $record->id);

        return [
            'character' => [
                'name' => (string) $record->name,
                'level' => (int) $record->level,
                'vocation' => (int) $record->vocation,
                'magic_level' => (int) $record->maglevel,
                'comment' => trim((string) $record->comment),
                'boss_points' => (int) $record->boss_points,
                'guild_name' => is_string($record->guild_name) && $record->guild_name !== '' ? $record->guild_name : null,
                'guild_rank' => is_string($record->guild_rank) && $record->guild_rank !== '' ? $record->guild_rank : null,
                'skills' => [
                    'fist' => (int) $record->skill_fist,
                    'club' => (int) $record->skill_club,
                    'sword' => (int) $record->skill_sword,
                    'axe' => (int) $record->skill_axe,
                    'distance' => (int) $record->skill_dist,
                    'shielding' => (int) $record->skill_shielding,
                    'fishing' => (int) $record->skill_fishing,
                ],
            ],
            'house' => $house === null ? null : [
                'name' => (string) $house->name,
                'size' => (int) $house->size,
            ],
            'deaths' => $this->gameData->deathsForPlayer(
                (int) $record->id,
                CommunityDataPolicy::profileDeathLimit(),
            ),
            'kills' => $this->gameData->killSummary(
                (string) $record->name,
                CommunityDataPolicy::profileRecentKillLimit(),
            ),
            'related_characters' => $relatedCharacters,
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
