<?php

namespace App\Marketplace\Actions;

use App\Accounts\Models\IdentityCanaryAccount;
use App\Marketplace\Contracts\CanaryCharacterTransferGateway;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Marketplace\Models\AuctionBid;
use App\Marketplace\Models\CharacterAuction;
use App\Wallet\WalletMutator;
use Illuminate\Support\Facades\DB;
use Throwable;

final class ReconcileCharacterAuctions
{
    public function __construct(
        private readonly CanaryCharacterTransferGateway $characters,
        private readonly WalletMutator $wallets,
    ) {}

    /** @return array{processed: int, completed: int, recovery_required: int} */
    public function run(int $limit = 100): array
    {
        $auctions = CharacterAuction::query()
            ->whereIn('status', [
                CharacterAuction::STATUS_ESCROW_PENDING,
                CharacterAuction::STATUS_ACTIVE,
                CharacterAuction::STATUS_SETTLEMENT_PENDING,
                CharacterAuction::STATUS_CANCEL_PENDING,
            ])
            ->orderBy('id')
            ->limit(max(1, min($limit, 1000)))
            ->get();

        $completed = 0;
        $recovery = 0;

        foreach ($auctions as $auction) {
            $result = $this->reconcile($auction);
            if ($result->isTerminal()) {
                $completed++;
            }
            if ($result->status === CharacterAuction::STATUS_RECOVERY_REQUIRED) {
                $recovery++;
            }
        }

        return ['processed' => $auctions->count(), 'completed' => $completed, 'recovery_required' => $recovery];
    }

    public function reconcile(CharacterAuction $auction): CharacterAuction
    {
        try {
            $fresh = CharacterAuction::query()->findOrFail($auction->id);

            if ($fresh->status === CharacterAuction::STATUS_ESCROW_PENDING) {
                return $this->reconcileEscrow($fresh);
            }

            if ($fresh->status === CharacterAuction::STATUS_ACTIVE) {
                if ($fresh->ends_at !== null && ! $fresh->ends_at->isFuture()) {
                    $fresh = $this->beginClose($fresh);
                } else {
                    return $fresh;
                }
            }

            if ($fresh->status === CharacterAuction::STATUS_SETTLEMENT_PENDING) {
                return $this->settle($fresh);
            }

            if ($fresh->status === CharacterAuction::STATUS_CANCEL_PENDING) {
                return $this->returnToSeller($fresh);
            }

            return $fresh;
        } catch (MarketplaceException $exception) {
            return $this->markRecovery($auction->id, $exception->reason);
        } catch (Throwable) {
            return $this->markRecovery($auction->id, 'dependency_unavailable');
        }
    }

    private function reconcileEscrow(CharacterAuction $auction): CharacterAuction
    {
        if ($auction->escrowed_at === null) {
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

            return CharacterAuction::query()->findOrFail($auction->id);
        }

        $configuredQuiescence = config('marketplace.escrow_quiescence_seconds', 30);
        if (! is_int($configuredQuiescence) || $configuredQuiescence < 1 || $configuredQuiescence > 3600) {
            throw new MarketplaceException('invalid_marketplace_configuration', 'The escrow quiescence configuration is invalid.');
        }

        if ($auction->escrowed_at->addSeconds($configuredQuiescence)->isFuture()) {
            return $auction;
        }

        $state = $this->characters->ownershipState($auction->player_id);
        if ($state->accountId !== $auction->escrow_canary_account_id) {
            throw new MarketplaceException('ownership_conflict', 'Escrow ownership could not be confirmed.');
        }
        if ($state->deletion !== 0) {
            throw new MarketplaceException('character_deleted', 'The escrowed character is not active.');
        }
        if ($state->hasClusterSession) {
            throw new MarketplaceException('character_online_or_session_active', 'The escrowed character still has a game session.');
        }

        return DB::transaction(function () use ($auction): CharacterAuction {
            $locked = CharacterAuction::query()->whereKey($auction->id)->lockForUpdate()->firstOrFail();
            if ($locked->status !== CharacterAuction::STATUS_ESCROW_PENDING) {
                return $locked;
            }

            $locked->status = CharacterAuction::STATUS_ACTIVE;
            $locked->saga_state = CharacterAuction::SAGA_ACTIVE;
            $locked->failure_code = null;
            $locked->starts_at = now();
            $locked->ends_at = now()->addDays($locked->duration_days);
            $locked->lock_version++;
            $locked->save();

            return $locked;
        }, 3);
    }

