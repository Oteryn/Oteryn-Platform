<?php

namespace Tests\Feature\Marketplace;

use App\Accounts\Models\IdentityCanaryAccount;
use App\Identity\Models\Identity;
use App\Marketplace\Actions\CancelCharacterAuction;
use App\Marketplace\Actions\PlaceAuctionBid;
use App\Marketplace\Actions\ReconcileCharacterAuctions;
use App\Marketplace\Actions\RecoverCharacterAuction;
use App\Marketplace\Contracts\CanaryCharacterTransferGateway;
use App\Marketplace\Data\CharacterOwnershipState;
use App\Marketplace\Data\CharacterSnapshot;
use App\Marketplace\Data\CharacterTransferResult;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Marketplace\Models\AuctionBid;
use App\Marketplace\Models\CharacterAuction;
use App\Wallet\Models\WalletAccount;
use App\Wallet\Models\WalletLedgerEntry;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use Tests\TestCase;

final class MarketplaceSettlementRecoveryTest extends TestCase
{
    use RefreshDatabase;

    protected function tearDown(): void
    {
        Carbon::setTestNow();
        CarbonImmutable::setTestNow();
        parent::tearDown();
    }

    public function test_buy_now_transfers_character_and_settles_reserved_coins_exactly_once(): void
    {
        config()->set('marketplace.commission_basis_points', 1000);

        $seller = $this->createIdentity('settlement-seller@example.com');
        $winner = $this->createIdentity('settlement-winner@example.com');
        $this->readyBinding($seller, 101);
        $this->readyBinding($winner, 202);
        $auction = $this->createAuction($seller, 101, 999, 41, CharacterAuction::STATUS_ACTIVE, [
            'starts_at' => now()->subHour(),
            'ends_at' => now()->addDay(),
            'escrowed_at' => now()->subHours(2),
        ]);
        $gateway = new SettlementCharacterTransferGateway([41 => 999]);
        $this->app->instance(CanaryCharacterTransferGateway::class, $gateway);
        WalletAccount::query()->create([
            'identity_id' => $winner->id,
            'available_balance' => 2_000,
            'reserved_balance' => 0,
        ]);

        $pending = app(PlaceAuctionBid::class)->execute(
            $winner,
            $auction,
            1_000,
            (string) Str::uuid(),
            true,
        );

        self::assertSame(CharacterAuction::STATUS_SETTLEMENT_PENDING, $pending->status);
        self::assertSame(999, $gateway->owner(41));
        self::assertSame(1_000, WalletAccount::query()->findOrFail($winner->id)->reserved_balance);

        $completed = app(ReconcileCharacterAuctions::class)->reconcile($pending);

        self::assertSame(CharacterAuction::STATUS_COMPLETED, $completed->status);
        self::assertSame(CharacterAuction::SAGA_DONE, $completed->saga_state);
        self::assertNull($completed->active_player_id);
        self::assertSame(202, $gateway->owner(41));
        self::assertSame(1_000, WalletAccount::query()->findOrFail($winner->id)->available_balance);
        self::assertSame(0, WalletAccount::query()->findOrFail($winner->id)->reserved_balance);
        self::assertSame(900, WalletAccount::query()->findOrFail($seller->id)->available_balance);
        self::assertSame(AuctionBid::STATUS_WON, AuctionBid::query()->where('auction_id', $auction->id)->value('status'));
        self::assertSame(1, WalletLedgerEntry::query()->where('operation_type', 'auction_purchase_settled')->count());
        self::assertSame(1, WalletLedgerEntry::query()->where('operation_type', 'auction_sale_proceeds')->count());

        app(ReconcileCharacterAuctions::class)->reconcile($completed);

        self::assertSame(900, WalletAccount::query()->findOrFail($seller->id)->available_balance);
        self::assertSame(1, WalletLedgerEntry::query()->where('operation_type', 'auction_purchase_settled')->count());
        self::assertSame(1, WalletLedgerEntry::query()->where('operation_type', 'auction_sale_proceeds')->count());
    }

