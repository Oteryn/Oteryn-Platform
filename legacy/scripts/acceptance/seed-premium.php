<?php

declare(strict_types=1);

use App\Admin\AdminPermission;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;

require __DIR__.'/../../vendor/autoload.php';

$app = require __DIR__.'/../../bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

if (! $app->environment('acceptance')) {
    fwrite(STDERR, "Premium time fixture seeding is restricted to the acceptance environment.\n");
    exit(2);
}

$adminEmail = $argv[1] ?? '';
$targetEmail = $argv[2] ?? '';

if ($adminEmail === '' || $targetEmail === '') {
    fwrite(STDERR, "Usage: php scripts/acceptance/seed-premium.php <seeded-admin-email> <target-email>\n");
    exit(2);
}

$admin = Identity::query()->where('email', $adminEmail)->firstOrFail();
$target = Identity::query()->updateOrCreate(
    ['email' => $targetEmail],
    ['password' => Hash::make(Str::random(32))],
);

$integerId = static function (mixed $value, string $label): int {
    if (is_int($value)) {
        return $value;
    }

    if (is_string($value) && ctype_digit($value)) {
        return (int) $value;
    }

    throw new RuntimeException("{$label} is unavailable after migrations.");
};

// An additional acceptance-only role: the shared browser administrator keeps its own grants unchanged.
$roleKey = 'acceptance_premium_operator_'.$admin->id;
$roleId = DB::table('admin_roles')->where('key', $roleKey)->value('id');
if ($roleId === null) {
    $now = now();
    $roleId = DB::table('admin_roles')->insertGetId([
        'key' => $roleKey,
        'name' => 'Acceptance Premium time operator',
        'created_at' => $now,
        'updated_at' => $now,
    ]);
}
$roleId = $integerId($roleId, 'Acceptance Premium time operator role');
$permissionId = $integerId(
    DB::table('admin_permissions')->where('key', AdminPermission::MANAGE_PREMIUM_TIME)->value('id'),
    'Permission '.AdminPermission::MANAGE_PREMIUM_TIME,
);
DB::table('admin_role_permissions')->insertOrIgnore(['role_id' => $roleId, 'permission_id' => $permissionId]);
DB::table('identity_admin_roles')->insertOrIgnore(['identity_id' => $admin->id, 'role_id' => $roleId]);

fwrite(STDOUT, json_encode([
    'admin_identity_id' => $admin->id,
    'target_identity_id' => $target->id,
    'target_email' => $targetEmail,
], JSON_THROW_ON_ERROR)."\n");
