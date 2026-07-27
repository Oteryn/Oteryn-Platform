<?php

declare(strict_types=1);

use App\Announcements\Models\SiteAnnouncement;
use App\Cms\Editorial\EditorialContentType;
use App\Cms\Models\EditorialTranslation;
use App\Identity\Mfa\MfaRecoveryCodes;
use App\Identity\Models\Identity;
use Carbon\CarbonImmutable;
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
    DB::table('editorial_translations')
        ->where('content_type', EditorialContentType::SiteAnnouncement->value)
        ->delete();
    DB::table('admin_audit_events')->where('target_type', 'site_announcement')->delete();
    DB::table('site_announcements')->delete();
};

if ($command === 'reset') {
    $reset();
    $json(['reset' => true]);
}

if ($command === 'seed-public') {
    $reset();
    $now = CarbonImmutable::now('UTC')->startOfMinute();

    $active = SiteAnnouncement::query()->create([
        'title' => 'Acceptance Active Notice',
        'body' => '<img src=x onerror=alert("announcement")> Plain-text maintenance details.',
        'severity' => SiteAnnouncement::SEVERITY_MAINTENANCE,
        'starts_at' => $now->subHour(),
        'ends_at' => $now->addHour(),
        'publication_state' => SiteAnnouncement::STATE_PUBLISHED,
        'action_label' => 'Read maintenance details',
        'action_url' => '/en/news',
        'lock_version' => 1,
    ]);

    SiteAnnouncement::query()->create([
        'title' => 'Acceptance Expired Notice',
        'body' => 'This expired notice must remain hidden.',
        'severity' => SiteAnnouncement::SEVERITY_INFO,
        'starts_at' => $now->subHours(3),
        'ends_at' => $now->subHours(2),
        'publication_state' => SiteAnnouncement::STATE_PUBLISHED,
        'lock_version' => 1,
    ]);

    SiteAnnouncement::query()->create([
        'title' => 'Acceptance Future Notice',
        'body' => 'This future notice must remain hidden.',
        'severity' => SiteAnnouncement::SEVERITY_WARNING,
        'starts_at' => $now->addHours(2),
        'ends_at' => $now->addHours(3),
        'publication_state' => SiteAnnouncement::STATE_PUBLISHED,
        'lock_version' => 1,
    ]);

    SiteAnnouncement::query()->create([
        'title' => 'Acceptance Draft Notice',
        'body' => 'This draft notice must remain hidden.',
        'severity' => SiteAnnouncement::SEVERITY_WARNING,
        'starts_at' => $now->subHour(),
        'ends_at' => $now->addHour(),
        'publication_state' => SiteAnnouncement::STATE_DRAFT,
        'lock_version' => 1,
    ]);

    EditorialTranslation::query()->create([
        'content_type' => EditorialContentType::SiteAnnouncement->value,
        'content_id' => $active->id,
        'locale' => 'pl',
        'title' => 'Aktywny komunikat akceptacyjny',
        'body' => 'Polskie informacje o przerwie technicznej.',
        'action_label' => 'Przeczytaj szczegóły',
        'source_updated_at' => $active->updated_at,
        'published_at' => $now->subMinute(),
    ]);

    $json([
        'active_id' => $active->id,
        'active_title' => $active->title,
    ]);
}

if ($command === 'seed-identity') {
    $email = $argv[2] ?? '';
    $password = $argv[3] ?? '';
    $recoveryCode = $argv[4] ?? '';
    $mfaConfirmed = ($argv[5] ?? '') === 'confirmed';
    $permissionCsv = $argv[6] ?? '';
    $permissions = array_values(array_filter(array_map('trim', explode(',', $permissionCsv))));

    if ($email === '' || $password === '') {
        $fail('Usage: php scripts/acceptance/seed-browser-announcements.php seed-identity <email> <password> <recovery-code> <confirmed|unconfirmed> <permission-csv>', 2);
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
        $roleKey = 'acceptance_announcements_'.$identity->id;
        $now = now();
        $roleId = DB::table('admin_roles')->where('key', $roleKey)->value('id');
        if ($roleId === null) {
            $roleId = DB::table('admin_roles')->insertGetId([
                'key' => $roleKey,
                'name' => 'Acceptance Announcements role',
                'created_at' => $now,
                'updated_at' => $now,
            ]);
        }

        $roleId = $integerId($roleId, 'Acceptance Announcements role');
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

if ($command === 'bump-lock') {
    $id = $integerId($argv[2] ?? null, 'Announcement id');
    SiteAnnouncement::query()->whereKey($id)->increment('lock_version');
    $announcement = SiteAnnouncement::query()->findOrFail($id);
    $json(['id' => $announcement->id, 'lock_version' => $announcement->lock_version]);
}

$fail('Unknown Announcements acceptance fixture command.', 2);
