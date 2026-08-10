<?php

namespace App\Downloads\Security;

use Illuminate\Support\Facades\Config;

final readonly class ArtifactUrlPolicy
{
    /**
     * @var list<string>
     */
    private array $allowedArtifactHosts;

    /**
     * @var array<string, array{type: 'object_version_query', parameter: string}>
     */
    private array $immutableReferenceContracts;

    /**
     * @param  list<string>|null  $allowedArtifactHosts
     * @param  array<string, mixed>|null  $immutableReferenceContracts
     */
    public function __construct(
        ?array $allowedArtifactHosts = null,
        ?array $immutableReferenceContracts = null,
    ) {
        $configuredHosts = $allowedArtifactHosts ?? Config::array('downloads.allowed_artifact_hosts', []);
        $normalizedHosts = [];

        foreach ($configuredHosts as $host) {
            if (! is_string($host)) {
                continue;
            }

            $normalizedHost = strtolower(rtrim(trim($host), '.'));

            if ($normalizedHost !== '' && ! in_array($normalizedHost, $normalizedHosts, true)) {
                $normalizedHosts[] = $normalizedHost;
            }
        }

        $this->allowedArtifactHosts = $normalizedHosts;

        $configuredContracts = $immutableReferenceContracts
            ?? Config::array('downloads.immutable_reference_contracts', []);
        $normalizedContracts = [];

        foreach ($configuredContracts as $host => $contract) {
            if (! is_string($host) || ! is_array($contract)) {
                continue;
            }

            $normalizedHost = strtolower(rtrim(trim($host), '.'));
            $type = $contract['type'] ?? null;
            $parameter = $contract['parameter'] ?? null;

            if (
                $normalizedHost === ''
                || $type !== 'object_version_query'
                || ! is_string($parameter)
                || preg_match('/^[A-Za-z][A-Za-z0-9_-]{0,63}$/', $parameter) !== 1
            ) {
                continue;
            }

            $normalizedContracts[$normalizedHost] = [
                'type' => 'object_version_query',
                'parameter' => $parameter,
            ];
        }

        $this->immutableReferenceContracts = $normalizedContracts;
    }

    public function isApproved(string $url): bool
    {
        return $this->rejectionReason($url) === null;
    }

    public function rejectionReason(string $url): ?string
    {
        if ($url === '' || trim($url) !== $url) {
            return 'must be a normalized absolute URL.';
        }

        if (preg_match('/[\x00-\x1F\x7F]/', $url) === 1 || filter_var($url, FILTER_VALIDATE_URL) === false) {
            return 'must be a valid absolute URL.';
        }

        $scheme = parse_url($url, PHP_URL_SCHEME);
        $host = parse_url($url, PHP_URL_HOST);
        $path = parse_url($url, PHP_URL_PATH);
        $query = parse_url($url, PHP_URL_QUERY);
        $port = parse_url($url, PHP_URL_PORT);
        $user = parse_url($url, PHP_URL_USER);
        $password = parse_url($url, PHP_URL_PASS);
        $fragment = parse_url($url, PHP_URL_FRAGMENT);

        if (! is_string($scheme) || strtolower($scheme) !== 'https') {
            return 'uses a scheme that is not approved.';
        }

        if (! is_string($host) || $host === '') {
            return 'must be a valid absolute URL.';
        }

        $normalizedHost = strtolower(rtrim($host, '.'));

        if (! in_array($normalizedHost, $this->allowedArtifactHosts, true)) {
            return 'uses a host that is not approved.';
        }

        if (is_string($user) || is_string($password)) {
            return 'must not contain URL user information.';
        }

        if (is_string($fragment)) {
            return 'must not contain a fragment.';
        }

        if ($port !== null && (! is_int($port) || $port !== 443)) {
            return 'must use the standard HTTPS port.';
        }

        if (! is_string($path) || $path === '' || $path === '/') {
            return 'must reference a concrete artifact path.';
        }

        $contract = $this->immutableReferenceContracts[$normalizedHost] ?? null;

        if ($contract === null) {
            return 'does not have an approved immutable-reference contract.';
        }

        if (! is_string($query) || $query === '') {
            return 'must include the configured immutable object-version reference.';
        }

        $versionId = $this->objectVersionFromQuery($query, $contract['parameter']);

        if ($versionId === null) {
            return 'must contain exactly one configured immutable object-version query parameter.';
        }

        if (preg_match('/^[A-Za-z0-9._~+\/=:-]{8,256}$/', $versionId) !== 1) {
            return 'contains an invalid immutable object-version identifier.';
        }

        return null;
    }

    private function objectVersionFromQuery(string $query, string $parameter): ?string
    {
        $pairs = explode('&', $query);

        if (count($pairs) !== 1) {
            return null;
        }

        $parts = explode('=', $pairs[0], 2);

        if (count($parts) !== 2) {
            return null;
        }

        $name = rawurldecode($parts[0]);
        $value = rawurldecode($parts[1]);

        if ($name !== $parameter || $value === '') {
            return null;
        }

        return $value;
    }
}
