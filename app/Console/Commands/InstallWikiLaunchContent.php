<?php

namespace App\Console\Commands;

use App\Identity\Models\Identity;
use App\Identity\Support\CanonicalEmail;
use App\Wiki\Content\WikiExpectedContentValidator;
use App\Wiki\Content\WikiLaunchContentCatalog;
use App\Wiki\Content\WikiLaunchContentInstaller;
use Illuminate\Auth\Access\AuthorizationException;
use Illuminate\Console\Command;
use LogicException;
use Throwable;

final class InstallWikiLaunchContent extends Command
{
    protected $signature = 'wiki:launch-content:install
        {publisher : Email of the enabled MFA-confirmed Wiki publisher}
        {--content-version= : Exact reviewed launch-content version to publish}';

    protected $description = 'Install the reviewed bilingual Wiki launch-content package without overwriting editorial changes.';

    public function handle(
        WikiExpectedContentValidator $validator,
        WikiLaunchContentCatalog $catalog,
        WikiLaunchContentInstaller $installer,
    ): int {
        $requestedVersion = $this->option('content-version');

        if (! is_string($requestedVersion) || $requestedVersion !== WikiLaunchContentCatalog::VERSION) {
            $this->components->error(
                'Pass --content-version='.WikiLaunchContentCatalog::VERSION
                .' to confirm the exact reviewed content package.',
            );

            return self::FAILURE;
        }

        try {
            $validator->validateCatalog($catalog);
        } catch (LogicException $exception) {
            $this->components->error('Wiki expected-content preflight failed: '.$exception->getMessage());

            return self::FAILURE;
        }

        $publisher = $this->argument('publisher');

        $identity = Identity::query()
            ->where('email', CanonicalEmail::normalize($publisher))
            ->first();

        if (! $identity instanceof Identity) {
            $this->components->error('The selected Wiki publisher Identity does not exist.');

            return self::FAILURE;
        }

        try {
            $result = $installer->install($identity);
        } catch (AuthorizationException|LogicException $exception) {
            $this->components->error($exception->getMessage());

            return self::FAILURE;
        } catch (Throwable) {
            $this->components->error('Wiki launch-content installation failed and no partial change was retained.');

            return self::FAILURE;
        }

        if (! $result->changed()) {
            $this->components->info(
                'Wiki launch content '.WikiLaunchContentCatalog::VERSION.' is already installed exactly.',
            );

            return self::SUCCESS;
        }

        $this->components->info(sprintf(
            'Installed Wiki launch content %s: %d categories and %d articles.',
            WikiLaunchContentCatalog::VERSION,
            $result->createdCategories,
            $result->createdArticles,
        ));

        return self::SUCCESS;
    }
}
