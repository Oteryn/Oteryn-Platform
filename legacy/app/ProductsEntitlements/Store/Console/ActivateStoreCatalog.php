<?php

namespace App\ProductsEntitlements\Store\Console;

use App\ProductsEntitlements\Store\StoreCatalogActivator;
use Illuminate\Console\Command;
use Illuminate\Database\QueryException;
use InvalidArgumentException;
use RuntimeException;

final class ActivateStoreCatalog extends Command
{
    protected $signature = 'store:catalog:activate {catalog_revision : Imported SHA-256 revision}';

    protected $description = 'Explicitly activate one imported Game Store price list';

    public function handle(StoreCatalogActivator $activator): int
    {
        $revision = $this->argument('catalog_revision');
        try {
            $activator->activate($revision);
        } catch (InvalidArgumentException $exception) {
            $this->error($exception->getMessage());

            return self::FAILURE;
        } catch (QueryException|RuntimeException) {
            $this->error('Store catalog activation is unavailable.');

            return self::FAILURE;
        }
        $this->info('Active Store catalog '.$revision.'. Store enablement flags are unchanged.');

        return self::SUCCESS;
    }
}
