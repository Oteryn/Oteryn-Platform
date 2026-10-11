<?php

namespace App\GameProjections;

use RuntimeException;

final class PublicProjectionRefused extends RuntimeException
{
    public function __construct(public readonly int $status)
    {
        parent::__construct('Public projection publication refused.');
    }
}
