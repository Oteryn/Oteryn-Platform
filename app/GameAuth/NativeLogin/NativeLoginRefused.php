<?php

namespace App\GameAuth\NativeLogin;

use RuntimeException;

/** A native login refusal with its contract error code; the message never carries secrets. */
final class NativeLoginRefused extends RuntimeException
{
    public function __construct(public readonly NativeLoginError $error)
    {
        parent::__construct($error->value);
    }
}
