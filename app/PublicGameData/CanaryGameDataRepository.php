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
    public function highscores(string $category = 'level', ?int $vocation = null, int $perPage = 50): LengthAwarePaginator
    {
        $scoreColumn = CommunityDataPolicy::highscoreColumn($category);
        $query = DB::connection('canary')
            ->table('players')
            ->select(['id', 'name', 'level', 'vocation'])
            ->selectRaw("{$scoreColumn} as score")
            ->where('deletion', 0);

        if ($vocation !== null) {
            $query->where('vocation', $vocation);
        }

        $players = $query
            ->orderByDesc($scoreColumn)
            ->orderByDesc('level')
            ->orderBy('name')
            ->paginate($perPage);
        $players->withQueryString();

        return $players;
    }

    /**
     * @deprecated Use highscores() with the level category.
     *
     * @return LengthAwarePaginator<int, stdClass>
     */
    public function levelHighscores(int $perPage = 50): LengthAwarePaginator
    {
        return $this->highscores('level', null, $perPage);
    }

    public function findActiveCharacter(string $name): ?object
    {
        return DB::connection('canary')
            ->table('players as player')
            ->leftJoin('guild_membership as membership', 'membership.player_id', '=', 'player.id')
            ->leftJoin('guilds as guild', 'guild.id', '=', 'membership.guild_id')
            ->leftJoin('guild_ranks as rank', 'rank.id', '=', 'membership.rank_id')
            ->select([
                'player.id',
                'player.name',
                'player.account_id',
                'player.level',
                'player.vocation',
                'player.maglevel',
                'player.lastlogin',
                'player.lastlogout',
                'player.comment',
                'player.boss_points',
                'player.skill_fist',
                'player.skill_club',
                'player.skill_sword',
                'player.skill_axe',
                'player.skill_dist',
                'player.skill_shielding',
                'player.skill_fishing',
                'guild.name as guild_name',
                'rank.name as guild_rank',
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
     * @return Collection<int, stdClass>
     */
    public function publicCharactersForAccount(int $accountId, int $exceptPlayerId, int $limit): Collection
    {
        return DB::connection('canary')
            ->table('players')
            ->select(['name', 'level', 'vocation'])
            ->where('account_id', $accountId)
            ->where('id', '!=', $exceptPlayerId)
            ->where('deletion', 0)
            ->orderBy('name')
            ->limit($limit)
            ->get();
    }

    public function houseForPlayer(int $playerId): ?object
    {
        return DB::connection('canary')
            ->table('houses')
            ->select(['name', 'size'])
            ->where('owner', $playerId)
            ->orderBy('channel_id')
            ->orderBy('id')
            ->first();
    }

    /**
     * @return Collection<int, stdClass>
     */
    public function deathsForPlayer(int $playerId, int $limit): Collection
    {
        return DB::connection('canary')
            ->table('player_deaths')
            ->select(['time', 'level', 'killed_by', 'is_player'])
            ->where('player_id', $playerId)
            ->orderByDesc('time')
            ->orderBy('killed_by')
            ->limit($limit)
            ->get();
    }

    /**
     * @return array{count: int, recent: Collection<int, stdClass>}
     */
    public function killSummary(string $characterName, int $limit): array
    {
        $query = DB::connection('canary')
            ->table('player_deaths as death')
            ->join('players as victim', 'victim.id', '=', 'death.player_id')
            ->where('death.is_player', 1)
            ->where('death.killed_by', $characterName)
            ->where('victim.deletion', 0);

        $count = (clone $query)->count();
        $recent = $query
            ->select([
                'victim.name as victim_name',
                'death.level as victim_level',
                'death.time',
            ])
            ->orderByDesc('death.time')
            ->orderBy('victim.name')
            ->limit($limit)
            ->get();

        return ['count' => $count, 'recent' => $recent];
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

    public function isCharacterOnline(int $playerId, ?int $readTimeEpochMs = null): bool
    {
        $readTimeEpochMs ??= (int) floor(microtime(true) * 1000);

        return DB::connection('canary')
            ->table('cluster_sessions')
            ->where('player_id', $playerId)
            ->where('status', 'ONLINE')
            ->where('expires_at', '>', $readTimeEpochMs)
            ->exists();
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
