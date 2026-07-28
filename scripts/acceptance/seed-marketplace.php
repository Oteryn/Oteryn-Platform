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

$listingRequestId = (string) Str::uuid();
$playerId = 4_000_000 + $seller->id;
$auction = CharacterAuction::query()->create([
    'listing_request_id' => $listingRequestId,
    'seller_identity_id' => $seller->id,
    'seller_canary_account_id' => 3_200_000_000 + $seller->id,
    'escrow_canary_account_id' => 3_299_999_999,
    'player_id' => $playerId,
    'active_player_id' => $playerId,
    'player_name' => 'Aurelia Market',
    'level' => 321,
    'vocation' => 4,
    'sex' => 1,
    'character_snapshot' => [
        'id' => $playerId,
        'name' => 'Aurelia Market',
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
    ],
    'status' => CharacterAuction::STATUS_ACTIVE,
    'saga_state' => CharacterAuction::SAGA_ACTIVE,
    'failure_code' => null,
    'duration_days' => 3,
    'starting_bid' => 100,
    'buy_now_price' => 1_000,
    'current_bid' => 0,
    'highest_bidder_identity_id' => null,
    'bid_count' => 0,
    'lock_version' => 1,
    'starts_at' => now()->subHour(),
    'ends_at' => now()->addDays(2),
    'escrowed_at' => now()->subHours(2),
]);

fwrite(STDOUT, json_encode([
    'bidder_identity_id' => $bidder->id,
    'email' => $bidder->email,
    'auction_id' => $auction->id,
    'player_name' => $auction->player_name,
], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES).PHP_EOL);