    private function beginClose(CharacterAuction $auction): CharacterAuction
    {
        return DB::transaction(function () use ($auction): CharacterAuction {
            $locked = CharacterAuction::query()->whereKey($auction->id)->lockForUpdate()->firstOrFail();
            if ($locked->status !== CharacterAuction::STATUS_ACTIVE || ($locked->ends_at !== null && $locked->ends_at->isFuture())) {
                return $locked;
            }

            if ($locked->highest_bidder_identity_id === null) {
                $locked->status = CharacterAuction::STATUS_CANCEL_PENDING;
                $locked->saga_state = CharacterAuction::SAGA_RETURN_TO_SELLER;
                $locked->failure_code = 'expired_no_bid';
            } else {
                $locked->status = CharacterAuction::STATUS_SETTLEMENT_PENDING;
                $locked->saga_state = CharacterAuction::SAGA_TRANSFER_TO_WINNER;
                $locked->settlement_started_at = now();
            }

            $locked->lock_version++;
            $locked->save();

            return $locked;
        }, 3);
    }

    private function settle(CharacterAuction $auction): CharacterAuction
    {
        if ($auction->highest_bidder_identity_id === null) {
            throw new MarketplaceException('auction_state_conflict', 'The settlement has no winning bidder.');
        }

        $winnerBinding = IdentityCanaryAccount::query()->whereKey($auction->highest_bidder_identity_id)->first();
        if ($winnerBinding === null || ! $winnerBinding->isReady() || ! is_int($winnerBinding->canary_account_id)) {
            throw new MarketplaceException('winner_binding_not_ready', 'The winning game account is not ready for settlement.');
        }

        $this->characters->transfer(
            $auction->player_id,
            $auction->escrow_canary_account_id,
            $winnerBinding->canary_account_id,
            true,
        );

        return DB::transaction(function () use ($auction): CharacterAuction {
            $locked = CharacterAuction::query()->whereKey($auction->id)->lockForUpdate()->firstOrFail();
            if ($locked->status === CharacterAuction::STATUS_COMPLETED) {
                return $locked;
            }
            if ($locked->status !== CharacterAuction::STATUS_SETTLEMENT_PENDING || $locked->highest_bidder_identity_id === null) {
                throw new MarketplaceException('auction_state_conflict', 'The settlement state changed unexpectedly.');
            }

            $winningBid = AuctionBid::query()
                ->where('auction_id', $locked->id)
                ->where('status', AuctionBid::STATUS_LEADING)
                ->lockForUpdate()
                ->first();
            if (! $winningBid instanceof AuctionBid || $winningBid->bidder_identity_id !== $locked->highest_bidder_identity_id) {
                throw new MarketplaceException('auction_state_conflict', 'The winning bid state requires recovery.');
            }

            $identityIds = [$locked->seller_identity_id, $winningBid->bidder_identity_id];
            sort($identityIds, SORT_NUMERIC);
            $wallets = [];
            foreach (array_values(array_unique($identityIds)) as $identityId) {
                $wallets[$identityId] = $this->wallets->lock($identityId);
            }

            $configuredCommissionBps = config('marketplace.commission_basis_points', 1000);
            if (! is_int($configuredCommissionBps) || $configuredCommissionBps < 0 || $configuredCommissionBps > 10_000) {
                throw new MarketplaceException('invalid_marketplace_configuration', 'The marketplace commission configuration is invalid.');
            }

            $commission = intdiv($winningBid->amount * $configuredCommissionBps, 10_000);
            $sellerProceeds = $winningBid->amount - $commission;

            $this->wallets->applyLocked(
                $wallets[$winningBid->bidder_identity_id],
                'auction_purchase_settled',
                0,
                -$winningBid->amount,
                "auction:{$locked->id}:winner:settlement",
                $locked->id,
                ['bid_id' => $winningBid->id, 'amount' => $winningBid->amount],
            );
            $this->wallets->applyLocked(
                $wallets[$locked->seller_identity_id],
                'auction_sale_proceeds',
                $sellerProceeds,
                0,
                "auction:{$locked->id}:seller:proceeds",
                $locked->id,
                ['gross' => $winningBid->amount, 'commission' => $commission],
            );

            $winningBid->status = AuctionBid::STATUS_WON;
            $winningBid->updated_at = now();
            $winningBid->save();

            $locked->status = CharacterAuction::STATUS_COMPLETED;
            $locked->saga_state = CharacterAuction::SAGA_DONE;
            $locked->failure_code = null;
            $locked->active_player_id = null;
            $locked->settled_at = now();
            $locked->lock_version++;
            $locked->save();

            return $locked;
        }, 3);
    }

