<?php

namespace App\ProductsEntitlements\Store;

use App\ProductsEntitlements\Store\Console\ActivateStoreCatalog;
use App\ProductsEntitlements\Store\Console\ImportStoreCatalog;
use Illuminate\Support\ServiceProvider;

final class StoreServiceProvider extends ServiceProvider
{
    public function boot(): void
    {
        if ($this->app->runningInConsole()) {
            $this->commands([ImportStoreCatalog::class, ActivateStoreCatalog::class]);
        }
    }
}
