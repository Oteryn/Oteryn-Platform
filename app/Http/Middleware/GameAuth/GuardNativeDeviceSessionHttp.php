<?php

namespace App\Http\Middleware\GameAuth;

use App\Http\Controllers\GameAuth\DeviceSessions\NativeDeviceSessionRequest;
use Closure;
use Illuminate\Cache\RateLimiter;
use Illuminate\Http\Request;
use InvalidArgumentException;
use Symfony\Component\HttpFoundation\Response;
use Throwable;

final class GuardNativeDeviceSessionHttp
{
    public function __construct(private readonly RateLimiter $limiter) {}

    /** @param Closure(Request): Response $next */
    public function handle(Request $request, Closure $next, string $kind = 'device'): Response
    {
        if (! app()->environment(['testing', 'preproduction'])
            || config('game-auth.device_sessions.enabled', false) !== true) {
            return $this->unavailable();
        }
        try {
            if (! $request->isSecure() && ! $this->explicitLocalDevelopment($request)) {
                return $this->unavailable();
            }
            $maximum = config('game-auth.device_sessions.requests_per_minute', 30);
            if (! is_int($maximum) || $maximum < 1 || $maximum > 60 || ! in_array($kind, ['enroll', 'device'], true)) {
                return $this->unavailable();
            }
            // Independent source limit cannot be bypassed by rotating to a new credential.
            $key = 'native-device-source:'.hash('sha256', $request->ip() ?? 'unknown');
            $response = $this->limiter->attempt($key, $maximum, function () use ($request, $next, $kind): Response {
                $length = $request->headers->get('Content-Length');
                if ($length !== null && (preg_match('/\A(?:0|[1-9][0-9]{0,6})\z/', $length) !== 1
                    || (int) $length > NativeDeviceSessionRequest::MAX_BODY_BYTES)) {
                    return response()->json(['error' => 'invalid_request'], 413);
                }
                $type = $request->headers->get('Content-Type');
                if (! is_string($type) || strtolower(trim(explode(';', $type, 2)[0])) !== 'application/json'
                    || ($request->headers->has('Content-Encoding') && $request->headers->get('Content-Encoding') !== 'identity')
                    || $request->headers->has('Transfer-Encoding')) {
                    return response()->json(['error' => 'invalid_request'], 400);
                }
                $body = $request->getContent();
                if (strlen($body) > NativeDeviceSessionRequest::MAX_BODY_BYTES) {
                    return response()->json(['error' => 'invalid_request'], 413);
                }
                if ($length !== null && strlen($body) !== (int) $length) {
                    return response()->json(['error' => 'invalid_request'], 400);
                }
                $headers = $request->headers->all('authorization');
                $authorization = $headers[0] ?? null;
                if (count($headers) !== 1 || ! is_string($authorization) || strlen($authorization) > 8192) {
                    return response()->json(['error' => 'device_authorization_unavailable'], 401);
                }
                if ($kind === 'enroll') {
                    if (preg_match('/\ABearer [A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\z/', $authorization) !== 1) {
                        return response()->json(['error' => 'device_authorization_unavailable'], 401);
                    }
                } elseif (preg_match('/\AOterynDevice otd1\.[A-Za-z0-9_-]{43}\z/', $authorization) !== 1) {
                    return response()->json(['error' => 'device_authorization_unavailable'], 401);
                }

                $clientId = NativeDeviceSessionRequest::decode($request, $kind === 'enroll');
                $request->attributes->set('native_device_client_id', $clientId);
                $response = $next($request);

                return $response->getStatusCode() >= 500 ? $this->unavailable() : $response;
            }, 60);

            if ($response === false) {
                return response()->json(['error' => 'too_many_requests'], 429, ['Retry-After' => (string) max(1, $this->limiter->availableIn($key))]);
            }

            return $response instanceof Response ? $response : $this->unavailable();
        } catch (InvalidArgumentException) {
            return response()->json(['error' => 'invalid_request'], 400);
        } catch (Throwable) {
            // Never forward exception messages/arguments or report a credential-bearing request.
            return $this->unavailable();
        }
    }

    private function explicitLocalDevelopment(Request $request): bool
    {
        $url = config('app.url');

        return config('game-auth.device_sessions.allow_insecure_loopback', false) === true
            && is_string($url)
            && $request->getHost() === '127.0.0.1'
            && in_array($request->server('REMOTE_ADDR'), ['127.0.0.1', '::1'], true)
            && in_array($request->ip(), ['127.0.0.1', '::1'], true)
            && $request->getScheme() === 'http'
            && hash_equals(rtrim($url, '/'), $request->getSchemeAndHttpHost());
    }

    private function unavailable(): Response
    {
        return response()->json(['error' => 'device_authorization_unavailable'], 503);
    }
}
