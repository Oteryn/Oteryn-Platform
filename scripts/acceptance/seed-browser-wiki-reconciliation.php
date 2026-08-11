<?php

declare(strict_types=1);

use App\Identity\Mfa\MfaRecoveryCodes;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use PragmaRX\Google2FA\Google2FA;

require dirname(__DIR__, 2).'/vendor/autoload.php';

$app = require dirname(__DIR__, 2).'/bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$command = $argv[1] ?? '';
$unavailableTable = 'wiki_article_translations_acceptance_unavailable';
$adminUnavailableTable = 'wiki_categories_acceptance_unavailable';

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
    if (Schema::hasTable($unavailableTable) && ! Schema::hasTable('wiki_article_translations')) {
        Schema::rename($unavailableTable, 'wiki_article_translations');
    }
};

$restoreAdminAvailability = static function () use ($adminUnavailableTable): void {
    if (Schema::hasTable($adminUnavailableTable) && ! Schema::hasTable('wiki_categories')) {
        Schema::rename($adminUnavailableTable, 'wiki_categories');
    }
};

$reset = static function () use ($restoreAvailability, $restoreAdminAvailability): void {
    $restoreAvailability();
    $restoreAdminAvailability();

    DB::transaction(static function (): void {
        DB::table('wiki_revisions')->update(['source_revision_id' => null]);
        DB::table('wiki_revisions')->delete();
        DB::table('wiki_article_category')->delete();
        DB::table('wiki_article_translations')->delete();
        DB::table('wiki_category_translations')->delete();
        DB::table('wiki_articles')->delete();
        DB::table('wiki_categories')->delete();
        DB::table('admin_audit_events')
            ->whereIn('target_type', ['wiki_article', 'wiki_category'])
            ->delete();
    });
};

if ($command === 'reset') {
    $reset();
    $json(['reset' => true]);
}

if ($command === 'set-public-unavailable') {
    $restoreAvailability();

    if (! Schema::hasTable('wiki_article_translations')) {
        $fail('Wiki translation table is unavailable before failure injection.');
    }

    Schema::rename('wiki_article_translations', $unavailableTable);
    $json(['unavailable' => true]);
}

if ($command === 'restore-public') {
    $restoreAvailability();
    $json(['restored' => Schema::hasTable('wiki_article_translations')]);
}

if ($command === 'set-admin-unavailable') {
    $restoreAdminAvailability();

    if (! Schema::hasTable('wiki_categories')) {
        $fail('Wiki category table is unavailable before administrator failure injection.');
    }

    Schema::rename('wiki_categories', $adminUnavailableTable);
    $json(['admin_unavailable' => true]);
}

if ($command === 'restore-admin') {
    $restoreAdminAvailability();
    $json(['admin_restored' => Schema::hasTable('wiki_categories')]);
}

if ($command === 'seed-identity') {
    $email = $argv[2] ?? '';
    $password = $argv[3] ?? '';
    $recoveryCode = $argv[4] ?? '';
    $mfaConfirmed = ($argv[5] ?? '') === 'confirmed';
    $permissionCsv = $argv[6] ?? '';
    $permissions = array_values(array_filter(array_map('trim', explode(',', $permissionCsv))));

    if ($email === '' || $password === '') {
        $fail('Usage: php scripts/acceptance/seed-browser-wiki-reconciliation.php seed-identity <email> <password> <recovery-code> <confirmed|unconfirmed> <permission-csv>', 2);
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
        $roleKey = 'acceptance_wiki_reconciliation_'.$identity->id;
        $now = now();
        $roleId = DB::table('admin_roles')->where('key', $roleKey)->value('id');

        if ($roleId === null) {
            $roleId = DB::table('admin_roles')->insertGetId([
                'key' => $roleKey,
                'name' => 'Acceptance Wiki reconciliation role',
                'created_at' => $now,
                'updated_at' => $now,
            ]);
        }

        $roleId = $integerId($roleId, 'Acceptance Wiki reconciliation role');
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

if ($command === 'bump-article-lock') {
    $articleId = $integerId($argv[2] ?? null, 'Wiki article id');
    DB::table('wiki_articles')->where('id', $articleId)->increment('lock_version');
    $lockVersion = $integerId(
        DB::table('wiki_articles')->where('id', $articleId)->value('lock_version'),
        'Wiki article lock version',
    );
    $json(['article_id' => $articleId, 'lock_version' => $lockVersion]);
}

$fail('Unknown Wiki reconciliation acceptance fixture command.', 2);
