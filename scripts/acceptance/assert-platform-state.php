<?php

declare(strict_types=1);

use App\Accounts\Models\IdentityCanaryAccount;
use App\Admin\AdminAuthorization;
use App\Identity\Mfa\MfaRecoveryCodes;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;

require dirname(__DIR__, 2).'/vendor/autoload.php';

$app = require dirname(__DIR__, 2).'/bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$command = $argv[1] ?? '';

$fail = static function (string $message, int $code = 1): never {
    fwrite(STDERR, $message.PHP_EOL);
    exit($code);
};

$json = static function (array $payload): never {
    fwrite(STDOUT, json_encode($payload, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES).PHP_EOL);
    exit(0);
};

if ($command === 'binding') {
    $email = $argv[2] ?? '';
    $identity = Identity::query()->where('email', $email)->first();
    if (! $identity instanceof Identity) {
        $fail('Identity not found.');
    }

    $binding = IdentityCanaryAccount::query()->find($identity->id);
    // Game-account provisioning has no native source yet, so the binding may legitimately
    // be pending or recoverable. Report the stored state instead of failing the probe.
    $json([
        'identity_id' => $identity->id,
        'canary_account_id' => $binding instanceof IdentityCanaryAccount ? $binding->canary_account_id : null,
        'status' => $binding instanceof IdentityCanaryAccount ? $binding->status : 'missing',
    ]);
}

if ($command === 'character-owner') {
    $name = $argv[2] ?? '';
    $email = $argv[3] ?? '';
    $identity = Identity::query()->where('email', $email)->first();
    if (! $identity instanceof Identity) {
        $fail('Identity not found.');
    }

    $binding = IdentityCanaryAccount::query()->find($identity->id);
    if (! $binding instanceof IdentityCanaryAccount || ! $binding->isReady()) {
        $fail('Identity Canary account binding is not ready.');
    }

    $player = DB::connection('canary_character_create')
        ->table('players')
        ->select(['id', 'name', 'account_id'])
        ->where('name', $name)
        ->first();

    if ($player === null || (int) $player->account_id !== $binding->canary_account_id) {
        $fail('Character is not owned by the expected bound Canary account.');
    }

    $json([
        'player_id' => (int) $player->id,
        'name' => (string) $player->name,
        'canary_account_id' => (int) $player->account_id,
    ]);
}

if ($command === 'unknown-permission-denied') {
    $email = $argv[2] ?? '';
    $identity = Identity::query()->where('email', $email)->first();
    if (! $identity instanceof Identity) {
        $fail('Identity not found.');
    }

    $allowed = $app->make(AdminAuthorization::class)->allows($identity, 'acceptance.unknown.permission');
    if ($allowed) {
        $fail('Unknown permission was unexpectedly allowed.');
    }

    $json(['unknown_permission_allowed' => false]);
}

if ($command === 'recovery-code-consumed') {
    $email = $argv[2] ?? '';
    $code = $argv[3] ?? '';
    $identity = Identity::query()->where('email', $email)->first();
    if (! $identity instanceof Identity) {
        $fail('Identity not found.');
    }

    $normalized = (new MfaRecoveryCodes)->normalize($code);
    foreach ($identity->two_factor_recovery_codes ?? [] as $hash) {
        if (Hash::check($normalized, $hash)) {
            $fail('Recovery code remains available after successful consumption.');
        }
    }

    $json(['recovery_code_consumed' => true]);
}

$fail('Unknown acceptance assertion command.', 2);
