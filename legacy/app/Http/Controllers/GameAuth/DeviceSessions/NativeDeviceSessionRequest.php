<?php

namespace App\Http\Controllers\GameAuth\DeviceSessions;

use Illuminate\Http\Request;
use InvalidArgumentException;
use JsonException;

/** Strict, flat candidate protocol. Device secrets are never accepted in the body or URL. */
final class NativeDeviceSessionRequest
{
    public const MAX_BODY_BYTES = 1024;

    /** Enrollment returns null; remembered operations return only the public client identifier. */
    public static function decode(Request $request, bool $enrollment): ?string
    {
        if ($request->query->count() !== 0) {
            throw new InvalidArgumentException('Invalid device request.');
        }
        $body = $request->getContent();
        if ($body === '' || strlen($body) > self::MAX_BODY_BYTES) {
            throw new InvalidArgumentException('Invalid device request.');
        }

        // Same duplicate-member rejection as the owning native evidence decoder. Escaped
        // spellings are decoded before comparison, rather than trusting json_decode's last key.
        preg_match_all('/"((?:[^"\\\\]|\\\\.)*)"\s*:/s', $body, $matches);
        $seen = [];
        try {
            foreach ($matches[1] as $encodedName) {
                $name = json_decode('"'.$encodedName.'"', true, 2, JSON_THROW_ON_ERROR);
                if (! is_string($name) || isset($seen[$name])) {
                    throw new InvalidArgumentException('Invalid device request.');
                }
                $seen[$name] = true;
            }
            $decoded = json_decode($body, true, 3, JSON_THROW_ON_ERROR);
        } catch (JsonException) {
            throw new InvalidArgumentException('Invalid device request.');
        }
        if (! is_array($decoded) || array_is_list($decoded)) {
            throw new InvalidArgumentException('Invalid device request.');
        }
        $keys = array_keys($decoded);
        sort($keys);
        $expected = $enrollment ? ['protocol_version', 'remember_device'] : ['client_id', 'protocol_version'];
        if ($keys !== $expected || ($decoded['protocol_version'] ?? null) !== 1) {
            throw new InvalidArgumentException('Invalid device request.');
        }
        if ($enrollment) {
            if (($decoded['remember_device'] ?? null) !== true) {
                throw new InvalidArgumentException('Invalid device request.');
            }

            return null;
        }
        $clientId = $decoded['client_id'] ?? null;
        if (! is_string($clientId)
            || preg_match('/\A[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/', $clientId) !== 1) {
            throw new InvalidArgumentException('Invalid device request.');
        }

        return $clientId;
    }
}
