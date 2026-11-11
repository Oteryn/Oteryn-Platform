<?php

namespace App\Http\Middleware;

use Closure;
use Illuminate\Http\Request;
use Illuminate\Routing\Route;
use Illuminate\Support\Facades\Log;
use Illuminate\Support\Str;
use Symfony\Component\HttpFoundation\Response;

final class RequestCorrelation
{
    /**
     * @param  Closure(Request): Response  $next
     */
    public function handle(Request $request, Closure $next): Response
    {
        $startedAt = hrtime(true);
        $requestId = (string) Str::uuid();

        $request->attributes->set('request_id', $requestId);

        $response = $next($request);
        $response->headers->set('X-Request-ID', $requestId);

        $route = $request->route();
        $routeName = $route instanceof Route ? $route->getName() : null;
        if (is_string($routeName) && str_starts_with($routeName, 'legacy.')) {
            $routeName = substr($routeName, 7);
        }

        Log::info('http.request.completed', [
            'request_id' => $requestId,
            'method' => $request->getMethod(),
            'route' => $routeName,
            'status' => $response->getStatusCode(),
            'duration_ms' => round((hrtime(true) - $startedAt) / 1_000_000, 3),
        ]);

        return $response;
    }
}
