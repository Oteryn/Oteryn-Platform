<?php

namespace App\ProductsEntitlements\Store\Purchase;

use Illuminate\Cache\RateLimiter;
use Symfony\Component\HttpFoundation\Response;

final class StoreAccountLimiter
{
    public function __construct(private readonly RateLimiter $limiter) {}

    public function refuse(string $account, int $limit, string $scope): ?Response
    {
        $key = 'products-entitlements-store-account:'.$scope.':'.hash('sha256', $account);
        if ($this->limiter->tooManyAttempts($key, $limit)) {
            return response('', 429, ['Retry-After' => (string) max(1, $this->limiter->availableIn($key))]);
        }
        $this->limiter->hit($key, 60);

        return null;
    }
}
