<?php

namespace App\Http\Middleware\GameAuth;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSettings;
use Closure;
use Illuminate\Cache\RateLimiter;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

final class GuardNativeAccountCharactersPeer
{
    public function __construct(private readonly RateLimiter $limiter) {}

    public function handle(Request $request, Closure $next): Response
    {
        $settings = NativeAccountCharactersSettings::current();
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

        $key = 'game-auth-native-account-characters:'.hash('sha256', $identity);
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
