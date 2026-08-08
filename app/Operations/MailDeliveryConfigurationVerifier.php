<?php

namespace App\Operations;

final class MailDeliveryConfigurationVerifier
{
    /** @var list<string> */
    private const DEPLOYMENT_ENVIRONMENTS = ['staging', 'production'];

    /** @var list<string> */
    private const INERT_ENVIRONMENTS = ['local', 'testing'];

    /** @var list<string> */
    private const NON_DELIVERY_TRANSPORTS = ['array', 'log', 'null'];

    /** @var list<string> */
    private const SMTP_SCHEMES = ['smtp', 'smtps'];

    /**
     * @return list<string>
     */
    public function inspect(?string $environment = null): array
    {
        if ($environment === null) {
            $configuredEnvironment = config('app.env');
            $environment = is_string($configuredEnvironment) ? $configuredEnvironment : '';
        }

        $environment = strtolower(trim($environment));

        if (in_array($environment, self::INERT_ENVIRONMENTS, true)) {
            return [];
        }

        if (! in_array($environment, self::DEPLOYMENT_ENVIRONMENTS, true)) {
            return ['Mail delivery readiness cannot classify the configured application environment.'];
        }

        $violations = [];
        $defaultMailer = config('mail.default');

        if (! is_string($defaultMailer) || trim($defaultMailer) === '') {
            $violations[] = 'The default mailer must identify a configured mailer.';
        } else {
            $defaultMailer = trim($defaultMailer);
            $mailer = config("mail.mailers.{$defaultMailer}");

            if (! is_array($mailer)) {
                $violations[] = 'The default mailer must identify a configured mailer.';
            } else {
                $transport = $mailer['transport'] ?? null;

                if (! is_string($transport)
                    || trim($transport) === ''
                    || in_array(strtolower(trim($transport)), self::NON_DELIVERY_TRANSPORTS, true)) {
                    $violations[] = 'The default mailer must use a delivery-capable transport.';
                } elseif (strtolower(trim($transport)) === 'smtp') {
                    array_push($violations, ...$this->smtpViolations($mailer));
                }
            }
        }

        $fromAddress = config('mail.from.address');
        if (! is_string($fromAddress) || ! filter_var($fromAddress, FILTER_VALIDATE_EMAIL)) {
            $violations[] = 'MAIL_FROM_ADDRESS must be a valid email address.';
        } elseif ($this->usesReservedTestDomain($fromAddress)) {
            $violations[] = 'MAIL_FROM_ADDRESS must not use a reserved test domain.';
        }

        return $violations;
    }

    /**
     * @param  array<array-key, mixed>  $mailer
     * @return list<string>
     */
    private function smtpViolations(array $mailer): array
    {
        $violations = [];
        $scheme = $mailer['scheme'] ?? null;

        if ($scheme !== null
            && (! is_string($scheme)
                || ! in_array(strtolower(trim($scheme)), self::SMTP_SCHEMES, true))) {
            $violations[] = 'SMTP mail delivery scheme must be smtp, smtps, or unset.';
        }

        $host = $mailer['host'] ?? null;
        if (! is_string($host) || trim($host) === '') {
            $violations[] = 'SMTP mail delivery requires a non-empty host.';
        }

        $port = $mailer['port'] ?? null;
        $validatedPort = filter_var($port, FILTER_VALIDATE_INT, [
            'options' => [
                'min_range' => 1,
                'max_range' => 65535,
            ],
        ]);

        if ($validatedPort === false) {
            $violations[] = 'SMTP mail delivery requires a port between 1 and 65535.';
        }

        return $violations;
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
