<?php

declare(strict_types=1);

use App\Admin\AdminPermission;
use App\Admin\AdminRoleManager;
use App\Downloads\Models\ClientRelease;
use App\Downloads\Models\ClientUpdateGeneration;
use App\Downloads\Models\ClientUpdatePolicy;
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

if ($command === 'updater-generation-template') {
    $channel = $argv[2] ?? '';
    $policy = ClientUpdatePolicy::query()
        ->where('channel', $channel)
        ->orderByDesc('revision')
        ->first();
    if (! $policy instanceof ClientUpdatePolicy) {
        $fail('Approved updater policy not found.');
    }

    $latest = ClientUpdateGeneration::query()
        ->where('channel', $channel)
        ->orderByDesc('timestamp_version')
        ->first();
    $nextTargetsVersion = ($latest?->targets_version ?? 0) + 1;
    $nextSnapshotVersion = ($latest?->snapshot_version ?? 0) + 1;
    $nextTimestampVersion = ($latest?->timestamp_version ?? 0) + 1;
    $rootVersion = max(1, $latest?->root_version ?? 1);

    /** @var list<array{artifact_id: int, platform: string, architecture: string, target_path: string, size_bytes: int, supplied_sha256: string}> $artifactTargets */
    $artifactTargets = $policy->artifact_targets;
    $targets = array_map(
        static fn (array $target): array => [
            'platform' => $target['platform'],
            'architecture' => $target['architecture'],
            'target_path' => $target['target_path'],
            'length' => $target['size_bytes'],
            'sha256' => $target['supplied_sha256'],
        ],
        $artifactTargets,
    );
    $generationId = sprintf('acceptance-%s-policy-%d-timestamp-%d', $channel, $policy->revision, $nextTimestampVersion);

    $json([
        'generation_id' => $generationId,
        'channel' => $channel,
        'policy_revision' => $policy->revision,
        'root_version' => $rootVersion,
        'targets_version' => $nextTargetsVersion,
        'snapshot_version' => $nextSnapshotVersion,
        'timestamp_version' => $nextTimestampVersion,
        'metadata_expires_at' => now()->addHour()->utc()->toIso8601String(),
        'metadata_set_sha256' => hash('sha256', 'acceptance-public-metadata-set|'.$generationId),
        'policy_target_path' => $policy->policy_target_path,
        'policy_target_sha256' => $policy->policy_document_sha256,
        'policy_target_length' => $policy->policy_document_length,
        'targets' => $targets,
    ]);
}

$fail('Unknown Downloads acceptance state command.', 2);