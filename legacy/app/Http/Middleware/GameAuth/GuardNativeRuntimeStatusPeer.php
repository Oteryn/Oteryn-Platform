<?php

namespace App\Http\Middleware\GameAuth;

use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusSettings;
use App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentSettings;
use Closure;
use Illuminate\Cache\RateLimiter;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

/**
 * Default-off switch (503), per-purpose mTLS identity (401) and per-identity rate limit (429) for
 * `ReportRuntimeStatusV1` (purpose `runtime`) and `ReportScopeAssignmentV1` (purpose `assignment`).
 * Only an identity configured for the route's purpose passes; every refusal is an empty body, as the
 * Game producer requires (§4).
 */
final class GuardNativeRuntimeStatusPeer
{
    public function __construct(private readonly RateLimiter $limiter) {}

    public function handle(Request $request, Closure $next, string $purpose = 'runtime'): Response
    {
        $settings = match ($purpose) {
            'runtime' => NativeRuntimeStatusSettings::current(),
            'assignment' => NativeScopeAssignmentSettings::current(),
            default => null,
        };
        if ($settings === null) {
            return response('', 503);
        }

        $identity = $request->server('SSL_CLIENT_S_DN');
        if ($request->server('SSL_CLIENT_VERIFY') !== 'SUCCESS'
            || $request->server('SSL_PROTOCOL') !== 'TLSv1.3'
            || ! is_string($identity)
            || ! $settings->knows($identity)) {
            return response('', 401);
        }

        $key = ($purpose === 'runtime' ? 'game-auth-native-runtime-status:' : 'game-auth-native-scope-assignment:').hash('sha256', $identity);
        $response = $this->limiter->attempt($key, $settings->requestsPerMinute, fn (): mixed => $next($request), 60);
        if ($response === false) {
            return response('', 429, ['Retry-After' => (string) max(1, $this->limiter->availableIn($key))]);
        }

        return $response instanceof Response ? $response : response('', 500);
    }
}
