<?php

namespace App\GameAuth\NativeLogin;

/** Facts yielded by native ticket redemption (contract §4.2 step 7); the only source of the grant's account claims. */
final readonly class RedeemedNativeAccount
{
    public function __construct(
        public int $identityId,
        public string $accountId,
        public string $accountSecurityGeneration,
    ) {}
}
