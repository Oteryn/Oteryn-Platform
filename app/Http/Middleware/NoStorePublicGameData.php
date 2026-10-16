<?php

namespace App\Http\Middleware;

use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

/**
 * Live projection pages (online, highscores, character) are never stored by a shared cache or the
 * browser, so a 30 s freshness bound cannot be stretched by an intermediary. Error responses
 * (404, 503) carry it too.
 */
final class NoStorePublicGameData
{
    /**
     * @param  Closure(Request): Response  $next
     */
    public function handle(Request $request, Closure $next): Response
    {
        $response = $next($request);
        $response->headers->set('Cache-Control', 'no-store, private');

        return $response;
    }
}
