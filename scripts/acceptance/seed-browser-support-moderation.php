<?php

declare(strict_types=1);

use App\Identity\Mfa\MfaRecoveryCodes;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use PragmaRX\Google2FA\Google2FA;

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

$integerId = static function (mixed $value, string $label) use ($fail): int {
    if (is_int($value)) {
        return $value;
    }
    if (is_string($value) && ctype_digit($value)) {
        return (int) $value;
    }

    $fail("{$label} is unavailable after migrations.");
};

$reset = static function (): void {
    DB::table('support_notification_deliveries')->delete();
    DB::table('support_ticket_messages')->delete();
    DB::table('support_tickets')->delete();
    DB::table('player_reports')->delete();
    DB::table('enforcement_records')->delete();
    DB::table('admin_audit_events')
        ->whereIn('target_type', ['support_ticket', 'player_report', 'enforcement_record'])
        ->delete();
};

if ($command === 'reset') {
    $reset();
    $json(['reset' => true]);
}

if ($command === 'seed-identity') {
    $email = $argv[2] ?? '';
    $password = $argv[3] ?? '';
    $recoveryCodeCsv = $argv[4] ?? '';
    $mfaConfirmed = ($argv[5] ?? '') === 'confirmed';
    $permissionCsv = $argv[6] ?? '';
    $permissions = array_values(array_filter(array_map('trim', explode(',', $permissionCsv))));
    $recoveryCodes = array_values(array_filter(array_map('trim', explode(',', $recoveryCodeCsv))));

    if ($email === '' || $password === '') {
        $fail('Usage: seed-identity <email> <password> <recovery-code-csv> <confirmed|unconfirmed> <permission-csv>', 2);
    }

    $identity = Identity::query()->updateOrCreate(
        ['email' => $email],
        ['password' => Hash::make($password)],
    );
    $attributes = [
        'web_session_generation' => 0,
        'game_auth_generation' => 0,
        'disabled_at' => null,
        'terminated_at' => null,
        'two_factor_secret' => null,
        'two_factor_recovery_codes' => null,
        'two_factor_confirmed_at' => null,
        'two_factor_last_used_timestep' => null,
    ];

    if ($mfaConfirmed) {
        if ($recoveryCodes === []) {
            $fail('At least one recovery code is required for a confirmed-MFA identity.');
        }

        $normalizer = new MfaRecoveryCodes;
        $attributes['two_factor_secret'] = (new Google2FA)->generateSecretKey();
        $attributes['two_factor_recovery_codes'] = array_map(
            static fn (string $recoveryCode): string => Hash::make($normalizer->normalize($recoveryCode)),
            $recoveryCodes,
        );
        $attributes['two_factor_confirmed_at'] = now();
    }

    $identity->forceFill($attributes)->save();
    DB::table('identity_admin_roles')->where('identity_id', $identity->id)->delete();

    if ($permissions !== []) {
        $roleKey = 'acceptance_support_moderation_'.$identity->id;
        $now = now();
        $roleId = DB::table('admin_roles')->where('key', $roleKey)->value('id');
        if ($roleId === null) {
            $roleId = DB::table('admin_roles')->insertGetId([
                'key' => $roleKey,
                'name' => 'Acceptance Support Moderation role',
                'created_at' => $now,
                'updated_at' => $now,
            ]);
        }
        $roleId = $integerId($roleId, 'Acceptance Support Moderation role');
        DB::table('admin_role_permissions')->where('role_id', $roleId)->delete();

        foreach ($permissions as $permission) {
            DB::table('admin_role_permissions')->insert([
                'role_id' => $roleId,
                'permission_id' => $integerId(
                    DB::table('admin_permissions')->where('key', $permission)->value('id'),
                    "Permission {$permission}",
                ),
            ]);
        }

        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => $roleId,
        ]);
    }

    $json([
        'identity_id' => $identity->id,
        'email' => $identity->email,
        'mfa_confirmed' => $mfaConfirmed,
        'recovery_code_count' => count($recoveryCodes),
        'permissions' => $permissions,
    ]);
}

$fail('Unknown Support Moderation acceptance fixture command.', 2);
