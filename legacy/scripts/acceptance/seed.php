<?php

declare(strict_types=1);

use App\Accounts\Models\IdentityCanaryAccount;
use App\Identity\Mfa\MfaRecoveryCodes;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use PragmaRX\Google2FA\Google2FA;

require __DIR__.'/../../vendor/autoload.php';

$app = require __DIR__.'/../../bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$command = $argv[1] ?? 'seed';

if ($command === 'empty-news') {
    DB::table('news_posts')->delete();
    fwrite(STDOUT, "acceptance-state: news emptied\n");
    exit(0);
}

if ($command === 'account-state') {
    $state = $argv[2] ?? '';
    $identity = Identity::query()->where('email', 'visual.user@example.test')->first();

    if (! $identity instanceof Identity) {
        fwrite(STDERR, "Visual acceptance identity is unavailable.\n");
        exit(2);
    }

    if ($state === 'missing') {
        IdentityCanaryAccount::query()->whereKey($identity->id)->delete();
        fwrite(STDOUT, "acceptance-state: account binding missing\n");
        exit(0);
    }

    $attributes = [
        'canary_account_id' => null,
        'provisioning_name' => 'op'.substr(hash('sha256', 'visual-user-account'), 0, 30),
        'canary_creation_epoch' => 2_000_000_001,
        'status' => IdentityCanaryAccount::STATUS_PENDING,
        'last_failure_code' => null,
        'last_attempt_at' => now()->subMinute(),
        'ready_at' => null,
    ];

    if ($state === 'ready') {
        $attributes['canary_account_id'] = 4_000_000_001;
        $attributes['status'] = IdentityCanaryAccount::STATUS_READY;
        $attributes['ready_at'] = now()->subMinute();
    } elseif ($state === 'conflict') {
        $attributes['status'] = IdentityCanaryAccount::STATUS_CONFLICT;
        $attributes['last_failure_code'] = 'binding_conflict';
    } elseif ($state !== 'pending') {
        fwrite(STDERR, "Unknown account acceptance state: {$state}\n");
        exit(2);
    }

    IdentityCanaryAccount::query()->updateOrCreate(
        ['identity_id' => $identity->id],
        $attributes,
    );

    fwrite(STDOUT, "acceptance-state: account binding {$state}\n");
    exit(0);
}

if ($command !== 'seed') {
    fwrite(STDERR, "Unknown acceptance seed command: {$command}\n");
    exit(2);
}

$now = now();

DB::table('news_posts')->delete();
DB::table('news_posts')->insert([
    [
        'slug' => 'welcome-to-oteryn',
        'title' => 'Welcome to the Oteryn Platform acceptance environment',
        'body' => "This is deterministic production-like acceptance content.\n\nIt intentionally contains multiple paragraphs so typography, spacing, wrapping, and long-form readability can be evaluated without using production data.",
        'published_at' => $now->copy()->subDay(),
        'created_at' => $now,
        'updated_at' => $now,
    ],
    [
        'slug' => 'long-title-layout-probe',
        'title' => 'A deliberately long published news title used to verify wrapping and visual hierarchy across narrow mobile viewports',
        'body' => str_repeat('Long-form acceptance content verifies that plain-text CMS output wraps safely and does not break the page layout. ', 8),
        'published_at' => $now->copy()->subHours(2),
        'created_at' => $now,
        'updated_at' => $now,
    ],
]);

for ($index = 3; $index <= 30; $index++) {
    DB::table('news_posts')->insert([
        'slug' => sprintf('pagination-probe-%02d', $index),
        'title' => sprintf('Pagination acceptance post %02d', $index),
        'body' => 'Deterministic acceptance content used only to force real public and administrator pagination rendering.',
        'published_at' => $now->copy()->subMinutes($index),
        'created_at' => $now,
        'updated_at' => $now->copy()->subMinutes($index),
    ]);
}

DB::table('managed_pages')->delete();
DB::table('managed_pages')->insert([
    'slug' => 'about-oteryn',
    'title' => 'About Oteryn',
    'body' => "Managed public page acceptance content.\n\nThis page is published and intentionally plain text so its real production rendering can be evaluated.",
    'published_at' => $now->copy()->subHour(),
    'created_at' => $now,
    'updated_at' => $now,
]);

