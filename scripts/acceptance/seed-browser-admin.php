<?php

declare(strict_types=1);

use App\Admin\AdminPermission;
use App\Identity\Mfa\MfaRecoveryCodes;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use PragmaRX\Google2FA\Google2FA;

require __DIR__.'/../../vendor/autoload.php';

$app = require __DIR__.'/../../bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$email = $argv[1] ?? '';
$password = $argv[2] ?? '';
$recoveryCode = $argv[3] ?? '';

if ($email === '' || $password === '' || $recoveryCode === '') {
    fwrite(STDERR, "Usage: php scripts/acceptance/seed-browser-admin.php <email> <password> <recovery-code>\n");
    exit(2);
}

$admin = Identity::query()->updateOrCreate(
    ['email' => $email],
    ['password' => Hash::make($password)],
);

$normalizer = new MfaRecoveryCodes;
$admin->forceFill([
    'web_session_generation' => 0,
    'disabled_at' => null,
    'two_factor_secret' => (new Google2FA)->generateSecretKey(),
    'two_factor_recovery_codes' => [
        Hash::make($normalizer->normalize($recoveryCode)),
    ],
    'two_factor_confirmed_at' => now(),
    'two_factor_last_used_timestep' => null,
])->save();

$integerId = static function (mixed $value, string $label): int {
    if (is_int($value)) {
        return $value;
    }

    if (is_string($value) && ctype_digit($value)) {
        return (int) $value;
    }

    throw new RuntimeException("{$label} is unavailable after migrations.");
};

// This role exists only inside the isolated acceptance database. It grants the
// exact permissions required to render every administrator destination covered
// by the visual review without broadening any production role or RBAC contract.
$permissionKeys = [
    AdminPermission::ACCESS,
    AdminPermission::MANAGE_NEWS,
    AdminPermission::MANAGE_PAGES,
    AdminPermission::MANAGE_EVENTS,
    AdminPermission::MANAGE_PORTAL_ANNOUNCEMENTS,
    AdminPermission::MANAGE_DOWNLOADS,
    AdminPermission::MANAGE_MEDIA,
    AdminPermission::WIKI_ACCESS,
    AdminPermission::MANAGE_SUPPORT_CONTENT,
    AdminPermission::MANAGE_PORTAL_SETTINGS,
    AdminPermission::MANAGE_ROLES,
    AdminPermission::MANAGE_SUPPORT_TICKETS,
    AdminPermission::MANAGE_SUPPORT_REPORTS,
    AdminPermission::MANAGE_SUPPORT_ENFORCEMENT,
    AdminPermission::GAME_CATALOG_ACCESS,
    AdminPermission::VIEW_GAME_CATALOG_SNAPSHOTS,
    AdminPermission::RECONCILE_PAYMENTS,
    AdminPermission::MANAGE_MARKETPLACE,
    AdminPermission::VIEW_AUDIT,
];

$roleKey = 'acceptance_portal_visual_admin_'.$admin->id;
$roleId = DB::table('admin_roles')->where('key', $roleKey)->value('id');
if ($roleId === null) {
    $now = now();
    $roleId = DB::table('admin_roles')->insertGetId([
        'key' => $roleKey,
        'name' => 'Acceptance portal visual administrator',
        'created_at' => $now,
        'updated_at' => $now,
    ]);
}
$roleId = $integerId($roleId, 'Acceptance portal visual administrator role');

DB::table('admin_role_permissions')->where('role_id', $roleId)->delete();
foreach ($permissionKeys as $permissionKey) {
    $permissionId = $integerId(
        DB::table('admin_permissions')->where('key', $permissionKey)->value('id'),
        "Permission {$permissionKey}",
    );
    DB::table('admin_role_permissions')->insert([
        'role_id' => $roleId,
        'permission_id' => $permissionId,
    ]);
}

DB::table('identity_admin_roles')->where('identity_id', $admin->id)->delete();
DB::table('identity_admin_roles')->insert([
    'identity_id' => $admin->id,
    'role_id' => $roleId,
]);

fwrite(STDOUT, json_encode([
    'identity_id' => $admin->id,
    'email' => $email,
], JSON_THROW_ON_ERROR)."\n");
