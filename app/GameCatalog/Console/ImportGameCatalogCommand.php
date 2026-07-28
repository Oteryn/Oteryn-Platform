<?php

namespace App\GameCatalog\Console;

use App\GameCatalog\Application\Import\CatalogImporter;
use App\GameCatalog\Validation\CatalogValidationException;
use Illuminate\Console\Command;
use Throwable;

final class ImportGameCatalogCommand extends Command
{
    protected $signature = 'game-catalog:import {path : Snapshot JSON path} {--sha256= : Expected lowercase SHA-256}';

    protected $description = 'Import a validated snapshot as inactive immutable Game Catalog state.';

    public function handle(CatalogImporter $importer): int
    {
        $expected = $this->option('sha256');

        try {
            $result = $importer->import(
                (string) $this->argument('path'),
                is_string($expected) && $expected !== '' ? $expected : null,
            );
        } catch (CatalogValidationException $exception) {
            foreach ($exception->findings() as $finding) {
                $this->error($finding['code'].' '.$finding['path'].' '.$finding['message']);
            }

            return self::FAILURE;
        } catch (Throwable $exception) {
            $this->error($exception->getMessage());

            return self::FAILURE;
        }

        $this->line(json_encode([
            'snapshot_id' => $result->snapshotId,
            'import_run_id' => $result->importRunId,
            'sha256' => $result->contentSha256,
            'already_imported' => $result->alreadyImported,
            'activated' => false,
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES));

        return self::SUCCESS;
    }
}
