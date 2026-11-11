<?php

namespace App\Identity\Support;

use Illuminate\Validation\Rules\Password;

final class IdentityPasswordPolicy
{
    public static function rule(): Password
    {
        return Password::min(12)
            ->letters()
            ->mixedCase()
            ->numbers()
            ->symbols();
    }
}
