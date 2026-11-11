<?php

namespace App\GameAuth\NativeAccountCharacters;

use RuntimeException;

final class NativeAccountCharactersRefused extends RuntimeException
{
    public function __construct(public readonly int $status)
    {
        parent::__construct('Native account-character projection publication refused.');
    }
}
