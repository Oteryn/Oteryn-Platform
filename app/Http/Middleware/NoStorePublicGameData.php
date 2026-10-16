<?php

namespace App\Http\Middleware;

use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

/**
 * Live projection pages (online, highscores, character) are never stored by a shared cache or the
 * browser, so a 30 s freshness bound cannot be stretched by an intermediary. Error responses
 * (404, 429, 503) carry it too; it sits ahead of the throttle in the priority list (bootstrap/app.php).
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
