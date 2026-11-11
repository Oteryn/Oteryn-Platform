<?php

namespace App\Http\Middleware;

use Illuminate\Http\Middleware\TrustProxies;
use Illuminate\Http\Request;
use RuntimeException;

final class TrustConfiguredProxies extends TrustProxies
{
    /**
     * @return list<string>
     */
    protected function proxies(): array
    {
        $configuredProxies = config('http.trusted_proxies', []);

        if (! is_array($configuredProxies)) {
            throw new RuntimeException('http.trusted_proxies must be an array of proxy IP addresses or CIDRs.');
        }

        $trustedProxies = [];

        foreach ($configuredProxies as $proxy) {
            if (! is_string($proxy) || $proxy === '') {
                throw new RuntimeException('http.trusted_proxies must contain only non-empty proxy IP addresses or CIDRs.');
            }

            if ($proxy === '*' || $proxy === '**') {
                throw new RuntimeException('Wildcard trusted proxy configuration is not allowed.');
            }

            $trustedProxies[] = $proxy;
        }

        return $trustedProxies;
    }

    protected function headers(): int
    {
        return Request::HEADER_X_FORWARDED_FOR
            | Request::HEADER_X_FORWARDED_HOST
            | Request::HEADER_X_FORWARDED_PORT
            | Request::HEADER_X_FORWARDED_PROTO;
    }
}
