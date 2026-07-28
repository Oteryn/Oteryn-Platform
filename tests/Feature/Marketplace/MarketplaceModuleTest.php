<?php

namespace Tests\Feature\Marketplace;

use App\Accounts\Models\IdentityCanaryAccount;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use App\Marketplace\Actions\PlaceAuctionBid;
use App\Marketplace\Actions\ReconcileCharacterAuctions;
use App\Marketplace\Contracts\CanaryCharacterTransferGateway;
use App\Marketplace\Data\CharacterOwnershipState;
use App\Marketplace\Data\CharacterSnapshot;
use App\Marketplace\Data\CharacterTransferResult;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Marketplace\Models\AuctionBid;
use App\Marketplace\Models\CharacterAuction;
use App\Wallet\Models\WalletAccount;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use RuntimeException;
use Tests\TestCase;

final class MarketplaceModuleTest extends TestCase
{
    use RefreshDatabase;

    protected function tearDown(): void
    {
        Carbon::setTestNow();
        CarbonImmutable::setTestNow();
        parent::tearDown();
    }

    public function test_public_catalogue_detail_and_localized_routes_render_safe_snapshot_data(): void
    {
        $auction = $this->createActiveAuction($this->createIdentity('seller@example.com'));

        $this->get(route('marketplace.index'))
            ->assertOk()
            ->assertSeeText('Character Bazaar')
            ->assertSeeText($auction->player_name)
            ->assertDontSeeText((string) $auction->seller_canary_account_id);

        $this->get(route('marketplace.show', $auction))
            ->assertOk()
            ->assertSeeText('Verified listing snapshot')
            ->assertSeeText('Sword fighting')
            ->assertDontSeeText((string) $auction->seller_canary_account_id);

        $this->get(route('marketplace.index', ['locale' => 'pl']))
            ->assertOk()
            ->assertSeeText('Bazar postaci');
    }

    public function test_listing_moves_owned_offline_character_to_escrow_and_activates_after_quiescence(): void
    {
        $at = CarbonImmutable::parse('2026-07-28 08:00:00', 'UTC');
        Carbon::setTestNow($at);
        CarbonImmutable::setTestNow($at);
        config()->set('marketplace.escrow_canary_account_id', 999);
        config()->set('marketplace.escrow_quiescence_seconds', 30);

        $seller = $this->createIdentity('listing-seller@example.com');
        $this->readyBinding($seller, 101);
        $gateway = new FakeCharacterTransferGateway([
            42 => ['account_id' => 101, 'name' => 'Escrow Candidate'],
        ]);
        $this->app->instance(CanaryCharacterTransferGateway::class, $gateway);
        $this->actingAsCurrent($seller);
        $requestId = (string) Str::uuid();

        $this->post(route('marketplace.listing.store'), [
            'player_id' => 42,
            'duration_days' => 3,
            'starting_bid' => 100,
            'buy_now_price' => 500,
            'request_id' => $requestId,
        ])->assertRedirect(route('marketplace.account'));

        $auction = CharacterAuction::query()->where('listing_request_id', $requestId)->firstOrFail();
        self::assertSame(CharacterAuction::STATUS_ESCROW_PENDING, $auction->status);
        self::assertSame(CharacterAuction::SAGA_QUIESCENCE_WAIT, $auction->saga_state);
        self::assertSame(999, $gateway->owners[42]);

        Carbon::setTestNow($at->addSeconds(31));
        CarbonImmutable::setTestNow($at->addSeconds(31));
        $active = app(ReconcileCharacterAuctions::class)->reconcile($auction);

        self::assertSame(CharacterAuction::STATUS_ACTIVE, $active->status);
        self::assertNotNull($active->starts_at);
        self::assertNotNull($active->ends_at);
        self::assertSame($active->player_id, $active->active_player_id);
    }

