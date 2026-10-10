<?php

namespace App\Console\Commands;

use App\GameAuth\OAuth\NativeOAuthClientManager;
use Illuminate\Console\Command;
use LogicException;

final class EnsureNativeOAuthClient extends Command
{
    protected $signature = 'game-auth:oauth-client:ensure {--rust : Ensure the separate native Rust client}';

    protected $description = 'Ensure the first-party public Oteryn native OAuth client exists.';

    public function handle(NativeOAuthClientManager $clients): int
    {
        $client = $this->option('rust') ? $clients->ensureRust() : $clients->ensure();
        $clientId = $client->getKey();

        if (! is_string($clientId) && ! is_int($clientId)) {
            throw new LogicException('Oteryn native OAuth client id is not scalar.');
        }

        $this->components->info('Oteryn native OAuth client id: '.(string) $clientId);

        return self::SUCCESS;
    }
}
