<?php

declare(strict_types=1);

use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

require dirname(__DIR__, 2).'/vendor/autoload.php';

$app = require dirname(__DIR__, 2).'/bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

if (! $app->environment('acceptance')) {
    fwrite(STDERR, "Portal #487 strictness fixture is acceptance-only.\n");
    exit(2);
}

$surfaces = [
    'admin.core-rbac-cms-audit' => 'news_posts',
    'admin.homepage-template-selector' => 'homepage_template_settings',
    'announcements.admin-localization-home-composition' => 'site_announcements',
    'downloads.public-admin-localization' => 'client_releases',
    'events.public-admin' => 'events',
    'public.home-and-seo' => 'news_posts',
    'public.localization-core' => 'news_posts',
    'public.news-and-managed-pages' => 'news_posts',
    'support-legal.public-admin-localization' => 'managed_pages',
    'support.moderation-lifecycle' => 'support_tickets',
];

$adminPermissions = [
    'cms.news.manage',
    'portal.settings.manage',
    'portal.announcements.manage',
    'downloads.manage',
    'events.manage',
    'support.content.manage',
    'support.tickets.manage',
];

$unavailableName = static fn (string $table): string => $table.'_acceptance_unavailable';

$fail = static function (string $message, int $code = 1): never {
    fwrite(STDERR, $message.PHP_EOL);
    exit($code);
};

$json = static function (array $payload): never {
    fwrite(STDOUT, json_encode($payload, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES).PHP_EOL);
    exit(0);
};

$restoreTable = static function (string $table) use ($unavailableName): void {
    $unavailable = $unavailableName($table);

    if (Schema::hasTable($unavailable) && ! Schema::hasTable($table)) {
        Schema::rename($unavailable, $table);
    }
};

$restoreAll = static function () use ($surfaces, $restoreTable): void {
    foreach (array_values(array_unique($surfaces)) as $table) {
        $restoreTable($table);
    }
};

$command = $argv[1] ?? '';

if ($command === 'restore-all') {
    $restoreAll();
    $json(['restored' => true]);
}

if ($command === 'grant-admin-permissions') {
    $roleId = DB::table('admin_roles')->where('key', 'platform_admin')->value('id');
    if (! is_numeric($roleId)) {
        $fail('Required platform_admin role is missing.');
    }

    foreach ($adminPermissions as $permission) {
        $permissionId = DB::table('admin_permissions')->where('key', $permission)->value('id');
        if (! is_numeric($permissionId)) {
            $fail("Required administrator permission {$permission} is missing.");
        }

        DB::table('admin_role_permissions')->insertOrIgnore([
            'role_id' => (int) $roleId,
            'permission_id' => (int) $permissionId,
        ]);
    }

    $json(['role' => 'platform_admin', 'permissions' => $adminPermissions]);
}

$surface = $argv[2] ?? '';
$table = $surfaces[$surface] ?? null;
if (! is_string($table)) {
    $fail('Unknown or missing strictness surface.', 2);
}

if ($command === 'make-unavailable') {
    $restoreTable($table);

    if (! Schema::hasTable($table)) {
        $fail("Required table {$table} is unavailable before failure injection.");
    }

    $unavailable = $unavailableName($table);
    if (Schema::hasTable($unavailable)) {
        $fail("Refusing to overwrite existing unavailable fixture table {$unavailable}.");
    }

    Schema::rename($table, $unavailable);
    $json(['surface' => $surface, 'table' => $table, 'unavailable' => true]);
}

if ($command === 'restore') {
    $restoreTable($table);
    $json(['surface' => $surface, 'table' => $table, 'restored' => Schema::hasTable($table)]);
}

$fail('Usage: php scripts/acceptance/seed-browser-portal-487-strictness.php <grant-admin-permissions|make-unavailable|restore> [surface] | restore-all', 2);
