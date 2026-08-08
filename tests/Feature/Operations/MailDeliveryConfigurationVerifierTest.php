<?php

namespace Tests\Feature\Operations;

use App\Operations\MailDeliveryConfigurationVerifier;
use Illuminate\Support\Facades\Artisan;
use Symfony\Component\Console\Command\Command;
use Tests\TestCase;

final class MailDeliveryConfigurationVerifierTest extends TestCase
{
    protected function setUp(): void
    {
        parent::setUp();

        config([
            'app.env' => 'staging',
            'mail.default' => 'smtp',
            'mail.mailers.smtp.transport' => 'smtp',
            'mail.mailers.smtp.host' => 'smtp.internal.oteryn.testnet',
            'mail.mailers.smtp.port' => 587,
            'mail.from.address' => 'noreply@oteryn.com',
        ]);
    }

    public function test_staging_smtp_configuration_is_structurally_ready_without_network_access(): void
    {
        self::assertSame([], app(MailDeliveryConfigurationVerifier::class)->inspect());
        self::assertSame(Command::SUCCESS, Artisan::call('mail:verify-delivery-readiness'));
        self::assertStringContainsString(
            'Mail delivery configuration is structurally ready for deployment verification.',
            Artisan::output(),
        );
    }

    public function test_local_and_testing_environments_may_keep_the_inert_array_mailer(): void
    {
        config([
            'mail.default' => 'array',
            'mail.mailers.array.transport' => 'array',
            'mail.from.address' => 'noreply@example.test',
        ]);

        foreach (['local', 'testing'] as $environment) {
            self::assertSame([], app(MailDeliveryConfigurationVerifier::class)->inspect($environment));
            self::assertSame(Command::SUCCESS, Artisan::call('mail:verify-delivery-readiness', [
                '--environment' => $environment,
            ]));
        }
    }

    public function test_staging_and_production_reject_non_delivery_transports(): void
    {
        foreach (['staging', 'production'] as $environment) {
            foreach (['array', 'log', 'null'] as $transport) {
                config([
                    'mail.default' => $transport,
                    "mail.mailers.{$transport}" => ['transport' => $transport],
                ]);

                self::assertContains(
                    'The default mailer must use a delivery-capable transport.',
                    app(MailDeliveryConfigurationVerifier::class)->inspect($environment),
                );
            }
        }
    }

    public function test_missing_or_unknown_default_mailer_fails_closed(): void
    {
        config(['mail.default' => '']);
        self::assertContains(
            'The default mailer must identify a configured mailer.',
            app(MailDeliveryConfigurationVerifier::class)->inspect('staging'),
        );

        config(['mail.default' => 'missing-provider']);
        self::assertContains(
            'The default mailer must identify a configured mailer.',
            app(MailDeliveryConfigurationVerifier::class)->inspect('production'),
        );
    }

    public function test_smtp_requires_non_empty_host_and_valid_port(): void
    {
        config(['mail.mailers.smtp.host' => '']);
        self::assertContains(
            'SMTP mail delivery requires a non-empty host.',
            app(MailDeliveryConfigurationVerifier::class)->inspect('staging'),
        );

        config(['mail.mailers.smtp.host' => 'smtp.internal.oteryn.testnet']);

        foreach ([0, 65536, 'not-a-port'] as $port) {
            config(['mail.mailers.smtp.port' => $port]);
            self::assertContains(
                'SMTP mail delivery requires a port between 1 and 65535.',
                app(MailDeliveryConfigurationVerifier::class)->inspect('production'),
            );
        }
    }

    public function test_sender_must_be_valid_and_not_use_a_reserved_test_domain(): void
    {
        config(['mail.from.address' => 'not-an-email']);
        self::assertContains(
            'MAIL_FROM_ADDRESS must be a valid email address.',
            app(MailDeliveryConfigurationVerifier::class)->inspect('staging'),
        );

        config(['mail.from.address' => 'noreply@example.test']);
        self::assertContains(
            'MAIL_FROM_ADDRESS must not use a reserved test domain.',
            app(MailDeliveryConfigurationVerifier::class)->inspect('production'),
        );
    }

    public function test_unknown_deployment_environment_fails_closed(): void
    {
        self::assertContains(
            'Mail delivery readiness cannot classify the configured application environment.',
            app(MailDeliveryConfigurationVerifier::class)->inspect('qa'),
        );
    }

    public function test_command_failure_does_not_print_mail_credentials(): void
    {
        config([
            'mail.default' => 'smtp',
            'mail.mailers.smtp.host' => '',
            'mail.mailers.smtp.username' => 'secret-user',
            'mail.mailers.smtp.password' => 'secret-password',
        ]);

        self::assertSame(Command::FAILURE, Artisan::call('mail:verify-delivery-readiness', [
            '--environment' => 'production',
        ]));

        self::assertStringContainsString('Mail delivery readiness verification failed.', Artisan::output());
        self::assertStringNotContainsString('secret-user', Artisan::output());
        self::assertStringNotContainsString('secret-password', Artisan::output());
    }
}
