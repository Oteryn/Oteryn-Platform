<?php

namespace Tests;

use App\Identity\Models\Identity;
use App\Identity\Models\IdentityWebSession;
use App\Identity\Sessions\WebSessionState;
use Illuminate\Contracts\Auth\Authenticatable;
use Illuminate\Foundation\Testing\TestCase as BaseTestCase;
use Illuminate\Support\Str;

abstract class TestCase extends BaseTestCase
{
    /**
     * Authenticate a persisted Platform Identity with the same registered-session
     * invariant required by the production web middleware.
     *
     * @param  string|null  $guard
     * @return $this
     */
    public function be(Authenticatable $user, $guard = null)
    {
        parent::be($user, $guard);

        if (! $user instanceof Identity || ! $user->exists) {
            return $this;
        }

        $identity = Identity::query()->find($user->getAuthIdentifier());
        if (! $identity instanceof Identity) {
            return $this;
        }

        $now = now();
        $registeredSession = IdentityWebSession::query()->create([
            'id' => (string) Str::uuid(),
            'identity_id' => $identity->id,
            'generation' => $identity->web_session_generation,
            'user_agent' => 'Laravel feature test',
            'ip_hash' => null,
            'issued_at' => $now,
            'last_seen_at' => $now,
            'expires_at' => $now->copy()->addHours(2),
        ]);

        $this->withSession([
            WebSessionState::GENERATION_KEY => $identity->web_session_generation,
            WebSessionState::REGISTRY_ID_KEY => $registeredSession->id,
        ]);

        return $this;
    }
}
