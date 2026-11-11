<?php

namespace Tests\Feature\GameAuth\OAuth\Concerns;

use OpenSSLAsymmetricKey;

trait ConfiguresEphemeralPassportKeys
{
    private function configureEphemeralPassportKeys(): void
    {
        $key = openssl_pkey_new([
            'private_key_bits' => 2048,
            'private_key_type' => OPENSSL_KEYTYPE_RSA,
        ]);

        if (! $key instanceof OpenSSLAsymmetricKey) {
            self::fail('Unable to generate an ephemeral Passport RSA key.');
        }

        $privateKey = '';

        if (! openssl_pkey_export($key, $privateKey)) {
            self::fail('Unable to export the ephemeral Passport private key.');
        }

        $details = openssl_pkey_get_details($key);

        if (! is_array($details) || ! isset($details['key']) || ! is_string($details['key'])) {
            self::fail('Unable to derive the ephemeral Passport public key.');
        }

        config([
            'passport.private_key' => $privateKey,
            'passport.public_key' => $details['key'],
        ]);
    }
}
