<?php

declare(strict_types=1);

use App\Cms\Editorial\EditorialContentType;
use App\Cms\Editorial\EditorialPageKey;
use App\Cms\Models\EditorialTranslation;
use App\Cms\Models\ManagedPage;
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

$polishTitles = [
    EditorialPageKey::GettingStarted->value => 'Akceptacyjny przewodnik dla początkujących',
    EditorialPageKey::ServerInformation->value => 'Akceptacyjne informacje o serwerze',
    EditorialPageKey::Support->value => 'Akceptacyjna pomoc techniczna',
    EditorialPageKey::ReportABug->value => 'Akceptacyjne zgłaszanie błędu',
    EditorialPageKey::Rules->value => 'Akceptacyjne zasady',
    EditorialPageKey::Terms->value => 'Akceptacyjny regulamin usługi',
    EditorialPageKey::Privacy->value => 'Akceptacyjna polityka prywatności',
    EditorialPageKey::Cookies->value => 'Akceptacyjna polityka plików cookie',
];

$describe = static function () use ($polishTitles): array {
    $pages = [];

    foreach (EditorialPageKey::cases() as $key) {
        $pages[] = [
            'key' => $key->value,
            'label' => $key->label(),
            'legacy_path' => route('legacy.'.$key->publicRouteName(), absolute: false),
            'english_path' => route($key->publicRouteName(), ['locale' => 'en'], false),
            'polish_path' => route($key->publicRouteName(), ['locale' => 'pl'], false),
            'english_title' => 'Acceptance '.$key->label(),
            'polish_title' => $polishTitles[$key->value],
            'english_body' => 'English acceptance content for '.$key->value.'. <img src=x onerror=alert("support-legal")>',
            'polish_body' => 'Polska treść akceptacyjna dla '.$key->value.'.',
            'draft_secret' => 'DRAFT-SECRET-'.$key->value,
            'legal' => $key->isLegal(),
            'legal_version' => $key->isLegal() ? '2026.1' : null,
            'legal_effective_date' => $key->isLegal() ? '2026-07-01' : null,
        ];
    }

    return $pages;
};

$reset = static function (): void {
    $pageIds = ManagedPage::query()
        ->whereIn('slug', EditorialPageKey::managedPageSlugs())
        ->pluck('id')
        ->map(static fn (mixed $id): int => (int) $id)
        ->all();

    if ($pageIds !== []) {
        DB::table('editorial_translations')
            ->where('content_type', EditorialContentType::ManagedPage->value)
            ->whereIn('content_id', $pageIds)
            ->delete();
        DB::table('managed_page_legal_versions')->whereIn('managed_page_id', $pageIds)->delete();
        DB::table('admin_audit_events')
            ->where('target_type', 'managed_page')
            ->whereIn('target_id', array_map(static fn (int $id): string => (string) $id, $pageIds))
            ->delete();
    }

    ManagedPage::query()->whereIn('slug', EditorialPageKey::managedPageSlugs())->delete();
};

if ($command === 'describe') {
    $json(['pages' => $describe()]);
}

if ($command === 'reset') {
    $reset();
    $json(['reset' => true]);
}

if ($command === 'seed-unpublished') {
    $reset();

    foreach ($describe() as $definition) {
        ManagedPage::query()->create([
            'slug' => EditorialPageKey::from($definition['key'])->managedPageSlug(),
            'title' => 'Draft '.$definition['label'],
            'body' => $definition['draft_secret'],
            'legal_version' => $definition['legal_version'],
            'legal_effective_date' => $definition['legal_effective_date'],
            'published_at' => null,
        ]);
    }

    $json(['pages' => $describe()]);
}

if ($command === 'seed-public') {
    $reset();
    $publishedAt = now()->subMinute();
    $pages = [];

    foreach ($describe() as $definition) {
        $key = EditorialPageKey::from($definition['key']);
        $page = ManagedPage::query()->create([
            'slug' => $key->managedPageSlug(),
            'title' => $definition['english_title'],
            'body' => $definition['english_body'],
            'legal_version' => $definition['legal_version'],
            'legal_effective_date' => $definition['legal_effective_date'],
            'published_at' => $publishedAt,
        ]);

        EditorialTranslation::query()->create([
            'content_type' => EditorialContentType::ManagedPage->value,
            'content_id' => $page->id,
            'locale' => 'pl',
            'title' => $definition['polish_title'],
            'body' => $definition['polish_body'],
            'action_label' => null,
            'source_updated_at' => $page->updated_at,
            'published_at' => $publishedAt,
        ]);

        $pages[] = [...$definition, 'id' => $page->id];
    }

    $json(['pages' => $pages]);
}

if ($command === 'seed-identity') {
    $email = $argv[2] ?? '';
    $password = $argv[3] ?? '';
    $recoveryCode = $argv[4] ?? '';
    $mfaConfirmed = ($argv[5] ?? '') === 'confirmed';
    $permissionCsv = $argv[6] ?? '';
    $permissions = array_values(array_filter(array_map('trim', explode(',', $permissionCsv))));

    if ($email === '' || $password === '') {
        $fail('Usage: php scripts/acceptance/seed-browser-support-legal.php seed-identity <email> <password> <recovery-code> <confirmed|unconfirmed> <permission-csv>', 2);
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
        $roleKey = 'acceptance_support_legal_'.$identity->id;
        $now = now();
        $roleId = DB::table('admin_roles')->where('key', $roleKey)->value('id');
        if ($roleId === null) {
            $roleId = DB::table('admin_roles')->insertGetId([
                'key' => $roleKey,
                'name' => 'Acceptance Support Legal role',
                'created_at' => $now,
                'updated_at' => $now,
            ]);
        }

        $roleId = $integerId($roleId, 'Acceptance Support Legal role');
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

$fail('Unknown Support/Legal acceptance fixture command.', 2);
