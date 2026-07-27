<?php

declare(strict_types=1);

use App\Accounts\Actions\ProvisionCanaryAccount;
use App\Accounts\Models\IdentityCanaryAccount;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;

require __DIR__.'/../../vendor/autoload.php';

$app = require __DIR__.'/../../bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

if (! $app->environment('acceptance')) {
    fwrite(STDERR, "Account overview fixture seeding is restricted to the acceptance environment.\n");
    exit(2);
}

$command = $argv[1] ?? '';
$email = $argv[2] ?? '';

$fail = static function (string $message, int $code = 1): never {
    fwrite(STDERR, $message.PHP_EOL);
    exit($code);
};

$json = static function (array $payload): never {
    fwrite(STDOUT, json_encode($payload, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES).PHP_EOL);
    exit(0);
};

if ($email === '') {
    $fail('Usage: php scripts/acceptance/seed-account-overview-state.php <seed|binding> <email> [ready|pending|recoverable|conflict|missing]', 2);
}

$identity = Identity::query()->where('email', $email)->first();
if (! $identity instanceof Identity) {
    $fail("Acceptance identity {$email} does not exist. Register it through the browser before changing Account Overview state.");
}

if ($command === 'binding') {
    $binding = IdentityCanaryAccount::query()->whereKey($identity->id)->first();

    $json([
        'email' => $identity->email,
        'status' => $binding?->status ?? 'missing',
        'canary_account_id' => $binding?->canary_account_id,
        'provisioning_name' => $binding?->provisioning_name,
    ]);
}

if ($command !== 'seed') {
    $fail('Unknown Account Overview fixture command.', 2);
}

$state = $argv[3] ?? '';
if (! in_array($state, ['ready', 'pending', 'recoverable', 'conflict', 'missing'], true)) {
    $fail('Seed state must be ready, pending, recoverable, conflict or missing.', 2);
}

if ($state === 'missing') {
    IdentityCanaryAccount::query()->whereKey($identity->id)->delete();

    $json([
        'email' => $identity->email,
        'state' => $state,
        'canary_account_id' => null,
        'provisioning_name' => null,
    ]);
}

$provisioningName = 'op'.substr(hash('sha256', 'account-overview-'.$email), 0, 30);
$creationEpoch = 2_000_100_000 + $identity->id;
$accountId = 3_000_000_000 + $identity->id;

$attributes = [
    'canary_account_id' => null,
    'provisioning_name' => $provisioningName,
    'canary_creation_epoch' => $creationEpoch,
    'status' => IdentityCanaryAccount::STATUS_PENDING,
    'last_failure_code' => null,
    'last_attempt_at' => now()->subMinute(),
    'ready_at' => null,
];

if ($state === 'ready') {
    $attributes['canary_account_id'] = $accountId;
    $attributes['status'] = IdentityCanaryAccount::STATUS_READY;
    $attributes['ready_at'] = now()->subMinute();
} elseif ($state === 'recoverable') {
    $attributes['last_failure_code'] = ProvisionCanaryAccount::FAILURE_DEPENDENCY_UNAVAILABLE;
} elseif ($state === 'conflict') {
    $attributes['status'] = IdentityCanaryAccount::STATUS_CONFLICT;
    $attributes['last_failure_code'] = ProvisionCanaryAccount::FAILURE_BINDING_CONFLICT;
}

IdentityCanaryAccount::query()->updateOrCreate(
    ['identity_id' => $identity->id],
    $attributes,
);

$json([
    'email' => $identity->email,
    'state' => $state,
    'canary_account_id' => $state === 'ready' ? $accountId : null,
    'provisioning_name' => $provisioningName,
]);
