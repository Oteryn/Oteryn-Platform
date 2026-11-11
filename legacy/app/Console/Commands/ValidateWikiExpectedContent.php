<?php

namespace App\Console\Commands;

use App\Wiki\Content\WikiExpectedContentValidator;
use App\Wiki\Content\WikiLaunchContentCatalog;
use Illuminate\Console\Command;
use LogicException;

final class ValidateWikiExpectedContent extends Command
{
    protected $signature = 'wiki:launch-content:validate {--json : Emit the validated inventory summary as JSON}';

    protected $description = 'Validate the reviewed Wiki launch catalog against the authoritative expected-content inventory.';

    public function handle(
        WikiExpectedContentValidator $validator,
        WikiLaunchContentCatalog $catalog,
    ): int {
        try {
            $summary = $validator->validateCatalog($catalog);
        } catch (LogicException $exception) {
            if ($this->option('json')) {
                $this->line(json_encode([
                    'status' => 'FAIL',
                    'error' => $exception->getMessage(),
                ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES));
            } else {
                $this->components->error($exception->getMessage());
            }

            return self::FAILURE;
        }

        if ($this->option('json')) {
            $this->line(json_encode([
                'status' => 'PASS',
                ...$summary,
            ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES));
        } else {
            $this->components->info(sprintf(
                'Wiki expected content %s validates catalog %s: %d categories, %d articles, %d translations, %d source references, %d internal links.',
                $summary['inventory_version'],
                $summary['catalog_version'],
                $summary['categories'],
                $summary['articles'],
                $summary['category_translations'] + $summary['article_translations'],
                $summary['source_references'],
                $summary['internal_links'],
            ));
        }

        return self::SUCCESS;
    }
}
