<?php

namespace App\GameCatalog;

use App\GameCatalog\Console\ActivateGameCatalogCommand;
use App\GameCatalog\Console\DiffGameCatalogCommand;
use App\GameCatalog\Console\ImportGameCatalogCommand;
use App\GameCatalog\Console\ValidateGameCatalogCommand;
use App\GameCatalog\Console\VerifyGameCatalogCommand;
use Illuminate\Support\ServiceProvider;

final class GameCatalogServiceProvider extends ServiceProvider
{
    public function register(): void {}

    public function boot(): void
    {
        if ($this->app->runningInConsole()) {
            $this->commands([
                ValidateGameCatalogCommand::class,
                ImportGameCatalogCommand::class,
                ActivateGameCatalogCommand::class,
                DiffGameCatalogCommand::class,
                VerifyGameCatalogCommand::class,
            ]);
        }
    }
}
