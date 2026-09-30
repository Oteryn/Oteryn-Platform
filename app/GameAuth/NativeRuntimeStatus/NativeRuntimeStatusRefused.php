<?php

namespace App\GameAuth\NativeRuntimeStatus;

use RuntimeException;

/** A refusal with the exact HTTP status the Game producer classifies (§4); the body stays empty. */
final class NativeRuntimeStatusRefused extends RuntimeException
{
    public function __construct(public readonly int $status)
    {
        parent::__construct('Runtime status report refused.');
    }
}
