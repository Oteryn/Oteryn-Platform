<?php

namespace App\Http\Middleware\GameAuth;

use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

final class PreventSensitiveGameAuthResponseCaching
{
    /**
     * @param  Closure(Request): Response  $next
     */
    public function handle(Request $request, Closure $next): Response
    {
        $response = $next($request);

        return self::appliesTo($request)
            ? self::apply($response)
            : $response;
    }

    public static function appliesTo(Request $request): bool
    {
        return $request->is('oauth/token')
            || $request->is('api/v1/game-auth/tickets')
            || $request->is('internal/v1/game-auth/tickets/redeem')
            || $request->is('internal/v1/game-auth/native-evidence');
    }

    public static function apply(Response $response): Response
    {
        $response->headers->set('Cache-Control', 'no-store, no-cache, must-revalidate, private');
        $response->headers->set('Pragma', 'no-cache');
        $response->headers->set('Expires', '0');

        return $response;
    }
}
