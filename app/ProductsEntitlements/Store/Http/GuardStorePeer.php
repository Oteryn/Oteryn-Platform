<?php

namespace App\ProductsEntitlements\Store\Http;

use App\Http\Middleware\GameAuth\PreventSensitiveGameAuthResponseCaching;
use App\ProductsEntitlements\Store\StoreSettings;
use Closure;
use Illuminate\Cache\RateLimiter;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

final class GuardStorePeer
{
    public function __construct(private readonly RateLimiter $limiter) {}

    public function handle(Request $request, Closure $next, string $surface): Response
    {
        $settings = StoreSettings::current($surface);
        if ($settings === null) {
            return $this->failure(503);
        }
        $identity = $request->server('SSL_CLIENT_S_DN');
        if ($request->server('SSL_CLIENT_VERIFY') !== 'SUCCESS' || $request->server('SSL_PROTOCOL') !== 'TLSv1.3'
            || ! is_string($identity) || strlen($identity) > 128 || ! hash_equals($settings->peerIdentity, $identity)) {
            return $this->failure(401);
        }
        $key = 'products-entitlements-store:'.hash('sha256', $identity);
        $response = $this->limiter->attempt($key, $settings->requestsPerMinute, fn (): mixed => $next($request), 60);
        if ($response === false) {
            return PreventSensitiveGameAuthResponseCaching::apply(response('', 429,
                ['Retry-After' => (string) max(1, $this->limiter->availableIn($key))]));
        }

        return PreventSensitiveGameAuthResponseCaching::apply($response instanceof Response ? $response : response('', 500));
    }

    private function failure(int $status): Response
    {
        return PreventSensitiveGameAuthResponseCaching::apply(response('', $status));
    }
}
