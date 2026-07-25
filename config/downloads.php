<?php

$allowedHosts = array_values(array_filter(array_map(
    static fn (string $host): string => strtolower(rtrim(trim($host), '.')),
    explode(',', (string) env('DOWNLOADS_ALLOWED_ARTIFACT_HOSTS', '')),
)));

return [
    'allowed_artifact_schemes' => ['https'],
    'allowed_artifact_hosts' => array_values(array_unique($allowedHosts)),
];
