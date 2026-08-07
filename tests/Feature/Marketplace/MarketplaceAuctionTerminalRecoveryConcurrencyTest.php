<?php

namespace Tests\Feature\Marketplace;

use App\Accounts\Models\IdentityCanaryAccount;
use App\Identity\Models\Identity;
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
use App\Wallet\Models\WalletLedgerEntry;
use App\Wallet\WalletMutator;
use Closure;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use Tests\TestCase;

final class MarketplaceAuctionTerminalRecoveryConcurrencyTest extends TestCase
{
    use RefreshDatabase;

    public function test_stale_settlement_failure_cannot_regress_completed_auction_or_wallet_truth(): void
    {
        config()->set('marketplace.commission_basis_points', 1000);

        $seller = $this->createIdentity('terminal-race-seller@example.com');
        $winner = $this->createIdentity('terminal-race-winner@example.com');
        $this->readyBinding($seller, 1101);
        $this->readyBinding($winner, 1102);

        $auction = $this->createAuction($seller, 1101, 1999, 181, CharacterAuction::STATUS_ACTIVE, [
            'starts_at' => now()->subHour(),
            'ends_at' => now()->addDay(),
            'escrowed_at' => now()->subHours(2),
        ]);

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

        $wallets = app(WalletMutator::class);
        $successfulGateway = new TerminalRecoveryCharacterTransferGateway([181 => 1999]);
        $staleGateway = new TerminalRecoveryFailingGateway(function () use ($pending, $successfulGateway, $wallets): void {
            $concurrent = new ReconcileCharacterAuctions($successfulGateway, $wallets);
            $completed = $concurrent->reconcile(CharacterAuction::query()->findOrFail($pending->id));

            self::assertSame(CharacterAuction::STATUS_COMPLETED, $completed->status);
            self::assertSame(CharacterAuction::SAGA_DONE, $completed->saga_state);
        });

        $result = (new ReconcileCharacterAuctions($staleGateway, $wallets))->reconcile($pending);

        self::assertSame(CharacterAuction::STATUS_COMPLETED, $result->status);
        self::assertSame(CharacterAuction::SAGA_DONE, $result->saga_state);
        self::assertNull($result->failure_code);
        self::assertNull($result->active_player_id);
        self::assertSame(1102, $successfulGateway->owner(181));
        self::assertSame(AuctionBid::STATUS_WON, AuctionBid::query()->where('auction_id', $auction->id)->value('status'));
        self::assertSame(1_000, WalletAccount::query()->findOrFail($winner->id)->available_balance);
        self::assertSame(0, WalletAccount::query()->findOrFail($winner->id)->reserved_balance);
        self::assertSame(900, WalletAccount::query()->findOrFail($seller->id)->available_balance);
        self::assertSame(1, WalletLedgerEntry::query()->where('operation_type', 'auction_purchase_settled')->count());
        self::assertSame(1, WalletLedgerEntry::query()->where('operation_type', 'auction_sale_proceeds')->count());
    }

    public function test_stale_return_failure_cannot_regress_cancelled_or_expired_terminal_state(): void
    {
        $seller = $this->createIdentity('terminal-return-seller@example.com');
        $this->readyBinding($seller, 1201);
        $wallets = app(WalletMutator::class);

        foreach ([
            ['player_id' => 191, 'failure_code' => null, 'expected' => CharacterAuction::STATUS_CANCELLED],
            ['player_id' => 192, 'failure_code' => 'expired_no_bid', 'expected' => CharacterAuction::STATUS_EXPIRED],
        ] as $case) {
            $auction = $this->createAuction($seller, 1201, 1999, $case['player_id'], CharacterAuction::STATUS_CANCEL_PENDING, [
                'saga_state' => CharacterAuction::SAGA_RETURN_TO_SELLER,
                'failure_code' => $case['failure_code'],
                'starts_at' => now()->subDays(4),
                'ends_at' => now()->subMinute(),
                'escrowed_at' => now()->subDays(4),
            ]);

            $successfulGateway = new TerminalRecoveryCharacterTransferGateway([$case['player_id'] => 1999]);
            $staleGateway = new TerminalRecoveryFailingGateway(function () use ($auction, $successfulGateway, $wallets, $case): void {
                $concurrent = new ReconcileCharacterAuctions($successfulGateway, $wallets);
                $terminal = $concurrent->reconcile(CharacterAuction::query()->findOrFail($auction->id));

                self::assertSame($case['expected'], $terminal->status);
                self::assertSame(CharacterAuction::SAGA_DONE, $terminal->saga_state);
            });

            $result = (new ReconcileCharacterAuctions($staleGateway, $wallets))->reconcile($auction);

            self::assertSame($case['expected'], $result->status);
            self::assertSame(CharacterAuction::SAGA_DONE, $result->saga_state);
            self::assertNull($result->active_player_id);
            self::assertSame(1201, $successfulGateway->owner($case['player_id']));
        }
    }

