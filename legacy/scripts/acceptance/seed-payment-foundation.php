<?php

declare(strict_types=1);

use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\Hash;

require __DIR__.'/../../vendor/autoload.php';

$app = require __DIR__.'/../../bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

if (! $app->environment('acceptance')) {
    fwrite(STDERR, "Payment acceptance fixture seeding is restricted to the acceptance environment.\n");
    exit(2);
}

$email = $argv[1] ?? '';
$password = $argv[2] ?? '';

if ($email === '' || $password === '') {
    fwrite(STDERR, "Usage: php scripts/acceptance/seed-payment-foundation.php <email> <password>\n");
    exit(2);
}
$identity = Identity::query()->updateOrCreate(
    ['email' => $email],
    ['password' => Hash::make($password)],
);
$identity->forceFill([
    'web_session_generation' => 0,
    'disabled_at' => null,
    'two_factor_secret' => null,
    'two_factor_recovery_codes' => null,
    'two_factor_confirmed_at' => null,
    'two_factor_last_used_timestep' => null,
])->save();

fwrite(STDOUT, json_encode([
    'identity_id' => $identity->id,
    'email' => $identity->email,
], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES).PHP_EOL);
