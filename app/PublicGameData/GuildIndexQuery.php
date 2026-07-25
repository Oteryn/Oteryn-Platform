<?php

namespace App\PublicGameData;

use Illuminate\Contracts\Pagination\LengthAwarePaginator;
use Illuminate\Support\Facades\DB;
use stdClass;

final class GuildIndexQuery
{
    private const PER_PAGE = 50;

    /**
     * @return LengthAwarePaginator<int, stdClass>
     */
    public function paginate(): LengthAwarePaginator
    {
        $activeMemberships = DB::connection('canary')
            ->table('guild_membership as membership')
            ->join('players as player', 'player.id', '=', 'membership.player_id')
            ->where('player.deletion', 0)
            ->groupBy('membership.guild_id')
            ->selectRaw('membership.guild_id, COUNT(*) as active_member_count');

        return DB::connection('canary')
            ->table('guilds as guild')
            ->leftJoinSub(
                $activeMemberships,
                'active_members',
                'active_members.guild_id',
                '=',
                'guild.id',
            )
            ->select(['guild.name'])
            ->selectRaw('COALESCE(active_members.active_member_count, 0) as active_member_count')
            ->orderBy('guild.name')
            ->orderBy('guild.id')
            ->paginate(self::PER_PAGE);
    }
}
