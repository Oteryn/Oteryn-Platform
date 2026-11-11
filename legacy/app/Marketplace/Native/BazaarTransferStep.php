<?php

namespace App\Marketplace\Native;

use App\CharacterAuthority\CharacterCommandState;
use App\CharacterAuthority\CharacterTransferPurpose;

/**
 * What the Bazaar knows about one ownership-transfer attempt: PENDING until Game's receipt is
 * stored, then COMMITTED or REJECTED with Game's result code. A timeout or an expired intent
 * without a receipt is still PENDING (Game contract §2): the saga waits and never settles money.
 */
final readonly class BazaarTransferStep
{
    /**
     * Rejection result code => [saga failure code, a new attempt may be requested]. The class of
     * each code follows the Game contract §7 table; anything else is a contract defect that is
     * never retried automatically.
     *
     * @var array<string, array{0: string, 1: bool}>
     */
    private const REJECTIONS = [
        'CHAR_CMD_CHARACTER_IN_SESSION' => ['character_online_or_session_active', true],
        'CHAR_CMD_INTENT_EXPIRED' => ['transfer_expired', true],
        'CHAR_CMD_HOUSING_NOT_PREPARED' => ['housing_not_prepared', true],
        'CHAR_CMD_ACCOUNT_CHARACTER_LIMIT' => ['target_character_limit', true],
        'CHAR_CMD_OWNER_MISMATCH' => ['ownership_conflict', false],
        'CHAR_CMD_CHARACTER_NOT_FOUND' => ['character_missing', false],
        'CHAR_CMD_CHARACTER_NOT_TRANSFERABLE' => ['character_not_transferable', false],
        'CHAR_CMD_ACCOUNT_NOT_ELIGIBLE' => ['account_not_eligible', false],
    ];

    public const CONTRACT_DEFECT = 'transfer_contract_defect';

    public function __construct(
        public int $auctionId,
        public CharacterTransferPurpose $purpose,
        public int $attempt,
        public string $operationId,
        public CharacterCommandState $state,
        public ?string $resultCode,
        public int $expiresAtSource,
    ) {}

    public function pending(): bool
    {
        return $this->state === CharacterCommandState::Pending;
    }

    public function committed(): bool
    {
        return $this->state === CharacterCommandState::Committed;
    }

    public function rejected(): bool
    {
        return $this->state === CharacterCommandState::Rejected;
    }

    /** The saga failure code of a rejected attempt; null unless rejected. */
    public function failureCode(): ?string
    {
        if (! $this->rejected()) {
            return null;
        }

        return self::REJECTIONS[(string) $this->resultCode][0] ?? self::CONTRACT_DEFECT;
    }

    /** Whether recovery may request a new attempt (a new operation) after this rejection. */
    public function retryable(): bool
    {
        return $this->rejected() && (self::REJECTIONS[(string) $this->resultCode][1] ?? false);
    }

    /** Still waiting for Game after the intent expired; an alarm for operators, never a failure. */
    public function awaitingPastExpiry(int $now): bool
    {
        return $this->pending() && $now >= $this->expiresAtSource;
    }
}