    public function test_genuine_non_terminal_settlement_failure_still_enters_recovery(): void
    {
        $seller = $this->createIdentity('recoverable-failure-seller@example.com');
        $winner = $this->createIdentity('recoverable-failure-winner@example.com');
        $this->readyBinding($seller, 1301);
        $this->readyBinding($winner, 1302);

        $auction = $this->createAuction($seller, 1301, 1999, 193, CharacterAuction::STATUS_SETTLEMENT_PENDING, [
            'saga_state' => CharacterAuction::SAGA_TRANSFER_TO_WINNER,
            'highest_bidder_identity_id' => $winner->id,
            'current_bid' => 500,
            'bid_count' => 1,
            'starts_at' => now()->subDays(2),
            'ends_at' => now()->subMinute(),
            'escrowed_at' => now()->subDays(2),
            'settlement_started_at' => now()->subMinute(),
        ]);

        $result = (new ReconcileCharacterAuctions(
            new TerminalRecoveryFailingGateway,
            app(WalletMutator::class),
        ))->reconcile($auction);

        self::assertSame(CharacterAuction::STATUS_RECOVERY_REQUIRED, $result->status);
        self::assertSame(CharacterAuction::SAGA_RECOVERY_REQUIRED, $result->saga_state);
        self::assertSame('dependency_unavailable', $result->failure_code);
        self::assertSame(2, $result->lock_version);
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
            'provisioning_name' => 'oteryn_terminal_recovery_'.$identity->id,
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
            'player_name' => 'Terminal Recovery Hero '.$playerId,
            'level' => 250,
            'vocation' => 4,
            'sex' => 1,
            'character_snapshot' => [
                'id' => $playerId,
                'name' => 'Terminal Recovery Hero '.$playerId,
                'level' => 250,
                'vocation' => 4,
                'sex' => 1,
            ],
            'status' => $status,
            'saga_state' => CharacterAuction::SAGA_RECOVERY_REQUIRED,
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

final class TerminalRecoveryCharacterTransferGateway implements CanaryCharacterTransferGateway
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
            name: 'Terminal Recovery Hero '.$playerId,
            deletion: 0,
            publicData: [
                'id' => $playerId,
                'name' => 'Terminal Recovery Hero '.$playerId,
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

final class TerminalRecoveryFailingGateway implements CanaryCharacterTransferGateway
{
    public function __construct(private readonly ?Closure $beforeFailure = null) {}

    public function snapshotOwnedCharacter(int $sourceAccountId, int $playerId): CharacterSnapshot
    {
        throw new MarketplaceException('dependency_unavailable', 'The test gateway is unavailable.');
    }

    public function transfer(
        int $playerId,
        int $expectedSourceAccountId,
        int $targetAccountId,
        bool $enforceTargetCharacterLimit,
    ): CharacterTransferResult {
        if ($this->beforeFailure instanceof Closure) {
            ($this->beforeFailure)();
        }

        throw new MarketplaceException('dependency_unavailable', 'The stale worker observed a dependency failure.');
    }

    public function ownershipState(int $playerId): CharacterOwnershipState
    {
        throw new MarketplaceException('dependency_unavailable', 'The test gateway is unavailable.');
    }
}
