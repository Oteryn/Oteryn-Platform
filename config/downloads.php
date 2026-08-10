<?php

$defaultAllowedHosts = env('APP_ENV') === 'acceptance'
    ? 'downloads.example.test'
    : '';
$allowedHosts = array_values(array_filter(array_map(
    static fn (string $host): string => strtolower(rtrim(trim($host), '.')),
    explode(',', (string) env('DOWNLOADS_ALLOWED_ARTIFACT_HOSTS', $defaultAllowedHosts)),
)));

$defaultImmutableReferenceContracts = env('APP_ENV') === 'acceptance'
    ? '{"downloads.example.test":{"type":"object_version_query","parameter":"versionId"}}'
    : '{}';
$decodedImmutableReferenceContracts = json_decode(
    (string) env('DOWNLOADS_IMMUTABLE_REFERENCE_CONTRACTS', $defaultImmutableReferenceContracts),
    true,
);
$immutableReferenceContracts = is_array($decodedImmutableReferenceContracts)
    ? $decodedImmutableReferenceContracts
    : [];

return [
    'allowed_artifact_schemes' => ['https'],
    'allowed_artifact_hosts' => array_values(array_unique($allowedHosts)),
    'immutable_reference_contracts' => $immutableReferenceContracts,
];
