<?php

declare(strict_types=1);

use App\Events\Models\Event;
use App\Events\Models\EventTranslation;
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

$createEventFixture = static function (
    string $key,
    string $status,
    CarbonImmutable $startsAt,
    CarbonImmutable $endsAt,
    array $translations,
    bool $featured = false,
): Event {
    $event = Event::query()->create([
        'status' => $status,
        'starts_at' => $startsAt,
        'ends_at' => $endsAt,
        'featured' => $featured,
        'lock_version' => 1,
    ]);

    foreach (['en', 'pl'] as $locale) {
        $title = $translations[$locale] ?? null;
        if (! is_string($title) || $title === '') {
            continue;
        }

        EventTranslation::query()->create([
            'event_id' => $event->id,
            'locale' => $locale,
            'slug' => "acceptance-{$key}-{$locale}",
            'title' => $title,
            'summary' => "{$title} summary",
            'body' => "{$title} body paragraph one.\n\n{$title} body paragraph two.",
        ]);
    }

    return $event;
};

if ($command === 'reset') {
    DB::table('event_translations')->delete();
    DB::table('events')->delete();
    DB::table('admin_audit_events')->where('target_type', 'event')->delete();

    $json(['reset' => true]);
}

if ($command === 'seed-public') {
    DB::table('event_translations')->delete();
    DB::table('events')->delete();

    $now = CarbonImmutable::now('UTC')->startOfMinute();
    $fixtures = [
        'active' => $createEventFixture(
            'active',
            Event::STATUS_SCHEDULED,
            $now->subHour(),
            $now->addHour(),
            ['en' => 'Acceptance Active Event', 'pl' => 'Aktywne wydarzenie akceptacyjne'],
            true,
        ),
        'upcoming' => $createEventFixture(
            'upcoming',
            Event::STATUS_SCHEDULED,
            $now->addHours(2),
            $now->addHours(3),
            ['en' => 'Acceptance Upcoming Event', 'pl' => 'Nadchodzące wydarzenie akceptacyjne'],
        ),
        'archived' => $createEventFixture(
            'archived',
            Event::STATUS_ACTIVE,
            $now->subHours(3),
            $now->subHours(2),
            ['en' => 'Acceptance Archived Event', 'pl' => 'Archiwalne wydarzenie akceptacyjne'],
        ),
        'cancelled' => $createEventFixture(
            'cancelled',
            Event::STATUS_CANCELLED,
            $now->addHours(4),
            $now->addHours(5),
            ['en' => 'Acceptance Cancelled Event', 'pl' => 'Anulowane wydarzenie akceptacyjne'],
        ),
    ];

    $json([
        'slugs' => [
            'active_en' => 'acceptance-active-en',
            'active_pl' => 'acceptance-active-pl',
            'upcoming_en' => 'acceptance-upcoming-en',
            'upcoming_pl' => 'acceptance-upcoming-pl',
        ],
        'ids' => array_map(static fn (Event $event): int => $event->id, $fixtures),
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
        $fail('Usage: php scripts/acceptance/seed-browser-events.php seed-identity <email> <password> <recovery-code> <confirmed|unconfirmed> <permission-csv>', 2);
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
        $roleKey = 'acceptance_events_'.$identity->id;
        $now = now();
        $roleId = DB::table('admin_roles')->where('key', $roleKey)->value('id');
        if ($roleId === null) {
            $roleId = DB::table('admin_roles')->insertGetId([
                'key' => $roleKey,
                'name' => 'Acceptance Events role',
                'created_at' => $now,
                'updated_at' => $now,
            ]);
        }

        $roleId = $integerId($roleId, 'Acceptance Events role');
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
    $slug = $argv[2] ?? '';
    $translation = EventTranslation::query()->where('locale', 'en')->where('slug', $slug)->first();
    if (! $translation instanceof EventTranslation) {
        $fail("Event translation {$slug} was not found.");
    }

    Event::query()->whereKey($translation->event_id)->increment('lock_version');
    $event = Event::query()->findOrFail($translation->event_id);
    $json([
        'event_id' => $event->id,
        'lock_version' => $event->lock_version,
    ]);
}

$fail('Unknown Events acceptance fixture command.', 2);
