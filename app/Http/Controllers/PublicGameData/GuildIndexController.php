<?php

namespace App\Http\Controllers\PublicGameData;

use App\PublicGameData\GuildIndexQuery;
use Illuminate\Contracts\View\View;
use Illuminate\Database\QueryException;
use Symfony\Component\HttpKernel\Exception\ServiceUnavailableHttpException;

final class GuildIndexController
{
    public function __construct(private readonly GuildIndexQuery $guilds) {}

    public function __invoke(): View
    {
        try {
            $guilds = $this->guilds->paginate();
        } catch (QueryException $exception) {
            throw new ServiceUnavailableHttpException(
                null,
                'Guild data is temporarily unavailable.',
                $exception,
            );
        }

        return view('game.guilds.index', ['guilds' => $guilds]);
    }
}
