<?php

declare(strict_types=1);

use App\Admin\AdminPermission;
use App\Admin\AdminRoleManager;
use App\Downloads\Models\ClientRelease;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;

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

$databaseId = static function (string $table, string $key) use ($fail): int {
    $id = DB::table($table)->where('key', $key)->value('id');

    if (is_int($id)) {
        return $id;
    }

    if (is_string($id) && ctype_digit($id)) {
        return (int) $id;
    }

    $fail("Expected integer-compatible id for {$table}.{$key}.");
};

if ($command === 'grant-downloads') {
    $email = $argv[2] ?? '';
    $identity = Identity::query()->where('email', $email)->first();
    if (! $identity instanceof Identity) {
        $fail('Identity not found.');
    }

    DB::transaction(static function () use ($databaseId, $identity): void {
        $roleId = $databaseId('admin_roles', AdminRoleManager::CONTENT_EDITOR);
        $permissionId = $databaseId('admin_permissions', AdminPermission::MANAGE_DOWNLOADS);

        DB::table('admin_role_permissions')->insertOrIgnore([
            'role_id' => $roleId,
            'permission_id' => $permissionId,
        ]);
        DB::table('identity_admin_roles')->insertOrIgnore([
            'identity_id' => $identity->id,
            'role_id' => $roleId,
        ]);
    }, 3);

    $json([
        'identity_id' => $identity->id,
        'permission' => AdminPermission::MANAGE_DOWNLOADS,
        'granted' => true,
    ]);
}

if ($command === 'seed-portability') {
    $version = '7.0.0-portability';
    $filename = 'oteryn-portability.zip';
    $artifactUrl = "https://downloads.example.test/releases/{$version}/{$filename}";

    $release = DB::transaction(static function () use ($artifactUrl, $filename, $version): ClientRelease {
        ClientRelease::query()
            ->where('version', '!=', $version)
            ->update(['is_current' => false]);

        $release = ClientRelease::query()->updateOrCreate(
            ['version' => $version],
            [
                'channel' => 'stable',
                'release_notes' => 'Deterministic browser portability fixture.',
                'published_at' => now()->subMinute(),
                'is_current' => true,
            ],
        );

        $release->artifacts()->delete();
        $release->artifacts()->create([
            'platform' => 'windows',
            'architecture' => 'x86_64',
            'artifact_url' => $artifactUrl,
            'filename' => $filename,
            'size_bytes' => 1_572_864,
            'sha256' => str_repeat('b', 64),
            'is_enabled' => true,
        ]);

        return $release;
    }, 3);

    $json([
        'release_id' => $release->id,
        'version' => $version,
        'artifact_url' => $artifactUrl,
        'seeded' => true,
    ]);
}

if ($command === 'set-artifact-url') {
    $version = $argv[2] ?? '';
    $url = $argv[3] ?? '';
    $release = ClientRelease::query()->where('version', $version)->first();
    if (! $release instanceof ClientRelease) {
        $fail('Client release not found.');
    }

    $artifact = $release->artifacts()->orderBy('id')->first();
    if ($artifact === null) {
        $fail('Client release artifact not found.');
    }

    $artifact->forceFill(['artifact_url' => $url])->save();

    $json([
        'release_id' => $release->id,
        'artifact_id' => $artifact->id,
        'updated' => true,
    ]);
}

$fail('Unknown Downloads acceptance state command.', 2);
