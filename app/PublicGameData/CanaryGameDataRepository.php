<?php

namespace App\PublicGameData;

use Illuminate\Contracts\Pagination\LengthAwarePaginator;
use Illuminate\Support\Collection;
use Illuminate\Support\Facades\DB;
use stdClass;

final class CanaryGameDataRepository
{
    /**
     * @return LengthAwarePaginator<int, stdClass>
     */
    public function levelHighscores(int $perPage = 50): LengthAwarePaginator
    {
        return DB::connection('canary')
            ->table('players')
            ->select(['id', 'name', 'level', 'vocation'])
            ->where('deletion', 0)
            ->orderByDesc('level')
            ->orderBy('name')
            ->paginate($perPage);
    }

    public function findActiveCharacter(string $name): ?object
    {
        return DB::connection('canary')
            ->table('players as player')
            ->leftJoin('guild_membership as membership', 'membership.player_id', '=', 'player.id')
            ->leftJoin('guilds as guild', 'guild.id', '=', 'membership.guild_id')
            ->select([
                'player.id',
                'player.name',
                'player.level',
                'player.vocation',
                'guild.name as guild_name',
            ])
            ->where('player.deletion', 0)
            ->where('player.name', $name)
            ->first();
    }

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
     * @return array{guild: stdClass, members: LengthAwarePaginator<int, stdClass>}|null
     */
    public function findGuild(string $name, int $perPage = 50): ?array
    {
        $guild = DB::connection('canary')
            ->table('guilds')
            ->select(['id', 'name', 'ownerid', 'level', 'creationdata', 'motd', 'residence', 'points'])
            ->where('name', $name)
            ->first();

        if ($guild === null) {
            return null;
        }

        $members = DB::connection('canary')
            ->table('guild_membership as membership')
            ->join('players as player', 'player.id', '=', 'membership.player_id')
            ->join('guild_ranks as rank', 'rank.id', '=', 'membership.rank_id')
            ->select([
                'player.id',
                'player.name',
                'player.level',
                'player.vocation',
                'membership.nick',
                'rank.id as rank_id',
                'rank.name as rank_name',
                'rank.level as rank_level',
            ])
            ->where('membership.guild_id', $guild->id)
            ->where('player.deletion', 0)
            ->orderByDesc('rank.level')
            ->orderBy('player.name')
            ->paginate($perPage);

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

    /**
     * @return LengthAwarePaginator<int, stdClass>
     */
    public function onlineCharacters(?int $readTimeEpochMs = null, int $perPage = 100): LengthAwarePaginator
    {
        $readTimeEpochMs ??= (int) floor(microtime(true) * 1000);

        return DB::connection('canary')
            ->table('cluster_sessions as session')
            ->join('players as player', 'player.id', '=', 'session.player_id')
            ->join('channels as channel', 'channel.id', '=', 'session.channel_id')
            ->select([
                'player.id',
                'player.name',
                'player.level',
                'player.vocation',
                'session.channel_id as channel_id',
                'channel.name as channel_name',
            ])
            ->where('session.status', 'ONLINE')
            ->where('session.expires_at', '>', $readTimeEpochMs)
            ->where('player.deletion', 0)
            ->orderBy('channel.sort_order')
            ->orderBy('channel.id')
            ->orderBy('player.name')
            ->paginate($perPage);
    }
}
