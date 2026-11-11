<?php

namespace App\Identity\Mfa;

enum MfaFactor
{
    case Totp;
    case RecoveryCode;
}
