<?php

namespace App\Http\Middleware\GameAuth;

use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

final class RequireCharacterBootstrapIntentMtlsPeer
{
    public function handle(Request $request, Closure $next): Response
    {
        $expected = config('game-auth.character_bootstrap_intent.mtls_client_identity');
        if (config('game-auth.character_bootstrap_intent.activated') !== true
            || ! is_string($expected) || $expected === '' || strlen($expected) > 128
            || preg_match('/^[\x20-\x7e]+$/', $expected) !== 1) {
            return response('', 503);
        }
        $peer = $request->server('SSL_CLIENT_S_DN');
        if ($request->server('SSL_CLIENT_VERIFY') !== 'SUCCESS'
            || $request->server('SSL_PROTOCOL') !== 'TLSv1.3'
            || ! is_string($peer) || strlen($peer) > 128 || ! hash_equals($expected, $peer)) {
            return response('', 401);
        }
        $response = $next($request);

        return $response instanceof Response ? $response : response('', 500);
    }
}
