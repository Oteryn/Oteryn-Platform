<?php

namespace App\GameAuth\OAuth;

use Closure;
use Illuminate\Http\Request;
use Laravel\Passport\Client;
use Symfony\Component\HttpFoundation\Response;

final class RequirePublicClientPkceS256
{
    /**
     * @param  Closure(Request): Response  $next
     */
    public function handle(Request $request, Closure $next): Response
    {
        $clientId = $request->query('client_id');

        if (! is_string($clientId) || $clientId === '') {
            return $next($request);
        }

        $client = Client::query()->find($clientId);

        if (! $client instanceof Client || $client->confidential()) {
            return $next($request);
        }

        if ($request->query('code_challenge_method') !== 'S256') {
            abort(Response::HTTP_BAD_REQUEST);
        }

        return $next($request);
    }
}
