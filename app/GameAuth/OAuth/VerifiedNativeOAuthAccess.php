<?php

namespace App\GameAuth\OAuth;

use App\Identity\Models\Identity;
use Laravel\Passport\Token;

final readonly class VerifiedNativeOAuthAccess
{
    public function __construct(
        public Identity $identity,
        public Token $token,
    ) {}
}
