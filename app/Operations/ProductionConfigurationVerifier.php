<?php

namespace App\Operations;

final class ProductionConfigurationVerifier
{
    /**
     * @return list<string>
     */
    public function inspect(): array
    {
        $violations = [];

        if (config('app.env') !== 'production') {
            $violations[] = 'APP_ENV must be production.';
        }

        if (config('app.debug') !== false) {
            $violations[] = 'APP_DEBUG must be disabled.';
        }

        $appKey = config('app.key');
        if (! is_string($appKey) || trim($appKey) === '') {
            $violations[] = 'APP_KEY must be configured.';
        }

        $appUrl = config('app.url');
        if (! is_string($appUrl) || trim($appUrl) === '') {
            $violations[] = 'APP_URL must be configured as an absolute HTTPS URL.';
        } else {
            $scheme = parse_url($appUrl, PHP_URL_SCHEME);
            $host = parse_url($appUrl, PHP_URL_HOST);

            if (! is_string($scheme) || strtolower($scheme) !== 'https') {
                $violations[] = 'APP_URL must use HTTPS.';
            }

            if (! is_string($host) || $host === '') {
                $violations[] = 'APP_URL must include a valid host.';
            } elseif ($this->isLocalHost($host)) {
                $violations[] = 'APP_URL must not use a localhost or loopback host.';
            }
        }

        if (config('session.secure') !== true) {
            $violations[] = 'Secure session cookies must be enabled.';
        }

        if (config('session.http_only') !== true) {
            $violations[] = 'HttpOnly session cookies must be enabled.';
        }

        if (! $this->hasDeliveryCapableMailer()) {
            $violations[] = 'The default mailer must use a delivery-capable transport.';
        }

        $fromAddress = config('mail.from.address');
        if (! is_string($fromAddress) || ! filter_var($fromAddress, FILTER_VALIDATE_EMAIL)) {
            $violations[] = 'MAIL_FROM_ADDRESS must be a valid email address.';
        } elseif ($this->usesReservedTestDomain($fromAddress)) {
            $violations[] = 'MAIL_FROM_ADDRESS must not use a reserved test domain.';
        }

        if (config('marketplace.enabled')) {
            array_push($violations, ...$this->marketplaceViolations());
        }

        return $violations;
    }

    /** @return list<string> */
    private function marketplaceViolations(): array
    {
        $violations = [];
        $escrowAccountId = config('marketplace.escrow_canary_account_id');
        if (! is_int($escrowAccountId) || $escrowAccountId <= 0) {
            $violations[] = 'MARKETPLACE_ESCROW_CANARY_ACCOUNT_ID must be a positive Canary account ID.';
        }

        $durations = config('marketplace.allowed_duration_days');
        if (! is_array($durations) || $durations === []) {
            $violations[] = 'Character Bazaar auction durations must be a non-empty unique list.';
        } elseif (array_filter($durations, static fn (mixed $duration): bool => ! is_int($duration)) !== []) {
            $violations[] = 'Character Bazaar auction durations must be integers between 1 and 30 days.';
        } else {
            /** @var array<array-key, int> $durations */
            if (array_values(array_unique($durations)) !== array_values($durations)) {
                $violations[] = 'Character Bazaar auction durations must be a non-empty unique list.';
            } elseif (array_filter($durations, static fn (int $duration): bool => $duration < 1 || $duration > 30) !== []) {
                $violations[] = 'Character Bazaar auction durations must be integers between 1 and 30 days.';
            }
        }

        foreach (['minimum_starting_bid', 'minimum_bid_increment', 'escrow_quiescence_seconds', 'public_bid_history_limit', 'character_limit'] as $key) {
            $value = config("marketplace.{$key}");
            if (! is_int($value) || $value < 1) {
                $violations[] = "Character Bazaar {$key} must be a positive integer.";
            }
        }

        $commission = config('marketplace.commission_basis_points');
        if (! is_int($commission) || $commission < 0 || $commission > 10_000) {
            $violations[] = 'Character Bazaar commission must be between 0 and 10000 basis points.';
        }

        $transferUsername = config('database.connections.canary_character_transfer.username');
        if (! is_string($transferUsername) || trim($transferUsername) === '' || $transferUsername === 'root') {
            $violations[] = 'The dedicated Canary character-transfer database username must be configured and must not be root.';
        }

        return $violations;
    }

    private function hasDeliveryCapableMailer(): bool
    {
        $defaultMailer = config('mail.default');

        if (! is_string($defaultMailer) || $defaultMailer === '') {
            return false;
        }

        $transport = config("mail.mailers.{$defaultMailer}.transport");

        return is_string($transport)
            && $transport !== ''
            && ! in_array(strtolower($transport), ['array', 'log'], true);
    }

    private function isLocalHost(string $host): bool
    {
        $host = strtolower(trim($host, '[]'));

        return $host === 'localhost'
            || str_ends_with($host, '.localhost')
            || str_starts_with($host, '127.')
            || $host === '::1';
    }

    private function usesReservedTestDomain(string $email): bool
    {
        $separator = strrpos($email, '@');

        if ($separator === false) {
            return true;
        }

        $domain = strtolower(substr($email, $separator + 1));

        foreach (['.test', '.example', '.invalid', '.localhost'] as $suffix) {
            if ($domain === ltrim($suffix, '.') || str_ends_with($domain, $suffix)) {
                return true;
            }
        }

        return false;
    }
}
