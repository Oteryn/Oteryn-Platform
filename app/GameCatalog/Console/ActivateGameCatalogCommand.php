<?php

namespace App\GameCatalog\Console;

use App\GameCatalog\Application\Profiles\CatalogProfileActivator;
use Illuminate\Console\Command;
use Throwable;

final class ActivateGameCatalogCommand extends Command
{
    protected $signature = 'game-catalog:activate {snapshot-id : Validated snapshot ID} {--profile= : Content profile key} {--rollback : Record this activation as rollback} {--reason= : Bounded audit reason}';

    protected $description = 'Activate or roll back to a validated snapshot for exactly one profile.';

    public function handle(CatalogProfileActivator $activator): int
    {
        $profile = $this->option('profile');
        if (! is_string($profile) || $profile === '') {
            $this->error('The --profile option is required.');

            return self::INVALID;
        }
        $snapshotId = filter_var($this->argument('snapshot-id'), FILTER_VALIDATE_INT, ['options' => ['min_range' => 1]]);
        if (! is_int($snapshotId)) {
            $this->error('snapshot-id must be a positive integer.');

            return self::INVALID;
        }
        $reason = $this->option('reason');

        try {
            $result = $activator->activate(
                $snapshotId,
                $profile,
                null,
                $this->option('rollback') ? 'rollback' : 'activate',
                is_string($reason) && $reason !== '' ? $reason : null,
            );
        } catch (Throwable $exception) {
            $this->error($exception->getMessage());

            return self::FAILURE;
        }

        $this->line(json_encode([
            'profile_id' => $result->profileId,
            'previous_snapshot_id' => $result->previousSnapshotId,
            'active_snapshot_id' => $result->activeSnapshotId,
            'visible_entities' => $result->projection->visibleEntities,
            'hidden_entities' => $result->projection->hiddenEntities,
            'visible_relations' => $result->projection->visibleRelations,
            'hidden_relations' => $result->projection->hiddenRelations,
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES));

        return self::SUCCESS;
    }
}
