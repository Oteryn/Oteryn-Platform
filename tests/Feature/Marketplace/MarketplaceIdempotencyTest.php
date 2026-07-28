<?php

namespace Tests\Feature\Marketplace;

use App\Accounts\Models\IdentityCanaryAccount;
use App\Identity\Models\Identity;
use App\Marketplace\Actions\CreateCharacterAuction;
use App\Marketplace\Actions\PlaceAuctionBid;
use App\Marketplace\Contracts\CanaryCharacterTransferGateway;
use App\Marketplace\Data\CharacterOwnershipState;
use App\Marketplace\Data\CharacterSnapshot;
use App\Marketplace\Data\CharacterTransferResult;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Marketplace\Models\AuctionBid;
use App\Marketplace\Models\CharacterAuction;
use App\Wallet\Actions\AdjustWalletBalance;
use App\Wallet\Models\WalletAccount;
use App\Wallet\Models\WalletLedgerEntry;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use Tests\TestCase;

final class MarketplaceIdempotencyTest extends TestCase
{
    use RefreshDatabase;

    public function test_listing_request_replay_requires_the_exact_original_payload(): void
    {
        config()->set('marketplace.escrow_canary_account_id', 999);

        $seller = $this->createIdentity('listing-idempotency@example.com');
        $this->readyBinding($seller, 101);
        $gateway = new IdempotencyCharacterTransferGateway([
            42 => ['account_id' => 101, 'name' => 'Exact Listing'],
        ]);
        $this->app->instance(CanaryCharacterTransferGateway::class, $gateway);
        $action = app(CreateCharacterAuction::class);
        $requestId = (string) Str::uuid();

        $created = $action->execute($seller, 42, 3, 100, 500, $requestId);
        $replayed = $action->execute($seller, 42, 3, 100, 500, $requestId);

        self::assertSame($created->id, $replayed->id);
        self::assertSame(1, CharacterAuction::query()->count());

        try {
            $action->execute($seller, 42, 3, 200, 500, $requestId);
            self::fail('A listing request identifier must not accept a different payload.');
        } catch (MarketplaceException $exception) {
            self::assertSame('idempotency_conflict', $exception->reason);
        }
    }

    public function test_bid_request_replay_requires_the_exact_amount_and_operation_type(): void
    {
        $seller = $this->createIdentity('bid-idempotency-seller@example.com');
        $bidder = $this->createIdentity('bid-idempotency-bidder@example.com');
        $this->readyBinding($bidder, 201);
        $auction = $this->createActiveAuction($seller);
        WalletAccount::query()->create([
            'identity_id' => $bidder->id,
            'available_balance' => 2_000,
            'reserved_balance' => 0,
        ]);
        $action = app(PlaceAuctionBid::class);
        $requestId = (string) Str::uuid();

        $action->execute($bidder, $auction, 200, $requestId, false);
        $action->execute($bidder, $auction->refresh(), 200, $requestId, false);

        $bid = AuctionBid::query()->where('request_id', $requestId)->firstOrFail();
        self::assertFalse($bid->is_buy_now);
        self::assertSame(1, AuctionBid::query()->count());
        self::assertSame(200, WalletAccount::query()->findOrFail($bidder->id)->reserved_balance);

        foreach ([[250, false], [200, true]] as [$amount, $buyNow]) {
            try {
                $action->execute($bidder, $auction->refresh(), $amount, $requestId, $buyNow);
                self::fail('A bid request identifier must not accept a different payload or operation type.');
            } catch (MarketplaceException $exception) {
                self::assertSame('idempotency_conflict', $exception->reason);
            }
        }
    }

    public function test_wallet_adjustment_replay_is_exact_and_audited_once(): void
    {
        $actor = $this->createIdentity('wallet-idempotency-actor@example.com');
        $target = $this->createIdentity('wallet-idempotency-target@example.com');
        $otherTarget = $this->createIdentity('wallet-idempotency-other@example.com');
        $action = app(AdjustWalletBalance::class);
        $requestId = (string) Str::uuid();
        $reason = 'Controlled idempotency verification adjustment.';

        $action->execute($actor, $target, 500, $reason, $requestId);
        $action->execute($actor, $target, 500, $reason, $requestId);

        self::assertSame(500, WalletAccount::query()->findOrFail($target->id)->available_balance);
        self::assertSame(1, WalletLedgerEntry::query()->where('idempotency_key', 'admin-wallet-adjustment:'.$requestId)->count());
        self::assertSame(1, DB::table('admin_audit_events')
            ->where('action', 'marketplace.wallet_adjusted')
            ->where('target_id', (string) $target->id)
            ->count());

        try {
            $action->execute($actor, $target, 600, $reason, $requestId);
            self::fail('A wallet adjustment identifier must not accept a different amount.');
        } catch (MarketplaceException $exception) {
            self::assertSame('idempotency_conflict', $exception->reason);
        }

        try {
            $action->execute($actor, $otherTarget, 500, $reason, $requestId);
            self::fail('A wallet adjustment identifier must not be reused for another target.');
        } catch (MarketplaceException $exception) {
            self::assertSame('idempotency_conflict', $exception->reason);
        }

        self::assertNull(WalletAccount::query()->find($otherTarget->id));
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
            'provisioning_name' => 'oteryn_idempotency_'.$identity->id,
            'canary_creation_epoch' => 1,
            'status' => IdentityCanaryAccount::STATUS_READY,
            'ready_at' => now(),
        ]);
    }

    private function createActiveAuction(Identity $seller): CharacterAuction
    {
        $playerId = 2_000_000 + $seller->id;

        return CharacterAuction::query()->create([
            'listing_request_id' => (string) Str::uuid(),
            'seller_identity_id' => $seller->id,
            'seller_canary_account_id' => 1001,
            'escrow_canary_account_id' => 9999,
            'player_id' => $playerId,
            'active_player_id' => $playerId,
            'player_name' => 'Idempotency Hero',
            'level' => 250,
            'vocation' => 4,
            'sex' => 1,
            'character_snapshot' => ['id' => $playerId, 'name' => 'Idempotency Hero'],
            'status' => CharacterAuction::STATUS_ACTIVE,
            'saga_state' => CharacterAuction::SAGA_ACTIVE,
            'duration_days' => 3,
            'starting_bid' => 100,
            'buy_now_price' => 1_000,
            'current_bid' => 0,
            'bid_count' => 0,
            'lock_version' => 1,
            'starts_at' => now()->subHour(),
            'ends_at' => now()->addDay(),
            'escrowed_at' => now()->subHours(2),
        ]);
    }
}

final class IdempotencyCharacterTransferGateway implements CanaryCharacterTransferGateway
{
    /** @var array<int, int> */
    private array $owners = [];

    /** @var array<int, string> */
    private array $names = [];

    /** @param array<int, array{account_id: int, name: string}> $characters */
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
            playerId: $playerId,
            accountId: $sourceAccountId,
            name: $name,
            deletion: 0,
            publicData: [
                'id' => $playerId,
                'name' => $name,
                'level' => 150,
                'vocation' => 3,
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
