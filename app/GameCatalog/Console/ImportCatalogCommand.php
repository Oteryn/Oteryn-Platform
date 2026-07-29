<?php

namespace App\GameCatalog\Console;

use App\GameCatalog\Application\Import\CatalogImportService;
use App\GameCatalog\Domain\Exceptions\CatalogValidationException;
use Illuminate\Console\Command;
use Throwable;

final class ImportCatalogCommand extends Command
{
    protected $signature = 'game-catalog:import
        {path : Path to a Game Catalog snapshot JSON file}
        {--sha256= : Expected lowercase SHA-256 of the input file}';

    protected $description = 'Import an inactive immutable Oteryn Game Catalog snapshot';

    public function handle(CatalogImportService $importer): int
    {
        try {
            $result = $importer->import(
                path: (string) $this->argument('path'),
                expectedSha256: $this->option('sha256') === null ? null : (string) $this->option('sha256'),
            );
        } catch (CatalogValidationException $exception) {
            $this->error('Game Catalog import rejected.');
            foreach ($exception->findings as $finding) {
                $this->line(sprintf(
                    '[%s] %s%s: %s',
                    strtoupper($finding->severity),
                    $finding->code,
                    $finding->path === null ? '' : ' '.$finding->path,
                    $finding->message,
                ));
            }

            return self::FAILURE;
        } catch (Throwable $exception) {
            report($exception);
            $this->error('Game Catalog import failed before publication. Public catalogue state was not changed.');

            return self::FAILURE;
        }

        $this->info($result->deduplicated
            ? 'An identical validated snapshot already exists; no duplicate catalogue state was created.'
            : 'Game Catalog snapshot imported and validated without activation.');
        $this->table(
            ['Snapshot ID', 'Import run ID', 'SHA-256', 'Activated'],
            [[$result->snapshotId, $result->importRunId, $result->contentSha256, 'no']],
        );

        return self::SUCCESS;
    }
}
