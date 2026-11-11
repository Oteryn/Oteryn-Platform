<?php

namespace App\GameCatalog\Console;

use App\GameCatalog\Application\Import\CatalogSnapshotValidator;
use App\GameCatalog\Domain\Exceptions\CatalogValidationException;
use Illuminate\Console\Command;

final class ValidateCatalogCommand extends Command
{
    protected $signature = 'game-catalog:validate
        {path : Path to a Game Catalog snapshot JSON file}
        {--sha256= : Expected lowercase SHA-256 of the input file}';

    protected $description = 'Validate an Oteryn Game Catalog snapshot without importing or activating it';

    public function handle(CatalogSnapshotValidator $validator): int
    {
        try {
            $validated = $validator->validate(
                path: (string) $this->argument('path'),
                expectedSha256: $this->option('sha256') === null ? null : (string) $this->option('sha256'),
            );
        } catch (CatalogValidationException $exception) {
            $this->error('Game Catalog validation failed.');
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
        }

        $this->info('Game Catalog snapshot is valid.');
        $this->table(
            ['Contract', 'Schema', 'SHA-256', 'Entities', 'Relations'],
            [[
                $validated->payload['contract'],
                $validated->payload['schema_version'],
                $validated->contentSha256,
                count($validated->payload['entities']),
                count($validated->payload['relations']),
            ]],
        );

        return self::SUCCESS;
    }
}