$regular = Identity::query()->updateOrCreate(
    ['email' => 'visual.user@example.test'],
    ['password' => Hash::make('Acceptance-User-9!Pass')],
);
$regular->forceFill([
    'web_session_generation' => 0,
    'disabled_at' => null,
    'two_factor_secret' => null,
    'two_factor_recovery_codes' => null,
    'two_factor_confirmed_at' => null,
    'two_factor_last_used_timestep' => null,
])->save();

IdentityCanaryAccount::query()->updateOrCreate(
    ['identity_id' => $regular->id],
    [
        'canary_account_id' => 4_000_000_001,
        'provisioning_name' => 'op'.substr(hash('sha256', 'visual-user-account'), 0, 30),
        'canary_creation_epoch' => 2_000_000_001,
        'status' => IdentityCanaryAccount::STATUS_READY,
        'last_failure_code' => null,
        'last_attempt_at' => $now->copy()->subMinute(),
        'ready_at' => $now->copy()->subMinute(),
    ],
);

$admin = Identity::query()->updateOrCreate(
    ['email' => 'visual.admin@example.test'],
    ['password' => Hash::make('Acceptance-Admin-9!Pass')],
);

$recoveryCodes = [
    'ADMIN-00001',
    'ADMIN-00002',
    'ADMIN-00003',
    'ADMIN-00004',
];
$normalizer = new MfaRecoveryCodes;
$recoveryHashes = array_map(
    static fn (string $code): string => Hash::make($normalizer->normalize($code)),
    $recoveryCodes,
);

$admin->forceFill([
    'web_session_generation' => 0,
    'disabled_at' => null,
    'two_factor_secret' => (new Google2FA)->generateSecretKey(),
    'two_factor_recovery_codes' => $recoveryHashes,
    'two_factor_confirmed_at' => $now,
    'two_factor_last_used_timestep' => null,
])->save();

$observer = Identity::query()->updateOrCreate(
    ['email' => 'extremely.long.identity.address.for.mobile.table.layout.acceptance@example.test'],
    ['password' => Hash::make('Acceptance-Observer-9!Pass')],
);
$observer->forceFill([
    'web_session_generation' => 0,
    'disabled_at' => null,
    'two_factor_secret' => null,
    'two_factor_recovery_codes' => null,
    'two_factor_confirmed_at' => null,
    'two_factor_last_used_timestep' => null,
])->save();

$platformAdminRoleId = DB::table('admin_roles')->where('key', 'platform_admin')->value('id');
if (! is_int($platformAdminRoleId) && ! (is_string($platformAdminRoleId) && ctype_digit($platformAdminRoleId))) {
    throw new RuntimeException('platform_admin role is unavailable after migrations.');
}

DB::table('identity_admin_roles')->where('identity_id', $admin->id)->delete();
DB::table('identity_admin_roles')->insert([
    'identity_id' => $admin->id,
    'role_id' => (int) $platformAdminRoleId,
]);

DB::table('admin_audit_events')->delete();
DB::table('admin_audit_events')->insert([
    [
        'actor_identity_id' => $admin->id,
        'action' => 'cms.news.saved',
        'target_type' => 'news_post',
        'target_id' => '1',
        'metadata' => json_encode(['slug' => 'welcome-to-oteryn'], JSON_THROW_ON_ERROR),
        'occurred_at' => $now->copy()->subMinutes(20),
    ],
    [
        'actor_identity_id' => $admin->id,
        'action' => 'admin.role_assigned',
        'target_type' => 'identity',
        'target_id' => (string) $observer->id,
        'metadata' => json_encode([
            'role' => 'content_editor',
            'acceptance_probe' => 'Long metadata value used to verify table wrapping rather than leaking production audit content.',
        ], JSON_THROW_ON_ERROR),
        'occurred_at' => $now->copy()->subMinutes(10),
    ],
]);

fwrite(STDOUT, json_encode([
    'regular_identity_id' => $regular->id,
    'admin_identity_id' => $admin->id,
    'observer_identity_id' => $observer->id,
    'admin_recovery_codes' => $recoveryCodes,
], JSON_THROW_ON_ERROR | JSON_PRETTY_PRINT)."\n");
