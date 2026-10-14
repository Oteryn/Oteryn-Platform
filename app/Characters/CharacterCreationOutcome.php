<?php

namespace App\Characters;

use App\CharacterAuthority\CharacterCommandState;
use App\CharacterAuthority\CharacterCommandStatus;

/**
 * The user-visible result of a web `CreateCharacter` command, read from Game's stored receipt
 * (Game contract §7). Without a receipt the request is still being processed, even past its
 * intent expiry: a timeout proves nothing.
 */
final readonly class CharacterCreationOutcome
{
    private const REJECTIONS = [
        'CHAR_CMD_NAME_INVALID' => 'That character name is not allowed.',
        'CHAR_CMD_NAME_UNAVAILABLE' => 'That character name is not available.',
        'CHAR_CMD_WORLD_NOT_ACCEPTED' => 'That world is not accepting new characters.',
        'CHAR_CMD_ACCOUNT_CHARACTER_LIMIT' => 'This account already has the maximum number of active characters.',
        'CHAR_CMD_ACCOUNT_NOT_ELIGIBLE' => 'Your game account is not ready for character creation.',
        'CHAR_CMD_INTENT_EXPIRED' => 'The request expired before it was processed. Please submit it again.',
    ];

    private const FAILED = 'Character creation could not be completed. Please try again later.';

    private function __construct(
        public string $state,
        public string $message,
    ) {}

    public static function of(CharacterCommandStatus $status): self
    {
        return match ($status->state) {
            CharacterCommandState::Pending => new self('pending', 'Your character request is being processed. Refresh this page to see the result.'),
            CharacterCommandState::Committed => new self('created', self::created($status->receipt['name'] ?? null)),
            CharacterCommandState::Rejected => new self('rejected', self::REJECTIONS[$status->resultCode] ?? self::FAILED),
        };
    }

    private static function created(mixed $name): string
    {
        return is_string($name) ? "Character {$name} created." : 'Character created.';
    }
}
