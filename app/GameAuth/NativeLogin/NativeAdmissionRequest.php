<?php

namespace App\GameAuth\NativeLogin;

use SensitiveParameter;

/**
 * Validated native login request members used by redemption and attempt idempotency
 * (contract §3.1, §6.1). Invalid input is NATIVE_LOGIN_REQUEST_MALFORMED; an offer without a
 * supported transport is NATIVE_LOGIN_OFFER_UNSUPPORTED. character_id and channel_id stay claims
 * to validate: they are never copied into a grant from here.
 */
final readonly class NativeAdmissionRequest
{
    private const UUID_V7 = '/\A[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/';

    private const TICKET = '/\A[\x21-\x7e]{1,256}\z/';

    private const CLIENT_BUILD = '/\A[\x21-\x7e]{1,64}\z/';

    private const ALPN = '/\A[\x21-\x7e]{1,255}\z/';

    private const CLIENT_PLATFORMS = ['windows'];

    /** Lowercase hex SHA-256 of the RFC 8785 serialization of the validated offer. */
    public string $offerDigest;

    /**
     * @param  array<mixed>  $offer
     */
    public function __construct(
        #[SensitiveParameter] public string $ticket,
        public string $attemptRef,
        public string $characterId,
        public ?string $channelId,
        array $offer,
    ) {
        if (preg_match(self::TICKET, $ticket) !== 1
            || preg_match(self::UUID_V7, $attemptRef) !== 1
            || preg_match(self::UUID_V7, $characterId) !== 1
            || ($channelId !== null && preg_match(self::UUID_V7, $channelId) !== 1)) {
            throw new NativeLoginRefused(NativeLoginError::RequestMalformed);
        }

        $this->offerDigest = hash('sha256', self::canonicalOffer($offer));
    }

    /** @return array<string, mixed> */
    public function __debugInfo(): array
    {
        return ['attemptRef' => $this->attemptRef, 'characterId' => $this->characterId, 'channelId' => $this->channelId];
    }

    /**
     * RFC 8785 for the exact offer shape: object members sorted by name, transports kept in request
     * order, ASCII strings and small integers only, so json_encode yields the canonical bytes.
     *
     * @param  array<mixed>  $offer
     */
    private static function canonicalOffer(array $offer): string
    {
        $malformed = static fn (): NativeLoginRefused => new NativeLoginRefused(NativeLoginError::RequestMalformed);

        if (! self::sameKeys($offer, ['client_build', 'client_platform', 'transports'])) {
            throw $malformed();
        }
        $build = $offer['client_build'] ?? null;
        $platform = $offer['client_platform'] ?? null;
        $transports = $offer['transports'] ?? null;
        if (! is_string($build) || preg_match(self::CLIENT_BUILD, $build) !== 1
            || ! is_string($platform) || ! in_array($platform, self::CLIENT_PLATFORMS, true)
            || ! is_array($transports) || ! array_is_list($transports)
            || count($transports) < 1 || count($transports) > 4) {
            throw $malformed();
        }

        $canonical = [];
        $supported = false;
        foreach ($transports as $transport) {
            if (! is_array($transport) || ! self::sameKeys($transport, ['alpn', 'protocol_major', 'transport_profile'])) {
                throw $malformed();
            }
            $alpn = $transport['alpn'] ?? null;
            $major = $transport['protocol_major'] ?? null;
            $profile = $transport['transport_profile'] ?? null;
            if (! is_string($alpn) || preg_match(self::ALPN, $alpn) !== 1
                || ! is_int($major) || $major < 0 || $major > 65535
                || ! is_int($profile) || $profile < 0 || $profile > 65535) {
                throw $malformed();
            }
            $supported = $supported || ($major === 1 && $profile === 1 && $alpn === 'oteryn-game/1');
            $canonical[] = ['alpn' => $alpn, 'protocol_major' => $major, 'transport_profile' => $profile];
        }
        if (! $supported) {
            throw new NativeLoginRefused(NativeLoginError::OfferUnsupported);
        }

        return json_encode(
            ['client_build' => $build, 'client_platform' => $platform, 'transports' => $canonical],
            JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR,
        );
    }

    /**
     * @param  array<mixed>  $value
     * @param  list<string>  $keys
     */
    private static function sameKeys(array $value, array $keys): bool
    {
        $actual = array_keys($value);
        sort($actual, SORT_STRING);

        return $actual === $keys;
    }
}
