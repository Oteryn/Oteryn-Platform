<?php

namespace App\ProductsEntitlements\Store\Console;

use App\ProductsEntitlements\Store\StoreCatalogImporter;
use App\ProductsEntitlements\Store\StorePriceList;
use Illuminate\Console\Command;
use Illuminate\Database\QueryException;
use InvalidArgumentException;
use RuntimeException;

final class ImportStoreCatalog extends Command
{
    protected $signature = 'store:catalog:import {file : Game-authored canonical price list file}';

    protected $description = 'Import immutable Game Store prices verbatim without activating them';

    public function handle(StoreCatalogImporter $importer): int
    {
        $path = $this->argument('file');
        if (! is_file($path) || ! is_readable($path)) {
            $this->error('Store price list must be a readable local file.');

            return self::FAILURE;
        }
        $stream = fopen($path, 'rb');
        if ($stream === false) {
            return self::FAILURE;
        }
        try {
            $raw = stream_get_contents($stream, StorePriceList::MAX_BYTES + 1);
        } finally {
            fclose($stream);
        }
        if ($raw === false) {
            return self::FAILURE;
        }
        try {
            $revision = $importer->import($raw);
        } catch (InvalidArgumentException $exception) {
            $this->error($exception->getMessage());

            return self::FAILURE;
        } catch (QueryException|RuntimeException) {
            $this->error('Store catalog import is unavailable.');

            return self::FAILURE;
        }
        $this->info('Imported Store catalog '.$revision.'; activation is separate.');

        return self::SUCCESS;
    }
}
