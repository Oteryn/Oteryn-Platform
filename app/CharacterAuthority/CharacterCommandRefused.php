<?php

namespace App\CharacterAuthority;

use RuntimeException;

/** A Game-initiated exchange answered with an empty-body HTTP status (§4). */
final class CharacterCommandRefused extends RuntimeException
{
    public function __construct(public readonly int $status, string $message = 'Character command exchange refused.')
    {
        parent::__construct($message);
    }
}
