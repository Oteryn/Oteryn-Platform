<?php

namespace App\GameProjections\Http;

use App\GameProjections\PublicProjectionSettings;
use Closure;
use Illuminate\Cache\RateLimiter;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

/**
 * Accepts only a configured public-projection client certificate over TLS 1.3 (§3) and rate-limits
 * each identity per route, so a large highscores publication never throttles `world_online`.
 * Responses are never cached.
 */
final class GuardPublicProjectionPeer
{
    public function __construct(private readonly RateLimiter $limiter) {}

    public function handle(Request $request, Closure $next): Response
    {
        return self::noStore($this->guard($request, $next));
    }

    private function guard(Request $request, Closure $next): Response
    {
        $settings = PublicProjectionSettings::current();
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

        $key = 'game-public-projection:'.hash('sha256', $identity.'|'.$request->path());
        $response = $this->limiter->attempt($key, $settings->requestsPerMinute, fn (): mixed => $next($request), 60);
        if ($response === false) {
            return response('', 429, ['Retry-After' => (string) max(1, $this->limiter->availableIn($key))]);
        }

        return $response instanceof Response ? $response : response('', 500);
    }

    private static function noStore(Response $response): Response
    {
        $response->headers->set('Cache-Control', 'no-store, private');
        $response->headers->set('Pragma', 'no-cache');

        return $response;
    }
}
