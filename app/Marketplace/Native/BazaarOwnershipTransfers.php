<?php

namespace App\Marketplace\Native;

use App\CharacterAuthority\CharacterCommandBacklogFull;
use App\CharacterAuthority\CharacterCommandClient;
use App\CharacterAuthority\CharacterCommandConflict;
use App\CharacterAuthority\CharacterCommandState;
use App\CharacterAuthority\CharacterCommandStatus;
use App\CharacterAuthority\CharacterCommandUnavailable;
use App\CharacterAuthority\CharacterTransferPurpose;
use App\Marketplace\Exceptions\MarketplaceException;
use Illuminate\Database\UniqueConstraintViolationException;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;
use stdClass;

/**
 * Bazaar saga steps that move a Character between AccountIds through Game Character Authority
 * (`TransferCharacterOwnership`, Game contract §6.2). Platform never writes ownership itself.
 *
 * Each step attempt is one ledger row bound to exactly one command operation, requested with a
 * deterministic idempotency key, so a saga retry or a concurrent reconcile reuses the same
 * operation. Game's stored receipt is the only proof of the outcome: a step is PENDING until it
 * exists, and only a COMMITTED receipt whose binding matches the ledger lets the saga advance.
 *
 * Ordering invariants (a Character never goes to two parties):
 * - listing escrow is the first step of an auction;
 * - settlement or escrow return starts only after a COMMITTED escrow step;
 * - settlement and escrow return exclude each other: one starts only when the other has no
 *   attempt, or its latest attempt is REJECTED.
 */
final class BazaarOwnershipTransfers
{
    public const TABLE = 'character_auction_transfers';

    public const MAX_ATTEMPTS = 8;

    public function __construct(private readonly CharacterCommandClient $commands) {}

    /**
     * Start the step or observe its current attempt. Calling it again for the same step is
     * idempotent and never issues a second operation.
     */
    public function advance(int $auctionId, CharacterTransferPurpose $purpose, string $characterId, string $fromAccountId, string $toAccountId): BazaarTransferStep
    {
        $row = $this->latestRow($auctionId, $purpose);
        if ($row !== null) {
            $this->assertBinding($row, $characterId, $fromAccountId, $toAccountId);

            return $this->observe($row);
        }

        return $this->request($auctionId, $purpose, 1, $characterId, $fromAccountId, $toAccountId);
    }

    /**
     * Request a new attempt after a retryable rejection (recovery only). The binding stays the
     * step's binding; a different destination is a different step.
     */
    public function retry(int $auctionId, CharacterTransferPurpose $purpose, string $characterId, string $fromAccountId, string $toAccountId): BazaarTransferStep
    {
        $row = $this->latestRow($auctionId, $purpose)
            ?? throw new MarketplaceException('auction_state_conflict', 'The transfer step has no attempt to retry.');
        $this->assertBinding($row, $characterId, $fromAccountId, $toAccountId);

        $current = $this->observe($row);
        if (! $current->retryable()) {
            throw new MarketplaceException('transfer_not_retryable', 'The transfer step cannot be retried in its current state.');
        }
        if ($current->attempt >= self::MAX_ATTEMPTS) {
            throw new MarketplaceException('transfer_attempts_exhausted', 'The transfer step has no attempts left.');
        }

        return $this->request($auctionId, $purpose, $current->attempt + 1, $characterId, $fromAccountId, $toAccountId);
    }

    /** The step's latest attempt, or null when the step has not started. */
    public function latest(int $auctionId, CharacterTransferPurpose $purpose): ?BazaarTransferStep
    {
        $row = $this->latestRow($auctionId, $purpose);

        return $row === null ? null : $this->observe($row);
    }

    private function request(int $auctionId, CharacterTransferPurpose $purpose, int $attempt, string $characterId, string $fromAccountId, string $toAccountId): BazaarTransferStep
    {
        try {
            $operationId = DB::transaction(function () use ($auctionId, $purpose, $attempt, $characterId, $fromAccountId, $toAccountId): string {
                $this->assertOrdering($auctionId, $purpose);
                $status = $this->commands->requestTransfer(
                    "bazaar:{$auctionId}:{$purpose->value}:{$attempt}",
                    $characterId,
                    $fromAccountId,
                    $toAccountId,
                    $purpose,
                );
                DB::table(self::TABLE)->insert([
                    'auction_id' => $auctionId,
                    'purpose' => $purpose->value,
                    'attempt' => $attempt,
                    'operation_id' => $status->operationId,
                    'character_id' => $characterId,
                    'from_account_id' => $fromAccountId,
                    'to_account_id' => $toAccountId,
                    'created_at' => now(),
                ]);

                return $status->operationId;
            }, 3);
        } catch (UniqueConstraintViolationException) {
            // A concurrent reconcile recorded this attempt first; the same key bound the same operation.
            $row = $this->latestRow($auctionId, $purpose)
                ?? throw new MarketplaceException('auction_state_conflict', 'The transfer step changed unexpectedly.');
            $this->assertBinding($row, $characterId, $fromAccountId, $toAccountId);

            return $this->observe($row);
        } catch (CharacterCommandUnavailable|CharacterCommandBacklogFull) {
            throw new MarketplaceException('dependency_unavailable', 'Character Authority commands are unavailable; retry later.');
        } catch (CharacterCommandConflict) {
            throw new MarketplaceException('idempotency_conflict', 'The transfer step is bound to a different operation.');
        } catch (InvalidArgumentException) {
            throw new MarketplaceException('transfer_binding_invalid', 'The transfer binding is invalid.');
        }

        $row = DB::table(self::TABLE)->where('operation_id', $operationId)->first();
        if (! $row instanceof stdClass) {
            throw new MarketplaceException('auction_state_conflict', 'The transfer step was not recorded.');
        }

        return $this->observe($row);
    }

