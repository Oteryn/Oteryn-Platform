<?php

namespace App\PublicGameData;

use Illuminate\Contracts\Pagination\LengthAwarePaginator;
use Illuminate\Support\Collection;
use Illuminate\Support\Facades\DB;
use stdClass;

final class CanaryGameDataRepository
{
    /**
     * @return Collection<int, stdClass>
     */
    public function activeCharactersForAccount(int $accountId): Collection
    {
        return DB::connection('canary')
            ->table('players')
            ->select(['id', 'name', 'level', 'vocation'])
            ->where('account_id', $accountId)
            ->where('deletion', 0)
            ->orderBy('name')
            ->get();
    }

    /**
     * @return LengthAwarePaginator<int, stdClass>
     */
    public function latestDeaths(int $perPage = 50): LengthAwarePaginator
    {
        return DB::connection('canary')
            ->table('player_deaths as death')
            ->join('players as player', 'player.id', '=', 'death.player_id')
            ->select([
                'player.name as player_name',
                'death.time',
                'death.level',
                'death.killed_by',
                'death.is_player',
            ])
            ->where('player.deletion', 0)
            ->orderByDesc('death.time')
            ->orderBy('player.name')
            ->orderBy('death.killed_by')
            ->paginate($perPage);
    }

    /**
     * @return array{guild: stdClass, members: LengthAwarePaginator<int, stdClass>}|null
     */
    public function findGuild(string $name, int $perPage = 50): ?array
    {
        $guild = DB::connection('canary')
            ->table('guilds as guild')
            ->leftJoin('players as owner', 'owner.id', '=', 'guild.ownerid')
            ->select([
                'guild.id',
                'guild.name',
                'guild.level',
                'guild.creationdata',
                'guild.motd',
                'guild.points',
                'owner.name as owner_name',
            ])
            ->where('guild.name', $name)
            ->first();

        if ($guild === null) {
            return null;
        }

        $members = DB::connection('canary')
            ->table('guild_membership as membership')
            ->join('players as player', 'player.id', '=', 'membership.player_id')
            ->join('guild_ranks as rank', 'rank.id', '=', 'membership.rank_id')
            ->select([
                'player.name',
                'player.level',
                'player.vocation',
                'membership.nick',
                'rank.name as rank_name',
                'rank.level as rank_level',
            ])
            ->where('membership.guild_id', $guild->id)
            ->where('player.deletion', 0)
            ->orderByDesc('rank.level')
            ->orderBy('player.name')
            ->paginate($perPage);
        $members->withQueryString();

        return ['guild' => $guild, 'members' => $members];
    }

    /**
     * @return Collection<int, stdClass>
     */
    public function configuredChannels(): Collection
    {
        return DB::connection('canary')
            ->table('channels')
            ->select(['id', 'name', 'pvp_type', 'max_players', 'maintenance', 'maintenance_message'])
            ->where('enabled', 1)
            ->orderBy('sort_order')
            ->orderBy('id')
            ->get();
    }
}
