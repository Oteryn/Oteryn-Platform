<?php

namespace App\Http\Controllers\PublicGameData;

use App\PublicGameData\CanaryChannelRuntimeService;
use App\PublicGameData\CanaryGameDataRepository;
use App\PublicGameData\CommunityDataPolicy;
use App\PublicGameData\PublicCharacterProfileService;
use Illuminate\Contracts\View\View;
use Illuminate\Database\QueryException;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Http\Response;
use Illuminate\Validation\Rule;
use UnexpectedValueException;

final class PublicGameDataController
{
    public function __construct(
        private readonly CanaryGameDataRepository $gameData,
        private readonly CanaryChannelRuntimeService $runtime,
        private readonly PublicCharacterProfileService $profiles,
    ) {}

    public function highscores(Request $request): View|Response
    {
        $validated = $request->validate([
            'category' => ['nullable', 'string', Rule::in(CommunityDataPolicy::highscoreCategories())],
            'vocation' => ['nullable', 'integer', Rule::in(CommunityDataPolicy::vocationIds())],
            'scope' => ['nullable', 'string', Rule::in(['global'])],
        ]);
        $category = is_string($validated['category'] ?? null) ? $validated['category'] : 'level';
        $vocation = is_int($validated['vocation'] ?? null) ? $validated['vocation'] : null;

        try {
            $players = $this->gameData->highscores($category, $vocation);
        } catch (QueryException) {
            return $this->unavailable('community.highscores.title');
        }

        return view('game.highscores', [
            'players' => $players,
            'category' => $category,
            'vocation' => $vocation,
            'categories' => CommunityDataPolicy::highscoreCategories(),
            'vocations' => CommunityDataPolicy::vocationIds(),
        ]);
    }

    public function characterSearch(Request $request): RedirectResponse
    {
        $validated = $request->validate([
            'name' => ['required', 'string', 'max:255'],
        ]);
        $name = $validated['name'] ?? null;

        if (! is_string($name)) {
            abort(422);
        }

        return redirect()->route('game.characters.show', [
            'name' => trim($name),
        ]);
    }

    public function character(string $name): View|Response
    {
        try {
            $profile = $this->profiles->find($name);
        } catch (QueryException) {
            return $this->unavailable('community.profile.title');
        }

        abort_if($profile === null, 404);

        return view('game.character', $profile);
    }

    public function deaths(): View|Response
    {
        try {
            $deaths = $this->gameData->latestDeaths();
        } catch (QueryException) {
            return $this->unavailable('community.deaths.title');
        }

        return view('game.deaths', ['deaths' => $deaths]);
    }

    public function guild(string $name): View|Response
    {
        try {
            $result = $this->gameData->findGuild($name);
        } catch (QueryException) {
            return $this->unavailable('community.guilds.title');
        }

        abort_if($result === null, 404);

        return view('game.guild', $result);
    }

    public function servers(): View|Response
    {
        try {
            $channels = $this->gameData->configuredChannels();
        } catch (QueryException) {
            return $this->unavailable('public.game.servers_title');
        }

        /** @var list<int> */
        $channelIds = $channels
            ->pluck('id')
            ->map(static function (mixed $channelId): int {
                if (is_int($channelId)) {
                    return $channelId;
                }

                if (is_string($channelId) && ctype_digit($channelId)) {
                    $parsedChannelId = (int) $channelId;

                    if ($parsedChannelId > 0) {
                        return $parsedChannelId;
                    }
                }

                throw new UnexpectedValueException('Configured Canary channel ID is invalid.');
            })
            ->values()
            ->all();

        return view('game.servers', [
            'channels' => $channels,
            'runtimeSnapshot' => $this->runtime->snapshot($channelIds),
        ]);
    }

    public function online(): View|Response
    {
        try {
            $characters = $this->gameData->onlineCharacters();
        } catch (QueryException) {
            return $this->unavailable('public.game.online_title');
        }

        return view('game.online', ['characters' => $characters]);
    }

    private function unavailable(string $titleKey): Response
    {
        return response()->view('game.unavailable', [
            'title' => __($titleKey),
        ], 503);
    }
}