    public function test_direct_bids_reserve_release_and_reject_self_bidding_idempotently(): void
    {
        $seller = $this->createIdentity('bid-seller@example.com');
        $firstBidder = $this->createIdentity('bid-one@example.com');
        $secondBidder = $this->createIdentity('bid-two@example.com');
        $this->readyBinding($firstBidder, 201);
        $this->readyBinding($secondBidder, 202);
        $auction = $this->createActiveAuction($seller);
        WalletAccount::query()->create(['identity_id' => $firstBidder->id, 'available_balance' => 1000, 'reserved_balance' => 0]);
        WalletAccount::query()->create(['identity_id' => $secondBidder->id, 'available_balance' => 1000, 'reserved_balance' => 0]);
        $action = app(PlaceAuctionBid::class);
        $firstRequest = (string) Str::uuid();

        $action->execute($firstBidder, $auction, 200, $firstRequest);
        $action->execute($firstBidder, $auction, 200, $firstRequest);
        $action->execute($secondBidder, $auction->refresh(), 250, (string) Str::uuid());

        $firstWallet = WalletAccount::query()->findOrFail($firstBidder->id);
        $secondWallet = WalletAccount::query()->findOrFail($secondBidder->id);
        self::assertSame(1000, $firstWallet->available_balance);
        self::assertSame(0, $firstWallet->reserved_balance);
        self::assertSame(750, $secondWallet->available_balance);
        self::assertSame(250, $secondWallet->reserved_balance);
        self::assertSame(2, AuctionBid::query()->count());
        self::assertSame(AuctionBid::STATUS_OUTBID, AuctionBid::query()->where('bidder_identity_id', $firstBidder->id)->value('status'));
        self::assertSame(AuctionBid::STATUS_LEADING, AuctionBid::query()->where('bidder_identity_id', $secondBidder->id)->value('status'));

        $this->expectException(MarketplaceException::class);
        $action->execute($seller, $auction->refresh(), 300, (string) Str::uuid());
    }

    public function test_admin_wallet_adjustment_requires_mfa_permission_and_is_audited(): void
    {
        $target = $this->createIdentity('wallet-target@example.com');
        $noPermission = $this->createIdentity('wallet-denied@example.com');
        $this->actingAsCurrent($noPermission);
        $this->post(route('admin.marketplace.wallet.adjust'), [
            'email' => $target->email,
            'amount' => 500,
            'reason' => 'Initial controlled launch allocation.',
            'request_id' => (string) Str::uuid(),
        ])->assertForbidden();

        $admin = $this->createIdentity('wallet-admin@example.com');
        $this->grantPermissions($admin, ['marketplace.manage']);
        $this->actingAsCurrent($admin);
        $requestId = (string) Str::uuid();

        $this->post(route('admin.marketplace.wallet.adjust'), [
            'email' => $target->email,
            'amount' => 500,
            'reason' => 'Initial controlled launch allocation.',
            'request_id' => $requestId,
        ])->assertRedirect(route('admin.marketplace.index', ['email' => $target->email]));

        $this->assertDatabaseHas('wallet_accounts', [
            'identity_id' => $target->id,
            'available_balance' => 500,
            'reserved_balance' => 0,
        ]);
        $this->assertDatabaseHas('wallet_ledger_entries', [
            'identity_id' => $target->id,
            'operation_type' => 'administrator_adjustment',
            'idempotency_key' => 'admin-wallet-adjustment:'.$requestId,
        ]);
        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $admin->id,
            'action' => 'marketplace.wallet_adjusted',
            'target_id' => (string) $target->id,
        ]);
    }

    private function createActiveAuction(Identity $seller): CharacterAuction
    {
        return CharacterAuction::query()->create([
            'listing_request_id' => (string) Str::uuid(),
            'seller_identity_id' => $seller->id,
            'seller_canary_account_id' => 1001,
            'escrow_canary_account_id' => 9999,
            'player_id' => random_int(10_000, 99_999),
            'active_player_id' => random_int(100_000, 999_999),
            'player_name' => 'Aurelia Test',
            'level' => 321,
            'vocation' => 4,
            'sex' => 1,
            'character_snapshot' => $this->snapshot('Aurelia Test', 321, 4, 1),
            'status' => CharacterAuction::STATUS_ACTIVE,
            'saga_state' => CharacterAuction::SAGA_ACTIVE,
            'duration_days' => 3,
            'starting_bid' => 100,
            'buy_now_price' => 1000,
            'current_bid' => 0,
            'bid_count' => 0,
            'lock_version' => 1,
            'starts_at' => now()->subHour(),
            'ends_at' => now()->addDay(),
            'escrowed_at' => now()->subHours(2),
        ]);
    }

    /** @return array<string, int|string|null> */
    private function snapshot(string $name, int $level, int $vocation, int $sex): array
    {
        return [
            'id' => 42,
            'name' => $name,
            'level' => $level,
            'vocation' => $vocation,
            'experience' => 123456789,
            'sex' => $sex,
            'maglevel' => 12,
            'skill_fist' => 15,
            'skill_club' => 20,
            'skill_sword' => 110,
            'skill_axe' => 21,
            'skill_dist' => 18,
            'skill_shielding' => 105,
            'skill_fishing' => 40,
            'looktype' => 128,
            'lookaddons' => 3,
            'lookhead' => 1,
            'lookbody' => 2,
            'looklegs' => 3,
            'lookfeet' => 4,
            'town_id' => 8,
            'lastlogin' => 1_700_000_000,
            'lastlogout' => 1_700_000_100,
        ];
    }

    private function createIdentity(string $email): Identity
    {
        $identity = Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
        $identity->forceFill([
            'two_factor_secret' => 'TEST-MFA-SECRET-NOT-REAL',
            'two_factor_confirmed_at' => now(),
        ])->save();

        return $identity;
    }

    private function readyBinding(Identity $identity, int $canaryAccountId): void
    {
        IdentityCanaryAccount::query()->create([
            'identity_id' => $identity->id,
            'canary_account_id' => $canaryAccountId,
            'provisioning_name' => 'oteryn_test_'.$identity->id,
            'canary_creation_epoch' => 1,
            'status' => IdentityCanaryAccount::STATUS_READY,
            'ready_at' => now(),
        ]);
    }

    /** @param  list<string>  $permissions */
    private function grantPermissions(Identity $identity, array $permissions): void
    {
        $now = now();
        $roleId = DB::table('admin_roles')->insertGetId([
            'key' => 'marketplace-role-'.$identity->id,
            'name' => 'Marketplace test role',
            'created_at' => $now,
            'updated_at' => $now,
        ]);

        foreach ($permissions as $permission) {
            $permissionId = $this->integerDatabaseValue(
                DB::table('admin_permissions')->where('key', $permission)->value('id'),
                "permission {$permission}",
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

    private function integerDatabaseValue(mixed $value, string $description): int
    {
        if (is_int($value)) {
            return $value;
        }
        if (is_string($value) && ctype_digit($value)) {
            return (int) $value;
        }

        throw new RuntimeException("Expected an integer-compatible {$description} id.");
    }

    private function actingAsCurrent(Identity $identity): void
    {
        $currentIdentity = Identity::query()->findOrFail($identity->id);
        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => $currentIdentity->web_session_generation]);
    }
}

