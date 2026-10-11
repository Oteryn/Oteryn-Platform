<?php

namespace App\Identity\Actions;

use App\Accounts\Actions\ProvisionCanaryAccount;
use App\Accounts\Exceptions\CanaryAccountProvisioningException;
use App\Accounts\Models\IdentityCanaryAccount;
use App\Audit\SecurityEventRecorder;
use App\Identity\Models\Identity;
use App\Identity\Support\CanonicalEmail;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;

final class RegisterIdentity
{
    public function __construct(
        private readonly SecurityEventRecorder $securityEvents,
        private readonly ProvisionCanaryAccount $provisionCanaryAccount,
    ) {}

    public function execute(string $email, #[\SensitiveParameter] string $password): Identity
    {
        $canonicalEmail = CanonicalEmail::normalize($email);

        $identity = DB::transaction(function () use ($canonicalEmail, $password): Identity {
            $identity = Identity::query()->create([
                'email' => $canonicalEmail,
                'password' => Hash::make($password),
            ]);

            IdentityCanaryAccount::query()->create([
                'identity_id' => $identity->id,
                'provisioning_name' => 'op'.bin2hex(random_bytes(15)),
                'canary_creation_epoch' => now()->timestamp,
                'status' => IdentityCanaryAccount::STATUS_PENDING,
            ]);

            $this->securityEvents->recordIdentityRegistered($identity->id);
            $this->securityEvents->recordCanaryAccountProvisioningRequested($identity->id);

            return $identity;
        });

        try {
            $this->provisionCanaryAccount->execute($identity->id);
        } catch (CanaryAccountProvisioningException) {
            // Registration owns Platform Identity creation. A Canary dependency
            // failure leaves the durable provisioning intent pending for retry.
        }

        return $identity;
    }
}
