<?php

declare(strict_types=1);

use App\EditorialMedia\Application\Actions\StoreEditorialImage;
use App\EditorialMedia\Application\EditorialMediaReferenceManager;
use App\EditorialMedia\Domain\EditorialMediaConsumer;
use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use App\Identity\Mfa\MfaRecoveryCodes;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Http\UploadedFile;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use Illuminate\Support\Facades\Storage;
use PragmaRX\Google2FA\Google2FA;

require dirname(__DIR__, 2).'/vendor/autoload.php';

$app = require dirname(__DIR__, 2).'/bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$command = $argv[1] ?? '';
$unavailableTable = 'editorial_media_acceptance_unavailable';

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

$restoreAvailability = static function () use ($unavailableTable): void {
    if (Schema::hasTable($unavailableTable) && ! Schema::hasTable('editorial_media')) {
        Schema::rename($unavailableTable, 'editorial_media');
    }
};

$mediaByArgument = static function (mixed $value) use ($integerId, $fail): EditorialMedia {
    $mediaId = $integerId($value, 'Editorial Media id');
    $media = EditorialMedia::query()->find($mediaId);
    if (! $media instanceof EditorialMedia) {
        $fail("Editorial Media {$mediaId} does not exist in the acceptance fixture.");
    }

    return $media;
};

$mediaPaths = static function (EditorialMedia $media): array {
    return array_values(array_filter([
        $media->storage_path,
        $media->thumbnail_path,
    ], static fn (mixed $path): bool => is_string($path) && $path !== ''));
};

$createPng = static function (string $path) use ($fail): void {
    $image = imagecreatetruecolor(320, 180);
    if (! $image instanceof GdImage) {
        $fail('Could not create the Editorial Media acceptance image.');
    }

    try {
        $background = imagecolorallocate($image, 24, 48, 72);
        $accent = imagecolorallocate($image, 190, 145, 72);
        if (! is_int($background) || ! is_int($accent)) {
            $fail('Could not allocate Editorial Media acceptance colours.');
        }

        imagefill($image, 0, 0, $background);
        imagefilledrectangle($image, 36, 80, 284, 100, $accent);

        if (! imagepng($image, $path, 6)) {
            $fail('Could not encode the Editorial Media acceptance image.');
        }
    } finally {
        imagedestroy($image);
    }
};

$removeUploadFixtures = static function (): void {
    foreach (glob(sys_get_temp_dir().'/oteryn-editorial-media-browser-*.png') ?: [] as $path) {
        if (is_string($path) && is_file($path)) {
            unlink($path);
        }
    }
};

$reset = static function () use ($removeUploadFixtures, $mediaPaths, $restoreAvailability): void {
    $restoreAvailability();
    $mediaItems = EditorialMedia::query()->get();

    DB::table('editorial_media_references')->delete();

    foreach ($mediaItems as $media) {
        $paths = $mediaPaths($media);

        if ($paths !== []) {
            Storage::disk($media->disk)->delete($paths);
        }
    }

    DB::table('admin_audit_events')->where('target_type', 'editorial_media')->delete();
    DB::table('editorial_media')->delete();
    $removeUploadFixtures();
};

if ($command === 'reset') {
    $reset();
    $json(['reset' => true]);
}

if ($command === 'set-admin-unavailable') {
    $restoreAvailability();

    if (! Schema::hasTable('editorial_media')) {
        $fail('Editorial Media table is unavailable before administrator failure injection.');
    }

    Schema::rename('editorial_media', $unavailableTable);
    $json(['admin_unavailable' => true]);
}

if ($command === 'restore-admin') {
    $restoreAvailability();
    $json(['admin_restored' => Schema::hasTable('editorial_media')]);
}

if ($command === 'create-upload-fixture') {
    $path = tempnam(sys_get_temp_dir(), 'oteryn-editorial-media-browser-');
    if (! is_string($path)) {
        $fail('Could not allocate an Editorial Media browser upload fixture.');
    }

    $pngPath = $path.'.png';
    if (! rename($path, $pngPath)) {
        if (is_file($path)) {
            unlink($path);
        }
        $fail('Could not prepare an Editorial Media browser upload fixture path.');
    }

    $createPng($pngPath);
    $json(['path' => $pngPath]);
}

