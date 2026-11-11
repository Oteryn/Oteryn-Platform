<?php

namespace App\Identity\Mfa;

use App\Identity\Models\Identity;
use Illuminate\Http\Request;

final class PendingMfaLogin
{
    public const IDENTITY_ID_KEY = 'identity.mfa.pending.identity_id';

    public const GENERATION_KEY = 'identity.mfa.pending.web_session_generation';

    public const CONFIRMED_AT_KEY = 'identity.mfa.pending.confirmed_at';

    public const ISSUED_AT_KEY = 'identity.mfa.pending.issued_at';

    private const LIFETIME_SECONDS = 300;

    public function begin(Request $request, Identity $identity): void
    {
        $confirmedAt = $identity->two_factor_confirmed_at;

        if (! $identity->hasConfirmedMfa() || $confirmedAt === null) {
            throw new MfaStateRejected;
        }

        $request->session()->regenerate();
        $request->session()->put([
            self::IDENTITY_ID_KEY => $identity->id,
            self::GENERATION_KEY => $identity->web_session_generation,
            self::CONFIRMED_AT_KEY => $confirmedAt->getTimestamp(),
            self::ISSUED_AT_KEY => now()->getTimestamp(),
        ]);
    }

    /**
     * @return array{identity_id: int, generation: int, confirmed_at: int}|null
     */
    public function state(Request $request): ?array
    {
        $identityId = $request->session()->get(self::IDENTITY_ID_KEY);
        $generation = $request->session()->get(self::GENERATION_KEY);
        $confirmedAt = $request->session()->get(self::CONFIRMED_AT_KEY);
        $issuedAt = $request->session()->get(self::ISSUED_AT_KEY);

        if (! is_int($identityId) || ! is_int($generation) || ! is_int($confirmedAt) || ! is_int($issuedAt)) {
            $this->clear($request);

            return null;
        }

        $age = now()->getTimestamp() - $issuedAt;

        if ($age < 0 || $age > self::LIFETIME_SECONDS) {
            $this->clear($request);

            return null;
        }

        return [
            'identity_id' => $identityId,
            'generation' => $generation,
            'confirmed_at' => $confirmedAt,
        ];
    }

    public function clear(Request $request): void
    {
        $request->session()->forget([
            self::IDENTITY_ID_KEY,
            self::GENERATION_KEY,
            self::CONFIRMED_AT_KEY,
            self::ISSUED_AT_KEY,
        ]);
    }
}
