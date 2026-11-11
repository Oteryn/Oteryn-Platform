<?php

namespace App\GameAuth\NativeLogin;

use JsonException;
use SensitiveParameter;

/**
 * Exact §3.1 request body of the native admission issuer: at most 2048 bytes, exact member set (no
 * unknown, duplicate, missing or null member except `channel_id`), nesting depth at most 3 containers
 * below the top object. `protocol_version` other than 2 is NATIVE_LOGIN_UNSUPPORTED_VERSION.
 */
final class NativeAdmissionWire
{
    public const MAX_REQUEST_BYTES = 2048;

    private const MEMBERS = ['attempt_ref', 'channel_id', 'character_id', 'game_login_ticket', 'offer', 'protocol_version'];

    private const UUID_V7 = '/\A[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/';

    /** The canonical attempt_ref of the body, for the §11.1 echo; null when it did not parse. */
    public static function attemptRef(#[SensitiveParameter] string $raw): ?string
    {
        $decoded = self::json($raw);
        $attemptRef = is_array($decoded) ? ($decoded['attempt_ref'] ?? null) : null;

        return is_string($attemptRef) && preg_match(self::UUID_V7, $attemptRef) === 1 ? $attemptRef : null;
    }

    public static function decode(#[SensitiveParameter] string $raw): NativeAdmissionRequest
    {
        $malformed = new NativeLoginRefused(NativeLoginError::RequestMalformed);
        if ($raw === '' || strlen($raw) > self::MAX_REQUEST_BYTES) {
            throw $malformed;
        }
        $decoded = self::json($raw);
        if (! is_array($decoded) || array_is_list($decoded)) {
            throw $malformed;
        }
        $version = $decoded['protocol_version'] ?? null;
        if (! is_int($version)) {
            throw $malformed;
        }
        if ($version !== 2) {
            throw new NativeLoginRefused(NativeLoginError::UnsupportedVersion);
        }

        // A duplicate (even an escaped spelling of a name) adds a member name that decoding dropped.
        preg_match_all('/"((?:[^"\\\\]|\\\\.)*)"\s*:/s', $raw, $names);
        $keys = array_keys($decoded);
        sort($keys, SORT_STRING);
        if ($keys !== self::MEMBERS || count($names[1]) !== self::memberCount($decoded)) {
            throw $malformed;
        }

        $ticket = $decoded['game_login_ticket'];
        $attemptRef = $decoded['attempt_ref'];
        $characterId = $decoded['character_id'];
        $channelId = $decoded['channel_id'];
        $offer = $decoded['offer'];
        if (! is_string($ticket) || ! is_string($attemptRef) || ! is_string($characterId)
            || ! ($channelId === null || is_string($channelId)) || ! is_array($offer) || $offer === [] || array_is_list($offer)) {
            throw $malformed;
        }

        return new NativeAdmissionRequest($ticket, $attemptRef, $characterId, $channelId, $offer);
    }

    /**
     * §3.2 success body, members in contract order.
     *
     * @param  array<string, mixed>  $endpoint
     * @return array<string, mixed>
     */
    public static function success(NativeAdmissionResult $result, array $endpoint): array
    {
        return [
            'protocol_version' => 2,
            'attempt_ref' => $result->attemptRef,
            'world_id' => $result->worldId,
            'channel_id' => $result->channelId,
            'endpoint' => $endpoint,
            'grant' => [
                'profile' => 'oteryn-pre-admission-v1',
                'token' => $result->token,
                'valid_for_seconds' => $result->validForSeconds,
            ],
        ];
    }

    /**
     * §11.1 error body: the public code only; SECURITY_TERMINAL rows collapse.
     *
     * @return array<string, mixed>
     */
    public static function error(NativeLoginError $error, ?string $attemptRef): array
    {
        return [
            'protocol_version' => 2,
            'error' => ['code' => $error->publicCode(), 'public_class' => $error->publicClass()],
            'attempt_ref' => $attemptRef,
        ];
    }

    private static function json(string $raw): mixed
    {
        if ($raw === '' || strlen($raw) > self::MAX_REQUEST_BYTES) {
            return null;
        }
        try {
            // Request object, offer, transports list, transport object, then scalars.
            return json_decode($raw, true, 5, JSON_THROW_ON_ERROR);
        } catch (JsonException) {
            return null;
        }
    }

    /** @param array<mixed> $value */
    private static function memberCount(array $value): int
    {
        $count = array_is_list($value) ? 0 : count($value);
        foreach ($value as $child) {
            if (is_array($child)) {
                $count += self::memberCount($child);
            }
        }

        return $count;
    }
}
