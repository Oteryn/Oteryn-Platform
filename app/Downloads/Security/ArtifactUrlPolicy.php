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
     * @param  list<string>|null  $allowedArtifactHosts
     */
    public function __construct(?array $allowedArtifactHosts = null)
    {
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
            return 'must reference a concrete immutable artifact path.';
        }

        return null;
    }
}