    public function test_user_cancellation_and_no_bid_expiry_return_character_to_seller(): void
    {
        $seller = $this->createIdentity('return-seller@example.com');
        $this->readyBinding($seller, 301);
        $cancelledAuction = $this->createAuction($seller, 301, 999, 51, CharacterAuction::STATUS_ACTIVE, [
            'starts_at' => now()->subHour(),
            'ends_at' => now()->addDay(),
            'escrowed_at' => now()->subHours(2),
        ]);
        $expiredAuction = $this->createAuction($seller, 301, 999, 52, CharacterAuction::STATUS_ACTIVE, [
            'starts_at' => now()->subDays(4),
            'ends_at' => now()->subMinute(),
            'escrowed_at' => now()->subDays(4),
        ]);
        $gateway = new SettlementCharacterTransferGateway([51 => 999, 52 => 999]);
        $this->app->instance(CanaryCharacterTransferGateway::class, $gateway);

        $cancelled = app(CancelCharacterAuction::class)->execute($seller, $cancelledAuction);
        $expired = app(ReconcileCharacterAuctions::class)->reconcile($expiredAuction);

        self::assertSame(CharacterAuction::STATUS_CANCELLED, $cancelled->status);
        self::assertSame(CharacterAuction::STATUS_EXPIRED, $expired->status);
        self::assertSame(301, $gateway->owner(51));
        self::assertSame(301, $gateway->owner(52));
        self::assertNull($cancelled->active_player_id);
        self::assertNull($expired->active_player_id);
    }

    public function test_recovery_resumes_transfer_success_before_platform_persistence(): void
    {
        $at = CarbonImmutable::parse('2026-07-28 09:00:00', 'UTC');
        Carbon::setTestNow($at);
        CarbonImmutable::setTestNow($at);
        config()->set('marketplace.escrow_quiescence_seconds', 30);

        $seller = $this->createIdentity('escrow-recovery-seller@example.com');
        $this->readyBinding($seller, 401);
        $auction = $this->createAuction($seller, 401, 999, 61, CharacterAuction::STATUS_RECOVERY_REQUIRED, [
            'saga_state' => CharacterAuction::SAGA_RECOVERY_REQUIRED,
            'failure_code' => 'platform_persistence_unavailable',
        ]);
        $gateway = new SettlementCharacterTransferGateway([61 => 999]);
        $this->app->instance(CanaryCharacterTransferGateway::class, $gateway);

        $pending = app(RecoverCharacterAuction::class)->execute($auction);

        self::assertSame(CharacterAuction::STATUS_ESCROW_PENDING, $pending->status);
        self::assertSame(CharacterAuction::SAGA_QUIESCENCE_WAIT, $pending->saga_state);
        self::assertNotNull($pending->escrowed_at);

        Carbon::setTestNow($at->addSeconds(31));
        CarbonImmutable::setTestNow($at->addSeconds(31));
        $active = app(ReconcileCharacterAuctions::class)->reconcile($pending);

        self::assertSame(CharacterAuction::STATUS_ACTIVE, $active->status);
        self::assertSame(CharacterAuction::SAGA_ACTIVE, $active->saga_state);
        self::assertSame(999, $gateway->owner(61));
    }

