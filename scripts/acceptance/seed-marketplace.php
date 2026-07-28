<?php

declare(strict_types=1);

use App\Accounts\Models\IdentityCanaryAccount;
use App\Identity\Models\Identity;
use App\Marketplace\Models\CharacterAuction;
use App\Wallet\Models\WalletAccount;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;

require __DIR__.'/../../vendor/autoload.php';

$app = require __DIR__.'/../../bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

if (! $app->environment('acceptance')) {
    fwrite(STDERR, "Character Bazaar fixture seeding is restricted to the acceptance environment.\n");
    exit(2);
}

$email = $argv[1] ?? '';
$password = $argv[2] ?? '';

if ($email === '' || $password === '') {
    fwrite(STDERR, "Usage: php scripts/acceptance/seed-marketplace.php <bidder-email> <bidder-password>\n");
    exit(2);
}

$bidder = Identity::query()->updateOrCreate(
    ['email' => $email],
    ['password' => Hash::make($password)],
);
$bidder->forceFill([
    'web_session_generation' => 0,
    'disabled_at' => null,
    'two_factor_secret' => null,
    'two_factor_recovery_codes' => null,
    'two_factor_confirmed_at' => null,
    'two_factor_last_used_timestep' => null,
])->save();

$accountId = 3_100_000_000 + $bidder->id;
IdentityCanaryAccount::query()->updateOrCreate(
    ['identity_id' => $bidder->id],
    [
        'canary_account_id' => $accountId,
        'provisioning_name' => 'mp'.substr(hash('sha256', $email), 0, 30),
        'canary_creation_epoch' => 2_000_200_000 + $bidder->id,
        'status' => IdentityCanaryAccount::STATUS_READY,
        'last_failure_code' => null,
        'last_attempt_at' => now()->subMinute(),
        'ready_at' => now()->subMinute(),
    ],
);
WalletAccount::query()->updateOrCreate(
    ['identity_id' => $bidder->id],
    ['available_balance' => 5_000, 'reserved_balance' => 0],
);

$sellerEmail = 'marketplace-seller-'.substr(hash('sha256', $email), 0, 16).'@example.test';
$seller = Identity::query()->updateOrCreate(
    ['email' => $sellerEmail],
    ['password' => Hash::make(Str::random(48))],
);

/** @return array<string, int|string|null> */
$snapshot = static function (int $playerId, string $name): array {
    return [
        'id' => $playerId,
        'name' => $name,
        'level' => 321,
        'vocation' => 4,
        'experience' => 123_456_789,
        'sex' => 1,
        'maglevel' => 12,
        'skill_fist' => 18,
        'skill_club' => 20,
        'skill_sword' => 112,
        'skill_axe' => 22,
        'skill_dist' => 19,
        'skill_shielding' => 108,
        'skill_fishing' => 41,
        'looktype' => 128,
        'lookaddons' => 3,
        'lookhead' => 78,
        'lookbody' => 94,
        'looklegs' => 110,
        'lookfeet' => 76,
        'town_id' => 8,
        'lastlogin' => now()->subDays(2)->timestamp,
        'lastlogout' => now()->subDays(2)->addMinutes(30)->timestamp,
    ];
};

/**
 * @param  array<string, mixed>  $overrides
 */
$createAuction = static function (
    Identity $owner,
    int $ownerAccountId,
    int $playerId,
    string $name,
    string $status,
    array $overrides = [],
) use ($snapshot): CharacterAuction {
    return CharacterAuction::query()->create(array_merge([
        'listing_request_id' => (string) Str::uuid(),
        'seller_identity_id' => $owner->id,
        'seller_canary_account_id' => $ownerAccountId,
        'escrow_canary_account_id' => 3_299_999_999,
        'player_id' => $playerId,
        'active_player_id' => $playerId,
        'player_name' => $name,
        'level' => 321,
        'vocation' => 4,
        'sex' => 1,
        'character_snapshot' => $snapshot($playerId, $name),
        'status' => $status,
        'saga_state' => $status === CharacterAuction::STATUS_ACTIVE
            ? CharacterAuction::SAGA_ACTIVE
            : CharacterAuction::SAGA_RECOVERY_REQUIRED,
        'failure_code' => null,
        'duration_days' => 3,
        'starting_bid' => 100,
        'buy_now_price' => 1_000,
        'current_bid' => 0,
        'highest_bidder_identity_id' => null,
        'bid_count' => 0,
        'lock_version' => 1,
        'starts_at' => null,
        'ends_at' => null,
        'escrowed_at' => null,
        'settlement_started_at' => null,
        'settled_at' => null,
        'cancelled_at' => null,
    ], $overrides));
};

$activePlayerId = 4_000_000 + $seller->id;
$auction = $createAuction(
    $seller,
    3_200_000_000 + $seller->id,
    $activePlayerId,
    'Aurelia Market',
    CharacterAuction::STATUS_ACTIVE,
    [
        'saga_state' => CharacterAuction::SAGA_ACTIVE,
        'starts_at' => now()->subHour(),
        'ends_at' => now()->addDays(2),
        'escrowed_at' => now()->subHours(2),
    ],
);

$pending = $createAuction(
    $bidder,
    $accountId,
    5_000_000 + $bidder->id,
    'Pending Escrow',
    CharacterAuction::STATUS_ESCROW_PENDING,
    [
        'saga_state' => CharacterAuction::SAGA_QUIESCENCE_WAIT,
        'escrowed_at' => now(),
    ],
);
$recovery = $createAuction(
    $bidder,
    $accountId,
    6_000_000 + $bidder->id,
    'Recovery Required',
    CharacterAuction::STATUS_RECOVERY_REQUIRED,
    [
        'saga_state' => CharacterAuction::SAGA_RECOVERY_REQUIRED,
        'failure_code' => 'acceptance_recovery_fixture',
        'escrowed_at' => now()->subMinute(),
    ],
);
$cancelled = $createAuction(
    $bidder,
    $accountId,
    7_000_000 + $bidder->id,
    'Cancelled Listing',
    CharacterAuction::STATUS_CANCELLED,
    [
        'saga_state' => CharacterAuction::SAGA_DONE,
        'active_player_id' => null,
        'starts_at' => now()->subDays(2),
        'ends_at' => now()->subDay(),
        'escrowed_at' => now()->subDays(2),
        'cancelled_at' => now()->subDay(),
    ],
);
$completed = $createAuction(
    $seller,
    3_200_000_000 + $seller->id,
    8_000_000 + $bidder->id,
    'Completed Purchase',
    CharacterAuction::STATUS_COMPLETED,
    [
        'saga_state' => CharacterAuction::SAGA_DONE,
        'active_player_id' => null,
        'highest_bidder_identity_id' => $bidder->id,
        'current_bid' => 750,
        'bid_count' => 1,
        'starts_at' => now()->subDays(4),
        'ends_at' => now()->subDay(),
        'escrowed_at' => now()->subDays(4),
        'settlement_started_at' => now()->subDay(),
        'settled_at' => now()->subDay(),
    ],
);

fwrite(STDOUT, json_encode([
    'bidder_identity_id' => $bidder->id,
    'email' => $bidder->email,
    'auction_id' => $auction->id,
    'player_name' => $auction->player_name,
    'pending_player_name' => $pending->player_name,
    'recovery_player_name' => $recovery->player_name,
    'cancelled_player_name' => $cancelled->player_name,
    'completed_player_name' => $completed->player_name,
], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES).PHP_EOL);
