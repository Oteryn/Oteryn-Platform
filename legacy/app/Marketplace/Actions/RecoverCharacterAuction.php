<?php

namespace App\Marketplace\Actions;

use App\Accounts\Models\IdentityCanaryAccount;
use App\Marketplace\Contracts\CanaryCharacterTransferGateway;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Marketplace\Models\CharacterAuction;
use Illuminate\Support\Facades\DB;

final class RecoverCharacterAuction
{
    public function __construct(
        private readonly CanaryCharacterTransferGateway $characters,
        private readonly ReconcileCharacterAuctions $reconcile,
    ) {}

    public function execute(CharacterAuction $auction): CharacterAuction
    {
        if ($auction->status !== CharacterAuction::STATUS_RECOVERY_REQUIRED) {
            return $this->reconcile->reconcile($auction);
        }

        $state = $this->characters->ownershipState($auction->player_id);
        if ($state->deletion !== 0 || $state->hasClusterSession) {
            throw new MarketplaceException('character_not_quiescent', 'Recovery requires an active character with no cluster session.');
        }

        $next = DB::transaction(function () use ($auction, $state): CharacterAuction {
            $locked = CharacterAuction::query()->whereKey($auction->id)->lockForUpdate()->firstOrFail();
            if ($locked->status !== CharacterAuction::STATUS_RECOVERY_REQUIRED) {
                return $locked;
            }

            if ($state->accountId === $locked->escrow_canary_account_id) {
                if ($locked->highest_bidder_identity_id !== null) {
                    $locked->status = CharacterAuction::STATUS_SETTLEMENT_PENDING;
                    $locked->saga_state = CharacterAuction::SAGA_TRANSFER_TO_WINNER;
                    $locked->settlement_started_at ??= now();
                } elseif ($locked->ends_at !== null && ! $locked->ends_at->isFuture()) {
                    $locked->status = CharacterAuction::STATUS_CANCEL_PENDING;
                    $locked->saga_state = CharacterAuction::SAGA_RETURN_TO_SELLER;
                    $locked->failure_code = 'expired_no_bid';
                } elseif ($locked->ends_at !== null) {
                    $locked->status = CharacterAuction::STATUS_ACTIVE;
                    $locked->saga_state = CharacterAuction::SAGA_ACTIVE;
                    $locked->failure_code = null;
                } else {
                    $locked->status = CharacterAuction::STATUS_ESCROW_PENDING;
                    $locked->saga_state = CharacterAuction::SAGA_QUIESCENCE_WAIT;
                    $locked->escrowed_at = now();
                    $locked->failure_code = null;
                }
            } elseif ($state->accountId === $locked->seller_canary_account_id && $locked->bid_count === 0) {
                $locked->status = CharacterAuction::STATUS_CANCELLED;
                $locked->saga_state = CharacterAuction::SAGA_DONE;
                $locked->failure_code = null;
                $locked->active_player_id = null;
                $locked->cancelled_at = now();
            } elseif ($locked->highest_bidder_identity_id !== null) {
                $winnerBinding = IdentityCanaryAccount::query()->whereKey($locked->highest_bidder_identity_id)->first();
                if ($winnerBinding === null || ! $winnerBinding->isReady() || ! is_int($winnerBinding->canary_account_id)
                    || $winnerBinding->canary_account_id !== $state->accountId) {
                    throw new MarketplaceException('ownership_conflict', 'The actual character owner does not match a recoverable marketplace participant.');
                }

                $locked->status = CharacterAuction::STATUS_SETTLEMENT_PENDING;
                $locked->saga_state = CharacterAuction::SAGA_WALLET_SETTLEMENT;
                $locked->settlement_started_at ??= now();
            } else {
                throw new MarketplaceException('ownership_conflict', 'The actual character owner does not match a recoverable marketplace state.');
            }

            $locked->lock_version++;
            $locked->save();

            return $locked;
        }, 3);

        return $this->reconcile->reconcile($next);
    }
}
