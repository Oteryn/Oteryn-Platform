<?php

namespace App\Console\Commands;

use App\Operations\MailDeliveryConfigurationVerifier;
use Illuminate\Console\Command;
use Symfony\Component\Console\Command\Command as SymfonyCommand;

final class VerifyMailDeliveryReadiness extends Command
{
    protected $signature = 'mail:verify-delivery-readiness
        {--environment= : Environment to validate; defaults to APP_ENV}';

    protected $description = 'Verify provider-neutral mail delivery readiness without sending mail';

    public function handle(MailDeliveryConfigurationVerifier $verifier): int
    {
        $option = $this->option('environment');
        $environment = is_string($option) && trim($option) !== ''
            ? strtolower(trim($option))
            : strtolower(trim((string) config('app.env')));

        $violations = $verifier->inspect($environment);

        if ($violations !== []) {
            $this->error('Mail delivery readiness verification failed.');

            foreach ($violations as $violation) {
                $this->line("- {$violation}");
            }

            return SymfonyCommand::FAILURE;
        }

        if (in_array($environment, ['local', 'testing'], true)) {
            $this->info('Mail delivery readiness is not required for this inert application environment.');

            return SymfonyCommand::SUCCESS;
        }

        $this->info('Mail delivery configuration is structurally ready for deployment verification.');

        return SymfonyCommand::SUCCESS;
    }
}
