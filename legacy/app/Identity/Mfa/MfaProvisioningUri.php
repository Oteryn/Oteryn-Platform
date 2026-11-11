<?php

namespace App\Identity\Mfa;

use App\Identity\Models\Identity;
use PragmaRX\Google2FA\Google2FA;

final class MfaProvisioningUri
{
    public function __construct(
        private readonly Google2FA $google2fa,
    ) {}

    public function forIdentity(Identity $identity): string
    {
        $secret = $identity->two_factor_secret;

        if (! is_string($secret) || $secret === '') {
            throw new MfaStateRejected;
        }

        $configuredIssuer = config('app.name');
        $issuer = is_string($configuredIssuer) && $configuredIssuer !== ''
            ? $configuredIssuer
            : 'Oteryn Platform';
        $label = rawurlencode($issuer.':'.$identity->email);
        $query = http_build_query([
            'secret' => $secret,
            'issuer' => $issuer,
            'algorithm' => strtoupper($this->google2fa->getAlgorithm()),
            'digits' => $this->google2fa->getOneTimePasswordLength(),
            'period' => $this->google2fa->getKeyRegeneration(),
        ], '', '&', PHP_QUERY_RFC3986);

        return 'otpauth://totp/'.$label.'?'.$query;
    }
}