    public function test_recovery_cancels_before_transfer_and_finishes_wallet_after_winner_transfer(): void
    {
        config()->set('marketplace.commission_basis_points', 1000);

        $seller = $this->createIdentity('interruption-seller@example.com');
        $winner = $this->createIdentity('interruption-winner@example.com');
        $this->readyBinding($seller, 501);
        $this->readyBinding($winner, 502);
        $beforeTransfer = $this->createAuction($seller, 501, 999, 71, CharacterAuction::STATUS_RECOVERY_REQUIRED, [
            'saga_state' => CharacterAuction::SAGA_RECOVERY_REQUIRED,
            'failure_code' => 'dependency_unavailable',
        ]);
        $afterWinnerTransfer = $this->createAuction($seller, 501, 999, 72, CharacterAuction::STATUS_RECOVERY_REQUIRED, [
            'saga_state' => CharacterAuction::SAGA_RECOVERY_REQUIRED,
            'failure_code' => 'platform_persistence_unavailable',
            'highest_bidder_identity_id' => $winner->id,
            'current_bid' => 500,
            'bid_count' => 1,
            'starts_at' => now()->subDays(2),
            'ends_at' => now()->subMinute(),
            'escrowed_at' => now()->subDays(2),
            'settlement_started_at' => now()->subMinute(),
        ]);
        AuctionBid::query()->create([
            'request_id' => (string) Str::uuid(),
            'auction_id' => $afterWinnerTransfer->id,
            'bidder_identity_id' => $winner->id,
            'amount' => 500,
            'is_buy_now' => false,
            'status' => AuctionBid::STATUS_LEADING,
            'placed_at' => now()->subHour(),
            'updated_at' => now()->subHour(),
        ]);
        WalletAccount::query()->create([
            'identity_id' => $winner->id,
            'available_balance' => 500,
            'reserved_balance' => 500,
        ]);
        $gateway = new SettlementCharacterTransferGateway([71 => 501, 72 => 502]);
        $this->app->instance(CanaryCharacterTransferGateway::class, $gateway);

        $cancelled = app(RecoverCharacterAuction::class)->execute($beforeTransfer);
        $completed = app(RecoverCharacterAuction::class)->execute($afterWinnerTransfer);

        self::assertSame(CharacterAuction::STATUS_CANCELLED, $cancelled->status);
        self::assertSame(501, $gateway->owner(71));
        self::assertSame(CharacterAuction::STATUS_COMPLETED, $completed->status);
        self::assertSame(502, $gateway->owner(72));
        self::assertSame(0, WalletAccount::query()->findOrFail($winner->id)->reserved_balance);
        self::assertSame(450, WalletAccount::query()->findOrFail($seller->id)->available_balance);
    }

    private function createIdentity(string $email): Identity
    {
        return Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }

    private function readyBinding(Identity $identity, int $canaryAccountId): void
    {
        IdentityCanaryAccount::query()->create([
            'identity_id' => $identity->id,
            'canary_account_id' => $canaryAccountId,
            'provisioning_name' => 'oteryn_settlement_'.$identity->id,
            'canary_creation_epoch' => 1,
            'status' => IdentityCanaryAccount::STATUS_READY,
            'ready_at' => now(),
        ]);
    }

    /** @param array<string, mixed> $overrides */
    private function createAuction(
        Identity $seller,
        int $sellerAccountId,
        int $escrowAccountId,
        int $playerId,
        string $status,
        array $overrides = [],
    ): CharacterAuction {
        return CharacterAuction::query()->create(array_merge([
            'listing_request_id' => (string) Str::uuid(),
            'seller_identity_id' => $seller->id,
            'seller_canary_account_id' => $sellerAccountId,
            'escrow_canary_account_id' => $escrowAccountId,
            'player_id' => $playerId,
            'active_player_id' => $playerId,
            'player_name' => 'Settlement Hero '.$playerId,
            'level' => 250,
            'vocation' => 4,
            'sex' => 1,
            'character_snapshot' => [
                'id' => $playerId,
                'name' => 'Settlement Hero '.$playerId,
                'level' => 250,
                'vocation' => 4,
                'sex' => 1,
            ],
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
        ], $overrides));
    }
}

final class SettlementCharacterTransferGateway implements CanaryCharacterTransferGateway
{
    /** @param array<int, int> $owners */
    public function __construct(private array $owners) {}

    public function owner(int $playerId): int
    {
        return $this->owners[$playerId] ?? 0;
    }

    public function snapshotOwnedCharacter(int $sourceAccountId, int $playerId): CharacterSnapshot
    {
        if ($this->owner($playerId) !== $sourceAccountId) {
            throw new MarketplaceException('ownership_conflict', 'Character ownership does not match.');
        }

        return new CharacterSnapshot(
            playerId: $playerId,
            accountId: $sourceAccountId,
            name: 'Settlement Hero '.$playerId,
            deletion: 0,
            publicData: [
                'id' => $playerId,
                'name' => 'Settlement Hero '.$playerId,
                'level' => 250,
                'vocation' => 4,
                'sex' => 1,
            ],
        );
    }

    public function transfer(
        int $playerId,
        int $expectedSourceAccountId,
        int $targetAccountId,
        bool $enforceTargetCharacterLimit,
    ): CharacterTransferResult {
        $currentOwner = $this->owner($playerId);
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
        return new CharacterOwnershipState($playerId, $this->owner($playerId), 0, false);
    }
}
