<?php

namespace App\CharacterAuthority;

/** What Platform knows about one command: PENDING until Game's receipt is stored, then its outcome. */
final readonly class CharacterCommandStatus
{
    /** @param array<string, int|string>|null $receipt the stored receipt members, once terminal */
    public function __construct(
        public string $operationId,
        public string $command,
        public CharacterCommandState $state,
        public ?string $resultCode,
        public int $expiresAtSource,
        public ?array $receipt,
    ) {}

    public static function of(CharacterCommandIntentRow $intent, ?CharacterCommandStoredReceipt $receipt): self
    {
        if ($receipt === null) {
            return new self($intent->operationId, $intent->command, CharacterCommandState::Pending, null, $intent->expiresAtSource, null);
        }

        return new self(
            $intent->operationId,
            $intent->command,
            $receipt->outcome === CharacterCommandWire::COMMITTED ? CharacterCommandState::Committed : CharacterCommandState::Rejected,
            $receipt->resultCode,
            $intent->expiresAtSource,
            $receipt->receipt(),
        );
    }

    /**
     * No receipt although the intent has expired. The command is still PENDING, never failed: a
     * timeout proves nothing and Game always publishes a terminal receipt (Game contract §2, §5.3).
     */
    public function awaitingPastExpiry(int $now): bool
    {
        return $this->state === CharacterCommandState::Pending && $now >= $this->expiresAtSource;
    }
}