    /** Runs inside the requesting transaction; the escrow row lock serializes competing steps. */
    private function assertOrdering(int $auctionId, CharacterTransferPurpose $purpose): void
    {
        if ($purpose === CharacterTransferPurpose::ListingEscrow) {
            $later = DB::table(self::TABLE)
                ->where('auction_id', $auctionId)
                ->where('purpose', '!=', CharacterTransferPurpose::ListingEscrow->value)
                ->exists();
            if ($later) {
                throw new MarketplaceException('auction_state_conflict', 'Listing escrow must be the first transfer of an auction.');
            }

            return;
        }

        $escrow = $this->latestRow($auctionId, CharacterTransferPurpose::ListingEscrow, true);
        if ($escrow === null || $this->observe($escrow)->state !== CharacterCommandState::Committed) {
            throw new MarketplaceException('auction_state_conflict', 'The Character is not in escrow.');
        }

        $competitor = $purpose === CharacterTransferPurpose::Settlement
            ? CharacterTransferPurpose::EscrowReturn
            : CharacterTransferPurpose::Settlement;
        $other = $this->latestRow($auctionId, $competitor);
        if ($other !== null && $this->observe($other)->state !== CharacterCommandState::Rejected) {
            throw new MarketplaceException('auction_state_conflict', 'A competing transfer of this Character is pending or committed.');
        }
    }

    private function latestRow(int $auctionId, CharacterTransferPurpose $purpose, bool $lock = false): ?stdClass
    {
        $query = DB::table(self::TABLE)
            ->where('auction_id', $auctionId)
            ->where('purpose', $purpose->value)
            ->orderByDesc('attempt');
        if ($lock) {
            $query->lockForUpdate();
        }
        $row = $query->first();

        return $row instanceof stdClass ? $row : null;
    }

    private function assertBinding(stdClass $row, string $characterId, string $fromAccountId, string $toAccountId): void
    {
        if ($row->character_id !== $characterId || $row->from_account_id !== $fromAccountId || $row->to_account_id !== $toAccountId) {
            throw new MarketplaceException('transfer_binding_conflict', 'The transfer step is bound to another Character or account.');
        }
    }

    private function observe(stdClass $row): BazaarTransferStep
    {
        $operationId = self::text($row, 'operation_id');
        try {
            $status = $this->commands->status($operationId);
        } catch (CharacterCommandUnavailable) {
            throw new MarketplaceException('dependency_unavailable', 'The transfer receipt cannot be read; retry later.');
        }
        if (! $status instanceof CharacterCommandStatus) {
            throw new MarketplaceException('transfer_operation_missing', 'The transfer operation is missing.');
        }
        if ($status->state === CharacterCommandState::Committed) {
            $this->assertCommittedReceipt($row, $status);
        }

        return new BazaarTransferStep(
            self::integer($row, 'auction_id'),
            CharacterTransferPurpose::from(self::text($row, 'purpose')),
            self::integer($row, 'attempt'),
            $operationId,
            $status->state,
            $status->resultCode,
            $status->expiresAtSource,
        );
    }

    private static function text(stdClass $row, string $column): string
    {
        $value = $row->{$column} ?? null;
        if (! is_string($value)) {
            throw new MarketplaceException('auction_state_conflict', "The transfer ledger column {$column} is malformed.");
        }

        return $value;
    }

    /** Drivers may return integer columns as numeric strings. */
    private static function integer(stdClass $row, string $column): int
    {
        $value = $row->{$column} ?? null;
        if (is_string($value) && preg_match('/^[0-9]{1,18}$/D', $value) === 1) {
            $value = (int) $value;
        }
        if (! is_int($value)) {
            throw new MarketplaceException('auction_state_conflict', "The transfer ledger column {$column} is malformed.");
        }

        return $value;
    }

    /** Money moves only on proof that this exact Character went to this exact account. */
    private function assertCommittedReceipt(stdClass $row, CharacterCommandStatus $status): void
    {
        $receipt = $status->receipt ?? [];
        if (($receipt['character_id'] ?? null) !== $row->character_id
            || ($receipt['from_account_id'] ?? null) !== $row->from_account_id
            || ($receipt['to_account_id'] ?? null) !== $row->to_account_id) {
            throw new MarketplaceException('transfer_receipt_conflict', 'The committed transfer receipt does not match the step.');
        }
    }
}
