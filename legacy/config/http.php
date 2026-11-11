<?php

$trustedProxies = array_values(array_filter(
    array_map(
        static fn (string $proxy): string => trim($proxy),
        explode(',', (string) env('TRUSTED_PROXIES', '')),
    ),
    static fn (string $proxy): bool => $proxy !== '',
));

if (in_array('*', $trustedProxies, true)) {
    throw new RuntimeException('TRUSTED_PROXIES must contain explicit proxy IP addresses or CIDRs; wildcard trust is not allowed.');
}

return [
    'trusted_proxies' => $trustedProxies,
];
