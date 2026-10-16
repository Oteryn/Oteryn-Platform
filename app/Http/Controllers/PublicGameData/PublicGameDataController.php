<?php

namespace App\Http\Controllers\PublicGameData;

use App\PublicGameData\CanaryGameDataRepository;
use App\PublicGameData\CommunityDataPolicy;
use App\PublicGameData\NativePublicGameData;
use App\PublicGameData\PublicGameDataSource;
use App\PublicGameData\PublicGameDataUnavailable;
use Illuminate\Contracts\View\View;
use Illuminate\Database\QueryException;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Http\Response;
use Illuminate\Pagination\LengthAwarePaginator;
use Illuminate\Validation\Rule;

final class PublicGameDataController
{
    public function __construct(
        private readonly CanaryGameDataRepository $gameData,
        private readonly NativePublicGameData $native,
    ) {}

    public function highscores(Request $request): View|Response
    {
        /** @var array{category?: mixed, vocation?: mixed, scope?: mixed} $validated */
        $validated = $request->validate([
            'category' => ['nullable', 'string', Rule::in(CommunityDataPolicy::highscoreCategories())],
            'vocation' => ['nullable', 'integer', Rule::in(CommunityDataPolicy::vocationIds())],
            'scope' => ['nullable', 'string', Rule::in(['global'])],
        ]);
        $category = is_string($validated['category'] ?? null) ? $validated['category'] : 'level';
        $vocationInput = $validated['vocation'] ?? null;
        $vocation = is_int($vocationInput)
            ? $vocationInput
            : (is_string($vocationInput) && ctype_digit($vocationInput) ? (int) $vocationInput : null);

        // Native projections are the only source; a stale or missing family answers 503.
        try {
            $players = $this->paginate($request, $this->native->highscores($category, $vocation, now()->getTimestamp()), 50);
        } catch (PublicGameDataUnavailable|QueryException) {
            return $this->unavailable('community.highscores.title');
        }

        return view('game.highscores', [
            'players' => $players,
            'category' => $category,
            'vocation' => $vocation,
            'categories' => CommunityDataPolicy::highscoreCategories(),
            'vocations' => CommunityDataPolicy::highscoreVocationIds(),
        ]);
    }

    public function characterSearch(Request $request): RedirectResponse
    {
        /** @var array{name?: mixed} $validated */
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
        // The native character_profile family is the only source; a stale or missing family
        // answers 503 and an unknown or hidden character 404.
        try {
            $character = $this->native->profile($name, now()->getTimestamp());
        } catch (PublicGameDataUnavailable|QueryException) {
            return $this->unavailable('community.profile.title');
        }

        abort_if($character === null, 404);

        return view('game.character', ['character' => $character]);
    }

    public function deaths(): View|Response
    {
        try {
            PublicGameDataSource::assertAvailable();
            $deaths = $this->gameData->latestDeaths();
        } catch (PublicGameDataUnavailable) {
            return $this->unavailable('community.deaths.title', Response::HTTP_OK);
        } catch (QueryException) {
            return $this->unavailable('community.deaths.title');
        }

        return view('game.deaths', ['deaths' => $deaths]);
    }

    public function guild(string $name): View|Response
    {
        try {
            PublicGameDataSource::assertAvailable();
            $result = $this->gameData->findGuild($name);
        } catch (PublicGameDataUnavailable) {
            return $this->unavailable('community.guilds.title', Response::HTTP_OK);
        } catch (QueryException) {
            return $this->unavailable('community.guilds.title');
        }

        abort_if($result === null, 404);

        return view('game.guild', $result);
    }

    public function servers(): View|Response
    {
        try {
            PublicGameDataSource::assertAvailable();
            $channels = $this->gameData->configuredChannels();
        } catch (PublicGameDataUnavailable) {
            return $this->unavailable('public.game.servers_title', Response::HTTP_OK);
        } catch (QueryException) {
            return $this->unavailable('public.game.servers_title');
        }

        return view('game.servers', ['channels' => $channels]);
    }

    public function online(Request $request): View|Response
    {
        try {
            $characters = $this->paginate($request, $this->native->online(now()->getTimestamp()), 100);
        } catch (PublicGameDataUnavailable|QueryException) {
            return $this->unavailable('public.game.online_title');
        }

        return view('game.online', ['characters' => $characters]);
    }

    /**
     * @param  list<object>  $items
     * @return LengthAwarePaginator<int, object>
     */
    private function paginate(Request $request, array $items, int $perPage): LengthAwarePaginator
    {
        // Clamp to the last page so an oversized ?page= cannot overflow the slice offset.
        $lastPage = max(1, intdiv(count($items) + $perPage - 1, $perPage));
        $page = min($lastPage, max(1, $request->integer('page', 1)));

        return (new LengthAwarePaginator(
            array_slice($items, ($page - 1) * $perPage, $perPage),
            count($items),
            $perPage,
            $page,
            ['path' => $request->url()],
        ))->withQueryString();
    }

    private function unavailable(string $titleKey, int $status = Response::HTTP_SERVICE_UNAVAILABLE): Response
    {
        return response()->view('game.unavailable', [
            'title' => __($titleKey),
        ], $status);
    }
}
