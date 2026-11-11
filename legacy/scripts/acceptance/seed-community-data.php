<?php

declare(strict_types=1);

use App\Accounts\Models\IdentityCanaryAccount;
use App\CharacterProfiles\Models\CharacterProfilePreference;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\Hash;

require dirname(__DIR__, 2).'/vendor/autoload.php';

$app = require dirname(__DIR__, 2).'/bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$email = 'community-acceptance@example.test';
$password = 'acceptance-community-not-a-user-password';
$existingIdentity = Identity::query()->where('email', $email)->first();
$passwordHash = $existingIdentity !== null
    && Hash::check($password, (string) $existingIdentity->password)
        ? (string) $existingIdentity->password
        : Hash::make($password);

$identity = Identity::query()->updateOrCreate(
    ['email' => $email],
    [
        'password' => $passwordHash,
        'public_account_association' => true,
        'public_status_visible' => true,
    ],
);

$identity->forceFill([
    'disabled_at' => null,
    'terminated_at' => null,
    'termination_requested_at' => null,
    'termination_scheduled_for' => null,
])->save();

IdentityCanaryAccount::query()->updateOrCreate(
    ['identity_id' => $identity->id],
    [
        'canary_account_id' => 9001,
        'provisioning_name' => 'community-acceptance',
        'canary_creation_epoch' => 1,
        'status' => IdentityCanaryAccount::STATUS_READY,
        'last_failure_code' => null,
        'last_attempt_at' => now(),
        'ready_at' => now(),
    ],
);

CharacterProfilePreference::query()
    ->where('identity_id', $identity->id)
    ->delete();

fwrite(STDOUT, json_encode([
    'identity_id' => $identity->id,
    'canary_account_id' => 9001,
    'public_account_association' => true,
    'public_status_visible' => true,
    'character_profile_preferences_reset' => true,
], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES).PHP_EOL);
