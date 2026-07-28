<?php

namespace App\GameCatalog\Console;

use App\GameCatalog\Validation\CatalogValidationException;
use App\GameCatalog\Validation\GameCatalogDocumentValidator;
use Illuminate\Console\Command;

final class ValidateGameCatalogCommand extends Command
{
    protected $signature = 'game-catalog:validate {path : Snapshot JSON path} {--sha256= : Expected lowercase SHA-256}';

    protected $description = 'Validate an Oteryn Game Catalog snapshot without importing it.';

    public function handle(GameCatalogDocumentValidator $validator): int
    {
        try {
            $validated = $validator->validatePath((string) $this->argument('path'), $this->stringOption('sha256'));
        } catch (CatalogValidationException $exception) {
            foreach ($exception->findings() as $finding) {
                $this->error($finding['code'].' '.$finding['path'].' '.$finding['message']);
            }

            return self::FAILURE;
        }

        $this->line(json_encode([
            'contract' => $validated->document['contract'],
            'schema_version' => $validated->document['schema_version'],
            'sha256' => $validated->contentSha256,
            'bytes' => $validated->byteSize,
            'entity_count' => $validated->document['snapshot']['entity_count'],
            'relation_count' => $validated->document['snapshot']['relation_count'],
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES));

        return self::SUCCESS;
    }

    private function stringOption(string $name): ?string
    {
        $value = $this->option($name);

        return is_string($value) && $value !== '' ? $value : null;
    }
}
