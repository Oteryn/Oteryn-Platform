<?php

namespace App\Http\Middleware\GameAuth;

use Closure;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

final class RequireGatewayServiceCredential
{
    /**
     * @param  Closure(Request): Response  $next
     */
    public function handle(Request $request, Closure $next): Response
    {
        $expectedHashes = $this->configuredHashes();

        if ($expectedHashes === null) {
            return new JsonResponse(['error' => 'service_unavailable'], 503);
        }

        $credential = $request->bearerToken();

        if (! is_string($credential) || $credential === '') {
            return new JsonResponse(['error' => 'unauthorized_service'], 401);
        }

        $presentedHash = hash('sha256', $credential);
        $matched = false;

        foreach ($expectedHashes as $expectedHash) {
            $matches = hash_equals($expectedHash, $presentedHash);
            $matched = $matched || $matches;
        }

        if (! $matched) {
            return new JsonResponse(['error' => 'unauthorized_service'], 401);
        }

        return $next($request);
    }

    /**
     * @return list<string>|null
     */
    private function configuredHashes(): ?array
    {
        $currentHash = config('game-auth.gateway.service_token_sha256');
        $previousHash = config('game-auth.gateway.previous_service_token_sha256');

        if (! is_string($currentHash) || ! $this->isValidHash($currentHash)) {
            return null;
        }

        $hashes = [strtolower($currentHash)];

        if ($previousHash !== null && $previousHash !== '') {
            if (! is_string($previousHash) || ! $this->isValidHash($previousHash)) {
                return null;
            }

            $normalizedPreviousHash = strtolower($previousHash);
            if (! in_array($normalizedPreviousHash, $hashes, true)) {
                $hashes[] = $normalizedPreviousHash;
            }
        }

        return $hashes;
    }

    private function isValidHash(string $hash): bool
    {
        return preg_match('/\A[a-f0-9]{64}\z/i', $hash) === 1;
    }
}
