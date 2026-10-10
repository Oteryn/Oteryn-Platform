<?php

namespace App\Http\Middleware;

use App\GameAuth\OAuth\NativeOAuthClientManager;
use Closure;
use Illuminate\Http\Request;
use Laravel\Passport\Client;
use LogicException;
use Symfony\Component\HttpFoundation\Response;

final class SecurityHeaders
{
    private const CONTENT_SECURITY_POLICY = "default-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'; object-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'";

    /**
     * @param  Closure(Request): Response  $next
     */
    public function handle(Request $request, Closure $next): Response
    {
        $response = $next($request);

        $response->headers->set('Content-Security-Policy', $this->contentSecurityPolicy($request, $response));
        $response->headers->set('X-Content-Type-Options', 'nosniff');
        $response->headers->set('X-Frame-Options', 'DENY');
        $response->headers->set('Referrer-Policy', 'strict-origin-when-cross-origin');
        $response->headers->set('Permissions-Policy', 'camera=(), geolocation=(), microphone=(), payment=(), usb=()');

        return $response;
    }

    private function contentSecurityPolicy(Request $request, Response $response): string
    {
        if (! $request->isMethod('GET')
            || ! $request->routeIs('passport.authorizations.authorize')
            || $response->getStatusCode() !== Response::HTTP_OK) {
            return self::CONTENT_SECURITY_POLICY;
        }

        $clientId = $request->query('client_id');
        $redirectUri = $request->query('redirect_uri');
        if (! is_string($clientId) || ! is_string($redirectUri)) {
            return self::CONTENT_SECURITY_POLICY;
        }
        $client = Client::query()->find($clientId);
        if (! $client instanceof Client) {
            return self::CONTENT_SECURITY_POLICY;
        }
        try {
            app(NativeOAuthClientManager::class)->assertExpected($client);
        } catch (LogicException) {
            return self::CONTENT_SECURITY_POLICY;
        }

        // Chromium checks the form's OAuth redirect against the originating
        // page's CSP. Permit only this validated native loopback destination.
        if (preg_match('~^http://127\.0\.0\.1:([0-9]{1,5})/callback$~D', $redirectUri, $matches) !== 1
            || (int) $matches[1] < 1 || (int) $matches[1] > 65535) {
            return self::CONTENT_SECURITY_POLICY;
        }

        return str_replace("form-action 'self';", "form-action 'self' {$redirectUri};", self::CONTENT_SECURITY_POLICY);
    }
}