if ($command === 'seed-identity') {
    $email = $argv[2] ?? '';
    $password = $argv[3] ?? '';
    $recoveryCode = $argv[4] ?? '';
    $mfaConfirmed = ($argv[5] ?? '') === 'confirmed';
    $permissionCsv = $argv[6] ?? '';
    $permissions = array_values(array_filter(array_map('trim', explode(',', $permissionCsv))));

    if ($email === '' || $password === '') {
        $fail('Usage: php scripts/acceptance/seed-browser-editorial-media.php seed-identity <email> <password> <recovery-code> <confirmed|unconfirmed> <permission-csv>', 2);
    }

    $identity = Identity::query()->updateOrCreate(
        ['email' => $email],
        ['password' => Hash::make($password)],
    );
    $attributes = [
        'web_session_generation' => 0,
        'disabled_at' => null,
        'two_factor_secret' => null,
        'two_factor_recovery_codes' => null,
        'two_factor_confirmed_at' => null,
        'two_factor_last_used_timestep' => null,
    ];

    if ($mfaConfirmed) {
        if ($recoveryCode === '') {
            $fail('A recovery code is required for a confirmed-MFA acceptance identity.');
        }

        $normalizer = new MfaRecoveryCodes;
        $attributes['two_factor_secret'] = (new Google2FA)->generateSecretKey();
        $attributes['two_factor_recovery_codes'] = [Hash::make($normalizer->normalize($recoveryCode))];
        $attributes['two_factor_confirmed_at'] = now();
    }

    $identity->forceFill($attributes)->save();
    DB::table('identity_admin_roles')->where('identity_id', $identity->id)->delete();

    if ($permissions !== []) {
        $roleKey = 'acceptance_editorial_media_'.$identity->id;
        $now = now();
        $roleId = DB::table('admin_roles')->where('key', $roleKey)->value('id');
        if ($roleId === null) {
            $roleId = DB::table('admin_roles')->insertGetId([
                'key' => $roleKey,
                'name' => 'Acceptance Editorial Media role',
                'created_at' => $now,
                'updated_at' => $now,
            ]);
        }

        $roleId = $integerId($roleId, 'Acceptance Editorial Media role');
        DB::table('admin_role_permissions')->where('role_id', $roleId)->delete();

        foreach ($permissions as $permission) {
            $permissionId = $integerId(
                DB::table('admin_permissions')->where('key', $permission)->value('id'),
                "Permission {$permission}",
            );
            DB::table('admin_role_permissions')->insert([
                'role_id' => $roleId,
                'permission_id' => $permissionId,
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
        'permissions' => $permissions,
    ]);
}

if ($command === 'seed-referenced') {
    $email = $argv[2] ?? '';
    $label = trim($argv[3] ?? 'Referenced acceptance image');

    if ($email === '' || $label === '') {
        $fail('Usage: php scripts/acceptance/seed-browser-editorial-media.php seed-referenced <email> <label>', 2);
    }

    $identity = Identity::query()->where('email', $email)->first();
    if (! $identity instanceof Identity) {
        $fail('The Editorial Media acceptance identity must be seeded first.');
    }

    $temporaryPath = tempnam(sys_get_temp_dir(), 'oteryn-editorial-media-');
    if (! is_string($temporaryPath)) {
        $fail('Could not allocate an Editorial Media acceptance file.');
    }

    try {
        $createPng($temporaryPath);

        $media = app(StoreEditorialImage::class)->execute(
            $identity,
            new UploadedFile(
                $temporaryPath,
                'referenced-acceptance.png',
                'image/png',
                null,
                true,
            ),
            $label,
        );

        app(EditorialMediaReferenceManager::class)->attach(
            $media,
            EditorialMediaConsumer::WIKI,
            'acceptance:editorial-media',
            'body',
        );
    } finally {
        if (is_file($temporaryPath)) {
            unlink($temporaryPath);
        }
    }

    $json([
        'media_id' => $media->id,
        'alt_text' => $media->alt_text,
    ]);
}

if ($command === 'remove-files') {
    $media = $mediaByArgument($argv[2] ?? '');
    $paths = $mediaPaths($media);
    if ($paths === []) {
        $fail('Editorial Media acceptance fixture has no stored paths to remove.');
    }

    Storage::disk($media->disk)->delete($paths);
    $json([
        'media_id' => $media->id,
        'state' => 'missing',
        'paths' => count($paths),
    ]);
}

if ($command === 'corrupt-files') {
    $media = $mediaByArgument($argv[2] ?? '');
    $paths = $mediaPaths($media);
    if ($paths === []) {
        $fail('Editorial Media acceptance fixture has no stored paths to corrupt.');
    }

    $filesystem = Storage::disk($media->disk);
    foreach ($paths as $path) {
        $filesystem->put($path, 'corrupt-editorial-media-acceptance-object', ['visibility' => 'private']);
    }

    $json([
        'media_id' => $media->id,
        'state' => 'integrity_failed',
        'paths' => count($paths),
    ]);
}

$fail('Unknown Editorial Media acceptance fixture command.', 2);
