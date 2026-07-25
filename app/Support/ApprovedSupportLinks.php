<?php

namespace App\Support;

final class ApprovedSupportLinks
{
    /**
     * @return list<array{label: string, href: string, detail: string|null, external: bool}>
     */
    public function all(): array
    {
        $links = [];

        $discord = $this->approvedHttpsUrl(
            config('support.discord_url'),
            $this->configuredHosts(config('support.discord_hosts')),
        );

        if ($discord !== null) {
            $links[] = [
                'label' => 'Join the official Discord',
                'href' => $discord,
                'detail' => null,
                'external' => true,
            ];
        }

        $email = config('support.contact_email');

        if (is_string($email) && filter_var($email, FILTER_VALIDATE_EMAIL) !== false) {
            $links[] = [
                'label' => 'Contact support',
                'href' => 'mailto:'.$email,
                'detail' => $email,
                'external' => false,
            ];
        }

        $supportUrl = $this->approvedHttpsUrl(
            config('support.support_url'),
            $this->configuredHosts(config('support.allowed_hosts')),
        );

        if ($supportUrl !== null) {
            $links[] = [
                'label' => 'Open the approved support service',
                'href' => $supportUrl,
                'detail' => null,
                'external' => true,
            ];
        }

        return $links;
    }

    /**
     * @return list<string>
     */
    private function configuredHosts(mixed $hosts): array
    {
        if (! is_array($hosts)) {
            return [];
        }

        $approved = [];

        foreach ($hosts as $host) {
            if (! is_string($host)) {
                continue;
            }

            $host = strtolower(rtrim(trim($host), '.'));

            if ($host !== '') {
                $approved[] = $host;
            }
        }

        return array_values(array_unique($approved));
    }

    /**
     * @param  list<string>  $approvedHosts
     */
    private function approvedHttpsUrl(mixed $candidate, array $approvedHosts): ?string
    {
        if (! is_string($candidate) || $candidate === '' || filter_var($candidate, FILTER_VALIDATE_URL) === false) {
            return null;
        }

        $scheme = parse_url($candidate, PHP_URL_SCHEME);
        $host = parse_url($candidate, PHP_URL_HOST);

        if (! is_string($scheme) || strtolower($scheme) !== 'https' || ! is_string($host)) {
            return null;
        }

        $host = strtolower(rtrim($host, '.'));

        foreach ($approvedHosts as $approvedHost) {
            if ($host === $approvedHost || str_ends_with($host, '.'.$approvedHost)) {
                return $candidate;
            }
        }

        return null;
    }
}
