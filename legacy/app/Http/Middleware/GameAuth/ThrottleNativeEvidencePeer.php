<?php

namespace App\Http\Middleware\GameAuth;

use Closure;
use Illuminate\Cache\RateLimiter;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

final class ThrottleNativeEvidencePeer
{
    private const WINDOW_SECONDS = 60;

    public function __construct(private readonly RateLimiter $limiter) {}

    public function handle(Request $request, Closure $next): Response
    {
        $maximumAttempts = $this->maximumAttempts();
        if ($maximumAttempts === null) {
            return $this->closedResponse(503);
        }

        $key = $this->rateLimitKey($request);
        $response = $this->limiter->attempt(
            $key,
            $maximumAttempts,
            fn (): mixed => $next($request),
            self::WINDOW_SECONDS,
        );

        if ($response === false) {
            return $this->closedResponse(429, [
                'Retry-After' => (string) max(1, $this->limiter->availableIn($key)),
            ]);
        }

        if (! $response instanceof Response) {
            return $this->closedResponse(500);
        }

        return $response;
    }

    private function maximumAttempts(): ?int
    {
        $value = config('game-auth.native_evidence.requests_per_minute');
        if (is_string($value)) {
            if (preg_match('/^[1-9][0-9]{0,2}$/D', $value) !== 1) {
                return null;
            }

            $value = (int) $value;
        }

        return is_int($value) && $value >= 1 && $value <= 600 ? $value : null;
    }

    private function rateLimitKey(Request $request): string
    {
        $peerIdentity = $request->server('SSL_CLIENT_S_DN');
        $peerKey = is_string($peerIdentity) && $peerIdentity !== ''
            ? hash('sha256', $peerIdentity)
            : 'missing';

        return 'game-auth-native-evidence:'.$peerKey;
    }

    /** @param array<string, string> $headers */
    private function closedResponse(int $status, array $headers = []): Response
    {
        return response('', $status, $headers + [
            'Cache-Control' => 'no-store, private',
            'Pragma' => 'no-cache',
        ]);
    }
}
