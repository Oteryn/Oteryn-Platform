<?php

declare(strict_types=1);

use App\Identity\Models\Identity;
use App\ProductsEntitlements\Premium\PremiumTimeLedger;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Foundation\Application;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;

require __DIR__.'/../../../vendor/autoload.php';

/** @var Application $app */
$app = require __DIR__.'/../../../bootstrap/app.php';
/** @var Kernel $kernel */
$kernel = $app->make(Kernel::class);
$kernel->bootstrap();

if (! $app->environment('testing')) {
    fwrite(STDERR, "Premium physical E2E control is restricted to testing.\n");
    exit(2);
}

$mode = $argv[1] ?? '';
$actorEmail = 'premium-e2e-operator@example.invalid';
$targetEmail = 'premium-e2e-target@example.invalid';

$identity = static function (string $email): Identity {
    return Identity::query()->updateOrCreate(
        ['email' => $email],
        ['password' => Hash::make(Str::random(32))],
    );
};

$actor = $identity($actorEmail);
$target = $identity($targetEmail);

if ($mode === 'seed') {
    fwrite(STDOUT, (string) $target->account_id."\n");
    exit(0);
}

/** @var PremiumTimeLedger $ledger */
$ledger = $app->make(PremiumTimeLedger::class);
if ($mode === 'grant') {
    $ledger->grant(
        $actor,
        $target,
        1,
        'Premium physical E2E operator test grant',
        strtolower((string) Str::uuid7()),
    );
    fwrite(STDOUT, "granted\n");
    exit(0);
}

if ($mode === 'revoke') {
    $ledger->revoke(
        $actor,
        $target,
        'Premium physical E2E operator test revocation',
        strtolower((string) Str::uuid7()),
    );
    fwrite(STDOUT, "revoked\n");
    exit(0);
}

fwrite(STDERR, "Usage: php control.php seed|grant|revoke\n");
exit(2);
