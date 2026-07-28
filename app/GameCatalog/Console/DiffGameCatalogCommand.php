<?php

namespace App\GameCatalog\Console;

use App\GameCatalog\Application\Inspection\CatalogDiffService;
use Illuminate\Console\Command;
use Throwable;

final class DiffGameCatalogCommand extends Command
{
    protected $signature = 'game-catalog:diff {snapshot-a : Earlier snapshot ID} {snapshot-b : Later snapshot ID}';

    protected $description = 'Compare stable entity and relation hashes between two immutable snapshots.';

    public function handle(CatalogDiffService $diff): int
    {
        $from = filter_var($this->argument('snapshot-a'), FILTER_VALIDATE_INT, ['options' => ['min_range' => 1]]);
        $to = filter_var($this->argument('snapshot-b'), FILTER_VALIDATE_INT, ['options' => ['min_range' => 1]]);
        if (! is_int($from) || ! is_int($to)) {
            $this->error('Snapshot IDs must be positive integers.');

            return self::INVALID;
        }

        try {
            $result = $diff->diff($from, $to);
        } catch (Throwable $exception) {
            $this->error($exception->getMessage());

            return self::FAILURE;
        }

        $this->line(json_encode($result, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES));

        return self::SUCCESS;
    }
}
