<?php

namespace App\GameCatalog\Console;

use App\GameCatalog\Application\Inspection\CatalogProfileVerifier;
use Illuminate\Console\Command;
use Throwable;

final class VerifyGameCatalogCommand extends Command
{
    protected $signature = 'game-catalog:verify {--profile= : Active content profile key}';

    protected $description = 'Verify the active snapshot and complete visibility projection for one profile.';

    public function handle(CatalogProfileVerifier $verifier): int
    {
        $profile = $this->option('profile');
        if (! is_string($profile) || $profile === '') {
            $this->error('The --profile option is required.');

            return self::INVALID;
        }

        try {
            $result = $verifier->verify($profile);
        } catch (Throwable $exception) {
            $this->error($exception->getMessage());

            return self::FAILURE;
        }

        $this->line(json_encode($result, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES));

        return self::SUCCESS;
    }
}
