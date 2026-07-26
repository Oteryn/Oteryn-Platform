<?php

declare(strict_types=1);

use App\Admin\AdminPermission;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;

require __DIR__.'/../../vendor/autoload.php';

$app = require __DIR__.'/../../bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$email = $argv[1] ?? '';
$wikiOnly = ($argv[2] ?? '') === '--wiki-only';

if ($email === '') {
    fwrite(STDERR, "Usage: php scripts/acceptance/seed-admin-wiki-permissions.php <email>\n");
    exit(2);
}

$identity = Identity::query()->where('email', $email)->first();

if (! $identity instanceof Identity) {
    throw new RuntimeException('The acceptance administrator must be seeded before Wiki permissions are assigned.');
}

$roleKey = 'acceptance_wiki_admin_'.$identity->id;
$now = now();
$roleId = DB::table('admin_roles')->where('key', $roleKey)->value('id');

if (! is_int($roleId) && ! (is_string($roleId) && ctype_digit($roleId))) {
    $roleId = DB::table('admin_roles')->insertGetId([
        'key' => $roleKey,
        'name' => 'Acceptance Wiki administrator',
        'created_at' => $now,
        'updated_at' => $now,
    ]);
}

$roleId = (int) $roleId;

foreach ([
    AdminPermission::WIKI_ACCESS,
    AdminPermission::MANAGE_WIKI_ARTICLES,
    AdminPermission::MANAGE_WIKI_CATEGORIES,
    AdminPermission::PUBLISH_WIKI,
] as $permission) {
    $permissionId = DB::table('admin_permissions')->where('key', $permission)->value('id');

    if (! is_int($permissionId) && ! (is_string($permissionId) && ctype_digit($permissionId))) {
        throw new RuntimeException("Required Wiki permission [{$permission}] is unavailable after migrations.");
    }

    DB::table('admin_role_permissions')->updateOrInsert([
        'role_id' => $roleId,
        'permission_id' => (int) $permissionId,
    ]);
}

DB::table('identity_admin_roles')->updateOrInsert([
    'identity_id' => $identity->id,
    'role_id' => $roleId,
]);

if ($wikiOnly) {
    DB::table('identity_admin_roles')
        ->where('identity_id', $identity->id)
        ->where('role_id', '!=', $roleId)
        ->delete();
}

fwrite(STDOUT, json_encode([
    'identity_id' => $identity->id,
    'role_key' => $roleKey,
    'wiki_only' => $wikiOnly,
], JSON_THROW_ON_ERROR)."\n");
