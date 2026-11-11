<?php

namespace App\CharacterAuthority\Restore\Http;

use App\CharacterAuthority\Restore\GameRestoreNoticeSettings;
use App\Http\Middleware\GameAuth\PreventSensitiveGameAuthResponseCaching;
use Closure;
use Illuminate\Cache\RateLimiter;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

/**
 * Default-off switch (503), dedicated restore-notice mTLS identity over TLS 1.3 (401) and a
 * per-identity rate limit (429) for the Game restore notice route. Every refusal is an empty body
 * and no response is cacheable.
 */
final class GuardGameRestoreNoticePeer
{
    public function __construct(private readonly RateLimiter $limiter) {}

    public function handle(Request $request, Closure $next): Response
    {
        return PreventSensitiveGameAuthResponseCaching::apply($this->guarded($request, $next));
    }

    private function guarded(Request $request, Closure $next): Response
    {
        $settings = GameRestoreNoticeSettings::current();
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

        $key = 'game-restore-notice:'.hash('sha256', $identity);
        $response = $this->limiter->attempt($key, $settings->requestsPerMinute, fn (): mixed => $next($request), 60);
        if ($response === false) {
            return response('', 429, ['Retry-After' => (string) max(1, $this->limiter->availableIn($key))]);
        }

        return $response instanceof Response ? $response : response('', 500);
    }
}
