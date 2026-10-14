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
    /** Receipt result codes with their own message key under `portal.create.outcome`. */
    private const REJECTIONS = [
        'CHAR_CMD_NAME_INVALID' => 'name_invalid',
        'CHAR_CMD_NAME_UNAVAILABLE' => 'name_unavailable',
        'CHAR_CMD_WORLD_NOT_ACCEPTED' => 'world_not_accepted',
        'CHAR_CMD_ACCOUNT_CHARACTER_LIMIT' => 'character_limit',
        'CHAR_CMD_ACCOUNT_NOT_ELIGIBLE' => 'account_not_eligible',
        'CHAR_CMD_INTENT_EXPIRED' => 'intent_expired',
    ];

    private function __construct(
        public string $state,
        public string $message,
    ) {}

    public static function of(CharacterCommandStatus $status): self
    {
        return match ($status->state) {
            CharacterCommandState::Pending => new self('pending', self::message('pending')),
            CharacterCommandState::Committed => new self('created', self::created($status->receipt['name'] ?? null)),
            CharacterCommandState::Rejected => new self('rejected', self::message(self::REJECTIONS[$status->resultCode ?? ''] ?? 'failed')),
        };
    }

    private static function created(mixed $name): string
    {
        return is_string($name) ? self::message('created_named', ['name' => $name]) : self::message('created');
    }

    /** @param array<string, string> $replace */
    private static function message(string $key, array $replace = []): string
    {
        $message = __('portal.create.outcome.'.$key, $replace);

        return is_string($message) ? $message : '';
    }
}