final class FakeCharacterTransferGateway implements CanaryCharacterTransferGateway
{
    /** @var array<int, int> */
    public array $owners = [];

    /** @var array<int, string> */
    private array $names = [];

    /** @param  array<int, array{account_id: int, name: string}>  $characters */
    public function __construct(array $characters)
    {
        foreach ($characters as $playerId => $character) {
            $this->owners[$playerId] = $character['account_id'];
            $this->names[$playerId] = $character['name'];
        }
    }

    public function snapshotOwnedCharacter(int $sourceAccountId, int $playerId): CharacterSnapshot
    {
        if (($this->owners[$playerId] ?? null) !== $sourceAccountId) {
            throw new MarketplaceException('ownership_conflict', 'Character ownership does not match.');
        }

        $name = $this->names[$playerId] ?? 'Unknown';

        return new CharacterSnapshot(
            $playerId,
            $sourceAccountId,
            $name,
            0,
            [
                'id' => $playerId,
                'name' => $name,
                'level' => 150,
                'vocation' => 3,
                'experience' => 10_000,
                'sex' => 1,
                'maglevel' => 20,
                'skill_fist' => 10,
                'skill_club' => 10,
                'skill_sword' => 10,
                'skill_axe' => 10,
                'skill_dist' => 100,
                'skill_shielding' => 90,
                'skill_fishing' => 10,
                'looktype' => 128,
                'lookaddons' => 0,
                'lookhead' => 1,
                'lookbody' => 2,
                'looklegs' => 3,
                'lookfeet' => 4,
                'town_id' => 8,
                'lastlogin' => 1_700_000_000,
                'lastlogout' => 1_700_000_100,
            ],
        );
    }

    public function transfer(
        int $playerId,
        int $expectedSourceAccountId,
        int $targetAccountId,
        bool $enforceTargetCharacterLimit,
    ): CharacterTransferResult {
        $currentOwner = $this->owners[$playerId] ?? 0;
        if ($currentOwner === $targetAccountId) {
            return new CharacterTransferResult(CharacterTransferResult::ALREADY_TRANSFERRED, $playerId, $targetAccountId);
        }
        if ($currentOwner !== $expectedSourceAccountId) {
            throw new MarketplaceException('ownership_conflict', 'Character ownership changed.');
        }

        $this->owners[$playerId] = $targetAccountId;

        return new CharacterTransferResult(CharacterTransferResult::TRANSFERRED, $playerId, $targetAccountId);
    }

    public function ownershipState(int $playerId): CharacterOwnershipState
    {
        return new CharacterOwnershipState($playerId, $this->owners[$playerId] ?? 0, 0, false);
    }
}
