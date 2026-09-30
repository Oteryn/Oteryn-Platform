<?php

namespace App\GameAuth\NativeLogin;

use RuntimeException;

/** A native login refusal with its contract error code; the message never carries secrets. */
final class NativeLoginRefused extends RuntimeException
{
    /** @param  int|null  $retryAfterSeconds  the `Retry-After` of a NATIVE_LOGIN_RATE_LIMITED refusal (§10) */
    public function __construct(public readonly NativeLoginError $error, public readonly ?int $retryAfterSeconds = null)
    {
        parent::__construct($error->value);
    }
}
