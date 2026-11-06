<?php

namespace App\Casts;

use App\GameAuth\NativeLogin\NativeAdmissionAttemptRace;
use App\GameAuth\NativeLogin\NativeGameLoginTickets;
use App\GameAuth\NativeLogin\NativeLoginRefused;
use App\Identity\Models\Identity;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\RateLimiter;
use Illuminate\Support\Str;

/** Gateway authenticates the account using the existing one-use native login ticket, without a character. */
final class CastWatchController
{
    public function __invoke(Request $request, NativeGameLoginTickets $tickets, CastControls $controls): JsonResponse
    {
        $started = hrtime(true);
        try {
            if (config('casts.enabled') !== true || strlen($request->getContent()) > 4096 || ! $request->isJson()
                || RateLimiter::hit('cast-gateway:'.hash('sha256', (string) $request->bearerToken()), 60) > 120) {
                throw new CastUnavailable;
            }
            $body = json_decode($request->getContent(), true, 8, JSON_THROW_ON_ERROR);
            if (! is_array($body) || ! is_string($body['accountTicket'] ?? null)
                || strlen($body['accountTicket']) > 2048 || ! is_string($body['attemptRef'] ?? null)
                || ! Str::isUuid($body['attemptRef'])
                || ! is_string($body['sourceIp'] ?? null) || filter_var($body['sourceIp'], FILTER_VALIDATE_IP) === false || ! is_string($body['cast'] ?? null)
                || ! preg_match('/\A[0-9a-f]{32}\z/D', $body['cast'])
                || (isset($body['chat']) && ! is_bool($body['chat']))
                || (isset($body['password']) && (! is_string($body['password']) || strlen($body['password']) > 128))) {
                throw new CastUnavailable;
            }
            // Commit ticket use even when access/password subsequently fails. Never run a KDF under this lock.
            $account = DB::transaction(fn () => $tickets->redeem($body['accountTicket'], $body['attemptRef']));
            $viewer = Identity::query()->whereKey($account->identityId)->first();
            if (! $viewer || (string) $viewer->native_security_generation !== $account->accountSecurityGeneration) {
                throw new CastUnavailable;
            }
            $grant = $controls->reserve($viewer, $body['cast'], $body['password'] ?? null, $body['sourceIp'], $body['chat'] ?? false);
            $route = config('casts.routes.'.$grant->nodeId);
            if (! is_array($route) || ! is_string($route['host'] ?? null) || ! is_int($route['port'] ?? null)
                || $route['port'] < 1 || $route['port'] > 65535) {
                $controls->release($grant->nodeId, $grant->reservationId);
                throw new CastUnavailable;
            }

            return response()->json(['purpose' => 'oteryn-cast-observer-v1', 'ticket' => $grant->ticket,
                'node' => $grant->nodeId, 'host' => $route['host'], 'port' => $route['port'],
                'expiresAt' => $grant->expiresAt, 'chat' => $grant->chatSelected], 200, ['Cache-Control' => 'no-store, private']);
        } catch (CastUnavailable|NativeLoginRefused|NativeAdmissionAttemptRace|\JsonException) {
            // Uniform outward result and a bounded minimum response time, including unknown casts.
            $remaining = 250_000 - intdiv(hrtime(true) - $started, 1000);
            if ($remaining > 0) {
                usleep($remaining);
            }

            return response()->json(['error' => 'Cast unavailable'], 409, ['Cache-Control' => 'no-store, private']);
        }
    }
}
