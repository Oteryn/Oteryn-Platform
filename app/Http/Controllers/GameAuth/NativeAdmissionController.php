<?php

namespace App\Http\Controllers\GameAuth;

use App\GameAuth\NativeLogin\NativeAdmissionAttempts;
use App\GameAuth\NativeLogin\NativeAdmissionWire;
use App\GameAuth\NativeLogin\NativeLoginError;
use App\GameAuth\NativeLogin\NativeLoginRefused;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusSettings;
use App\GameAuth\Worlds\NativeRouteRecords;
use Illuminate\Cache\RateLimiter;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Throwable;

/**
 * `POST /internal/v1/game-auth/native-admissions` (login contract §3.3, Decision D1): the private
 * native admission issuer the Gateway calls once per native login. It is reached only behind the
 * Gateway service credential; it rate-limits per credential (§10), decodes the exact §3.1 body, runs
 * the §6.2 issuer transaction and answers with the §3.2 body whose endpoint is the Registry route
 * record bound to the grant's route_revision, or the §11.1 error body. Responses are never cached.
 */
final class NativeAdmissionController
{
    public function __invoke(Request $request, NativeAdmissionAttempts $attempts, NativeRouteRecords $routes, RateLimiter $limiter): JsonResponse
    {
        $raw = $request->getContent();
        $attemptRef = NativeAdmissionWire::attemptRef($raw);

        $limit = NativeRuntimeStatusSettings::bounded(config('game-auth.native_admission.requests_per_minute'), 1, 600);
        if ($limit === null) {
            return self::refused(NativeLoginError::Unavailable, $attemptRef);
        }
        $key = 'game-auth-native-admission:'.hash('sha256', (string) $request->bearerToken());
        if ($limiter->tooManyAttempts($key, $limit)) {
            return self::refused(NativeLoginError::RateLimited, $attemptRef)
                ->header('Retry-After', (string) max(1, $limiter->availableIn($key)));
        }
        $limiter->hit($key, 60);

        try {
            $result = $attempts->admit(NativeAdmissionWire::decode($raw));
            $route = $routes->find($result->worldId, $result->channelId);
            if ($route === null || ! hash_equals($route->routeRevision, $result->routeRevision)) {
                // The route record changed after issuance: the grant would be ROUTE_STALE at admission.
                throw new NativeLoginRefused(NativeLoginError::RouteUnavailable);
            }
        } catch (NativeLoginRefused $refused) {
            return self::refused($refused->error, $attemptRef);
        } catch (Throwable) {
            return self::refused(NativeLoginError::Unavailable, $attemptRef);
        }

        return new JsonResponse(NativeAdmissionWire::success($result, $route->endpoint()), 200, [], JSON_UNESCAPED_SLASHES);
    }

    private static function refused(NativeLoginError $error, ?string $attemptRef): JsonResponse
    {
        return new JsonResponse(NativeAdmissionWire::error($error, $attemptRef), $error->httpStatus(), [], JSON_UNESCAPED_SLASHES);
    }
}
