<?php

namespace App\Casts;

use Closure;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\RateLimiter;
use Illuminate\Support\Str;
use Symfony\Component\HttpFoundation\Response;

final class GuardCastNode
{
    /** @param Closure(Request): Response $next */
    public function handle(Request $request, Closure $next): Response
    {
        $response = $this->guard($request, $next);
        $response->headers->set('Cache-Control', 'no-store, private');
        $response->headers->set('Pragma', 'no-cache');

        return $response;
    }

    /** @param Closure(Request): Response $next */
    private function guard(Request $request, Closure $next): Response
    {
        if (config('casts.enabled') !== true) {
            return response('', 503);
        }
        $subject = $request->server('SSL_CLIENT_S_DN');
        $nodes = config('casts.nodes', []);
        $node = is_string($subject) && is_array($nodes) ? ($nodes[$subject] ?? null) : null;
        if ($request->server('SSL_CLIENT_VERIFY') !== 'SUCCESS' || $request->server('SSL_PROTOCOL') !== 'TLSv1.3'
            || ! is_string($node) || ! Str::isUuid($node)) {
            return response('', 401);
        }
        if (! RateLimiter::attempt('cast-node:'.$node, 10000, fn (): bool => true, 60)) {
            return response('', 429);
        }
        $request->attributes->set('cast_node', $node);

        return $next($request);
    }
}
