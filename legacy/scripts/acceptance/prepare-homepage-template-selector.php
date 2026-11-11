<?php

declare(strict_types=1);

use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;

require dirname(__DIR__, 2).'/vendor/autoload.php';

$app = require dirname(__DIR__, 2).'/bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$email = $argv[1] ?? '';
$identity = Identity::query()->where('email', $email)->first();
if (! $identity instanceof Identity) {
    fwrite(STDERR, "Identity not found.\n");
    exit(1);
}

$roleId = DB::table('admin_roles')->where('key', 'platform_admin')->value('id');
$permissionId = DB::table('admin_permissions')->where('key', 'portal.settings.manage')->value('id');
if (! is_numeric($roleId) || ! is_numeric($permissionId)) {
    fwrite(STDERR, "Required administrator role or permission is missing.\n");
    exit(1);
}

DB::transaction(function () use ($identity, $roleId, $permissionId): void {
    DB::table('admin_role_permissions')->insertOrIgnore([
        'role_id' => (int) $roleId,
        'permission_id' => (int) $permissionId,
    ]);
    DB::table('identity_admin_roles')->insertOrIgnore([
        'identity_id' => $identity->id,
        'role_id' => (int) $roleId,
    ]);
    DB::table('homepage_template_settings')->where('id', 1)->update([
        'active_key' => 'production',
        'previous_key' => null,
        'version' => 0,
        'updated_by_identity_id' => null,
        'updated_at' => now(),
    ]);
});

fwrite(STDOUT, json_encode(['status' => 'ready'], JSON_THROW_ON_ERROR).PHP_EOL);
