<?php

namespace App\GameCatalog;

use App\GameCatalog\Application\Import\CatalogImportService;
use App\GameCatalog\Application\Import\CatalogSemanticValidator;
use App\GameCatalog\Application\Import\CatalogSnapshotValidator;
use App\GameCatalog\Console\ImportCatalogCommand;
use App\GameCatalog\Console\ValidateCatalogCommand;
use App\GameCatalog\Infrastructure\Json\BundledJsonSchemaValidator;
use App\GameCatalog\Infrastructure\Json\DuplicateJsonKeyDetector;
use Illuminate\Contracts\Foundation\Application;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\ServiceProvider;

final class GameCatalogServiceProvider extends ServiceProvider
{
    public function register(): void
    {
        $this->mergeConfigFrom(base_path('config/game-catalog.php'), 'game-catalog');

        $this->app->singleton(DuplicateJsonKeyDetector::class, fn (): DuplicateJsonKeyDetector => new DuplicateJsonKeyDetector(128));
        $this->app->singleton(BundledJsonSchemaValidator::class, fn (): BundledJsonSchemaValidator => new BundledJsonSchemaValidator(
            maximumFindings: (int) config('game-catalog.limits.validation_findings', 2_000),
        ));
        $this->app->singleton(CatalogSemanticValidator::class);
        $this->app->singleton(CatalogSnapshotValidator::class);
        $this->app->singleton(CatalogImportService::class, function (Application $app): CatalogImportService {
            return new CatalogImportService(
                validator: $app->make(CatalogSnapshotValidator::class),
                database: DB::connection(),
            );
        });
    }

    public function boot(): void
    {
        if ($this->app->runningInConsole()) {
            $this->commands([
                ValidateCatalogCommand::class,
                ImportCatalogCommand::class,
            ]);
        }
    }
}
