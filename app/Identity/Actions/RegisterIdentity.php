<?php

namespace App\Identity\Actions;

use App\Audit\SecurityEventRecorder;
use App\Identity\Models\Identity;
use App\Identity\Support\CanonicalEmail;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;

final class RegisterIdentity
{
    public function __construct(
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    public function execute(string $email, #[\SensitiveParameter] string $password): Identity
    {
        $canonicalEmail = CanonicalEmail::normalize($email);

        return DB::transaction(function () use ($canonicalEmail, $password): Identity {
            $identity = Identity::query()->create([
                'email' => $canonicalEmail,
                'password' => Hash::make($password),
            ]);

            $this->securityEvents->recordIdentityRegistered($identity->id);

            return $identity;
        });
    }
}
