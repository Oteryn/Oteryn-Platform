<?php

namespace App\Http\Middleware\GameAuth;

use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

final class RequireNativeEvidenceMtlsPeer
{
    public function handle(Request $request, Closure $next): Response
    {
        $expectedIdentity = config('game-auth.native_evidence.mtls_client_identity');
        if (! is_string($expectedIdentity)
            || $expectedIdentity === ''
            || strlen($expectedIdentity) > 128
            || preg_match('/^[\x20-\x7e]+$/', $expectedIdentity) !== 1) {
            return response('', 503);
        }

        $verify = $request->server('SSL_CLIENT_VERIFY');
        $protocol = $request->server('SSL_PROTOCOL');
        $peerIdentity = $request->server('SSL_CLIENT_S_DN');
        if ($verify !== 'SUCCESS'
            || $protocol !== 'TLSv1.3'
            || ! is_string($peerIdentity)
            || strlen($peerIdentity) > 128
            || ! hash_equals($expectedIdentity, $peerIdentity)) {
            return response('', 401);
        }

        $response = $next($request);
        if (! $response instanceof Response) {
            return response('', 500);
        }

        return $response;
    }
}
