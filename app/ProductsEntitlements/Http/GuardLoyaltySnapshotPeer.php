<?php

namespace App\ProductsEntitlements\Http;

use App\ProductsEntitlements\Loyalty\LoyaltySnapshotSettings;
use Closure;
use Illuminate\Cache\RateLimiter;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

/** Enforces the dedicated TLS 1.3 peer and per-peer request bound for loyalty snapshots. */
final class GuardLoyaltySnapshotPeer
{
    public function __construct(private readonly RateLimiter $limiter) {}

    public function handle(Request $request, Closure $next): Response
    {
        $settings = LoyaltySnapshotSettings::current();
        if ($settings === null) {
            return response('', 503);
        }

        $peerIdentity = $request->server('SSL_CLIENT_S_DN');
        if ($request->server('SSL_CLIENT_VERIFY') !== 'SUCCESS'
            || $request->server('SSL_PROTOCOL') !== 'TLSv1.3'
            || ! is_string($peerIdentity)
            || strlen($peerIdentity) > 128
            || ! hash_equals($settings->peerIdentity, $peerIdentity)) {
            return response('', 401);
        }

        $key = 'products-entitlements-loyalty-snapshot:'.hash('sha256', $peerIdentity);
        $response = $this->limiter->attempt(
            $key,
            $settings->requestsPerMinute,
            fn (): mixed => $next($request),
            60,
        );
        if ($response === false) {
            return response('', 429, ['Retry-After' => (string) max(1, $this->limiter->availableIn($key))]);
        }

        return $response instanceof Response ? $response : response('', 500);
    }
}
