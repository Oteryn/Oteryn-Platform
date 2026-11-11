<?php

namespace App\Identity\Mfa;

use App\Identity\Models\Identity;

final readonly class MfaEnrollmentConfirmation
{
    /**
     * @param  list<string>  $recoveryCodes
     */
    public function __construct(
        public Identity $identity,
        public array $recoveryCodes,
    ) {}
}
