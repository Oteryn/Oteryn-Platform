<?php

namespace App\Identity\Sessions;

use App\Identity\Models\Identity;
use App\Identity\Models\IdentityWebSession;
use Illuminate\Database\Eloquent\Collection;
use Illuminate\Http\Request;
use Illuminate\Support\Str;
use LogicException;

final class IdentityWebSessionRegistry
{
    public function establish(Request $request, Identity $identity): IdentityWebSession
    {
        $now = now();
        $lifetimeMinutes = config('session.lifetime');
        if (! is_int($lifetimeMinutes) || $lifetimeMinutes < 1) {
            throw new LogicException('The session lifetime configuration is invalid.');
        }

        return IdentityWebSession::query()->create([
            'id' => (string) Str::uuid(),
            'identity_id' => $identity->id,
            'generation' => $identity->web_session_generation,
            'user_agent' => $this->boundedUserAgent($request),
            'ip_hash' => $this->sourceIpHash($request),
            'issued_at' => $now,
            'last_seen_at' => $now,
            'expires_at' => $now->copy()->addMinutes($lifetimeMinutes),
        ]);
    }

    public function current(Request $request, Identity $identity): ?IdentityWebSession
    {
        $registryId = $request->session()->get(WebSessionState::REGISTRY_ID_KEY);
        if (! is_string($registryId) || $registryId === '') {
            return null;
        }

        return IdentityWebSession::query()
            ->whereKey($registryId)
            ->where('identity_id', $identity->id)
            ->first();
    }

    public function touch(IdentityWebSession $session): void
    {
        $intervalSeconds = config('identity_security.sessions.touch_interval_seconds');
        if (! is_int($intervalSeconds) || $intervalSeconds < 1 || $intervalSeconds > 3600) {
            throw new LogicException('The identity session touch interval is invalid.');
        }

        if ($session->last_seen_at->gt(now()->subSeconds($intervalSeconds))) {
            return;
        }

        $session->forceFill(['last_seen_at' => now()])->save();
    }

    public function revokeCurrent(Request $request): void
    {
        $registryId = $request->session()->get(WebSessionState::REGISTRY_ID_KEY);
        if (! is_string($registryId) || $registryId === '') {
            return;
        }

        IdentityWebSession::query()
            ->whereKey($registryId)
            ->whereNull('revoked_at')
            ->update(['revoked_at' => now(), 'updated_at' => now()]);
    }

    public function revokeOwned(Identity $identity, string $registryId): bool
    {
        return IdentityWebSession::query()
            ->whereKey($registryId)
            ->where('identity_id', $identity->id)
            ->whereNull('revoked_at')
            ->update(['revoked_at' => now(), 'updated_at' => now()]) === 1;
    }

    public function revokeOthers(Identity $identity, string $currentRegistryId): int
    {
        return IdentityWebSession::query()
            ->where('identity_id', $identity->id)
            ->whereKeyNot($currentRegistryId)
            ->whereNull('revoked_at')
            ->update(['revoked_at' => now(), 'updated_at' => now()]);
    }

    public function revokeAll(Identity $identity): int
    {
        return IdentityWebSession::query()
            ->where('identity_id', $identity->id)
            ->whereNull('revoked_at')
            ->update(['revoked_at' => now(), 'updated_at' => now()]);
    }

    /** @return Collection<int, IdentityWebSession> */
    public function activeFor(Identity $identity): Collection
    {
        return IdentityWebSession::query()
            ->where('identity_id', $identity->id)
            ->whereNull('revoked_at')
            ->where('expires_at', '>', now())
            ->latest('last_seen_at')
            ->limit(50)
            ->get();
    }

    private function boundedUserAgent(Request $request): ?string
    {
        $userAgent = $request->userAgent();
        if (! is_string($userAgent) || trim($userAgent) === '') {
            return null;
        }

        return Str::limit(trim($userAgent), 160, '');
    }

    private function sourceIpHash(Request $request): ?string
    {
        $sourceIp = $request->ip();
        if (! is_string($sourceIp) || $sourceIp === '') {
            return null;
        }

        $key = config('app.key');
        if (! is_string($key) || $key === '') {
            throw new LogicException('The application key is required to hash session source addresses.');
        }

        return hash_hmac('sha256', $sourceIp, $key);
    }
}
