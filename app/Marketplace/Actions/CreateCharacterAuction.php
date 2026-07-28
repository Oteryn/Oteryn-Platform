<?php

namespace App\Marketplace\Actions;

use App\Accounts\Models\IdentityCanaryAccount;
use App\Identity\Models\Identity;
use App\Marketplace\Contracts\CanaryCharacterTransferGateway;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Marketplace\Models\CharacterAuction;
use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;
use Throwable;

final class CreateCharacterAuction
{
    public function __construct(private readonly CanaryCharacterTransferGateway $characters) {}

    public function execute(
        Identity $seller,
        int $playerId,
        int $durationDays,
        int $startingBid,
        ?int $buyNowPrice,
        string $requestId,
    ): CharacterAuction {
        $existing = CharacterAuction::query()->where('listing_request_id', $requestId)->first();
        if ($existing instanceof CharacterAuction) {
            if ($existing->seller_identity_id !== $seller->id) {
                throw new MarketplaceException('idempotency_conflict', 'The listing request identifier is already in use.');
            }

            return $existing;
        }

        $binding = IdentityCanaryAccount::query()->whereKey($seller->id)->first();
        if ($binding === null || ! $binding->isReady() || ! is_int($binding->canary_account_id)) {
            throw new MarketplaceException('binding_not_ready', 'Your game account must be ready before listing a character.');
        }

        $configuredEscrowAccountId = config('marketplace.escrow_canary_account_id', 0);
        if (! is_int($configuredEscrowAccountId)
            || $configuredEscrowAccountId <= 0
            || $configuredEscrowAccountId === $binding->canary_account_id) {
            throw new MarketplaceException('invalid_escrow_configuration', 'Character Bazaar escrow is not configured.');
        }

        $allowedDurations = config('marketplace.allowed_duration_days', [1, 3, 7]);
        if (! is_array($allowedDurations)
            || array_filter($allowedDurations, static fn (mixed $duration): bool => ! is_int($duration)) !== []
            || ! in_array($durationDays, $allowedDurations, true)) {
            throw new MarketplaceException('invalid_duration', 'Select an available auction duration.');
        }

        $configuredMinimumStartingBid = config('marketplace.minimum_starting_bid', 100);
        if (! is_int($configuredMinimumStartingBid) || $configuredMinimumStartingBid < 1) {
            throw new MarketplaceException('invalid_marketplace_configuration', 'The marketplace minimum starting bid configuration is invalid.');
        }
        if ($startingBid < $configuredMinimumStartingBid) {
            throw new MarketplaceException('starting_bid_too_low', "The starting bid must be at least {$configuredMinimumStartingBid} Oteryn Coins.");
        }

        if ($buyNowPrice !== null && $buyNowPrice < $startingBid) {
            throw new MarketplaceException('buy_now_too_low', 'The buy-now price cannot be lower than the starting bid.');
        }

        $snapshot = $this->characters->snapshotOwnedCharacter($binding->canary_account_id, $playerId);
        $public = $snapshot->publicData;
        $level = $this->snapshotInt($public, 'level');
        $vocation = $this->snapshotInt($public, 'vocation');
        $sex = $this->snapshotInt($public, 'sex');

        try {
            $auction = DB::transaction(function () use (
                $seller,
                $binding,
                $configuredEscrowAccountId,
                $snapshot,
                $durationDays,
                $startingBid,
                $buyNowPrice,
                $requestId,
                $level,
                $vocation,
                $sex,
            ): CharacterAuction {
                return CharacterAuction::query()->create([
                    'listing_request_id' => $requestId,
                    'seller_identity_id' => $seller->id,
                    'seller_canary_account_id' => $binding->canary_account_id,
                    'escrow_canary_account_id' => $configuredEscrowAccountId,
                    'player_id' => $snapshot->playerId,
                    'active_player_id' => $snapshot->playerId,
                    'player_name' => $snapshot->name,
                    'level' => $level,
                    'vocation' => $vocation,
                    'sex' => $sex,
                    'character_snapshot' => $snapshot->publicData,
                    'status' => CharacterAuction::STATUS_ESCROW_PENDING,
                    'saga_state' => CharacterAuction::SAGA_ESCROW_REQUESTED,
                    'failure_code' => null,
                    'duration_days' => $durationDays,
                    'starting_bid' => $startingBid,
                    'buy_now_price' => $buyNowPrice,
                    'current_bid' => 0,
                    'highest_bidder_identity_id' => null,
                    'bid_count' => 0,
                    'lock_version' => 1,
                ]);
            }, 3);
        } catch (QueryException $exception) {
            if (! $this->isDuplicateKey($exception)) {
                throw new MarketplaceException('dependency_unavailable', 'The marketplace database is temporarily unavailable.');
            }

            $recovered = CharacterAuction::query()->where('listing_request_id', $requestId)->first();
            if ($recovered instanceof CharacterAuction && $recovered->seller_identity_id === $seller->id) {
                return $recovered;
            }

            throw new MarketplaceException('character_already_listed', 'This character is already controlled by another auction.');
        }

        try {
            $this->characters->transfer(
                $auction->player_id,
                $auction->seller_canary_account_id,
                $auction->escrow_canary_account_id,
                false,
            );

            CharacterAuction::query()->whereKey($auction->id)->update([
                'saga_state' => CharacterAuction::SAGA_QUIESCENCE_WAIT,
                'escrowed_at' => now(),
                'failure_code' => null,
                'lock_version' => $auction->lock_version + 1,
                'updated_at' => now(),
            ]);
        } catch (MarketplaceException $exception) {
            $this->markRecovery($auction, $exception->reason);

            throw $exception;
        } catch (Throwable) {
            $this->markRecovery($auction, 'platform_persistence_unavailable');

            throw new MarketplaceException('dependency_unavailable', 'The listing entered recovery because its escrow state could not be persisted.');
        }

        return $auction->refresh();
    }

    private function markRecovery(CharacterAuction $auction, string $reason): void
    {
        try {
            CharacterAuction::query()->whereKey($auction->id)->update([
                'status' => CharacterAuction::STATUS_RECOVERY_REQUIRED,
                'saga_state' => CharacterAuction::SAGA_RECOVERY_REQUIRED,
                'failure_code' => $reason,
                'lock_version' => $auction->lock_version + 1,
                'updated_at' => now(),
            ]);
        } catch (Throwable) {
            report(new MarketplaceException('recovery_state_persistence_failed', 'The listing recovery state could not be persisted.'));
        }
    }

    /** @param  array<string, int|string|null>  $snapshot */
    private function snapshotInt(array $snapshot, string $key): int
    {
        $value = $snapshot[$key] ?? null;
        if (! is_int($value) && ! is_string($value)) {
            throw new MarketplaceException('snapshot_invalid', 'The character snapshot is incomplete.');
        }

        return (int) $value;
    }

    private function isDuplicateKey(QueryException $exception): bool
    {
        $driverCode = $exception->errorInfo[1] ?? null;

        return (string) $exception->getCode() === '23000'
            && (is_int($driverCode) || is_string($driverCode))
            && (int) $driverCode === 1062;
    }
}
