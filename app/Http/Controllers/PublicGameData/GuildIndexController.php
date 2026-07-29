<?php

namespace App\Http\Controllers\PublicGameData;

use App\PublicGameData\CommunityDataPolicy;
use App\PublicGameData\GuildIndexQuery;
use Illuminate\Contracts\View\View;
use Illuminate\Database\QueryException;
use Illuminate\Http\Request;
use Illuminate\Http\Response;

final class GuildIndexController
{
    public function __construct(private readonly GuildIndexQuery $guilds) {}

    public function __invoke(Request $request): View|Response
    {
        $validated = $request->validate([
            'q' => ['nullable', 'string', 'max:'.CommunityDataPolicy::guildSearchLimit()],
        ]);
        $search = is_string($validated['q'] ?? null) ? trim($validated['q']) : null;
        $search = $search === '' ? null : $search;

        try {
            $guilds = $this->guilds->paginate($search);
        } catch (QueryException) {
            return response()->view('game.unavailable', [
                'title' => __('community.guilds.title'),
            ], 503);
        }

        return view('game.guilds.index', [
            'guilds' => $guilds,
            'search' => $search,
        ]);
    }
}