    private function returnToSeller(CharacterAuction $auction): CharacterAuction
    {
        $sellerBinding = IdentityCanaryAccount::query()->whereKey($auction->seller_identity_id)->first();
        if ($sellerBinding === null || ! $sellerBinding->isReady() || ! is_int($sellerBinding->canary_account_id)) {
            throw new MarketplaceException('seller_binding_not_ready', 'The seller game account is not ready for character return.');
        }
        if ($sellerBinding->canary_account_id !== $auction->seller_canary_account_id) {
            throw new MarketplaceException('seller_binding_conflict', 'The seller game account binding changed unexpectedly.');
        }

        $this->characters->transfer(
            $auction->player_id,
            $auction->escrow_canary_account_id,
            $auction->seller_canary_account_id,
            true,
        );

        return DB::transaction(function () use ($auction): CharacterAuction {
            $locked = CharacterAuction::query()->whereKey($auction->id)->lockForUpdate()->firstOrFail();
            if (in_array($locked->status, [CharacterAuction::STATUS_CANCELLED, CharacterAuction::STATUS_EXPIRED], true)) {
                return $locked;
            }
            if ($locked->status !== CharacterAuction::STATUS_CANCEL_PENDING) {
                throw new MarketplaceException('auction_state_conflict', 'The cancellation state changed unexpectedly.');
            }

            $locked->status = $locked->failure_code === 'expired_no_bid'
                ? CharacterAuction::STATUS_EXPIRED
                : CharacterAuction::STATUS_CANCELLED;
            $locked->saga_state = CharacterAuction::SAGA_DONE;
            $locked->active_player_id = null;
            $locked->cancelled_at = now();
            $locked->lock_version++;
            $locked->save();

            return $locked;
        }, 3);
    }

    private function markRecovery(int $auctionId, string $failureCode): CharacterAuction
    {
        return DB::transaction(function () use ($auctionId, $failureCode): CharacterAuction {
            $locked = CharacterAuction::query()->whereKey($auctionId)->lockForUpdate()->firstOrFail();

            if ($locked->isTerminal()) {
                return $locked;
            }

            if (! in_array($locked->status, [
                CharacterAuction::STATUS_ESCROW_PENDING,
                CharacterAuction::STATUS_ACTIVE,
                CharacterAuction::STATUS_SETTLEMENT_PENDING,
                CharacterAuction::STATUS_CANCEL_PENDING,
                CharacterAuction::STATUS_RECOVERY_REQUIRED,
            ], true)) {
                return $locked;
            }

            $locked->status = CharacterAuction::STATUS_RECOVERY_REQUIRED;
            $locked->saga_state = CharacterAuction::SAGA_RECOVERY_REQUIRED;
            $locked->failure_code = $failureCode;
            $locked->lock_version++;
            $locked->save();

            return $locked;
        }, 3);
    }
}
