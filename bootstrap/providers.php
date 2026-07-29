<?php

use App\GameCatalog\GameCatalogServiceProvider;
use App\Providers\AccountSecurityServiceProvider;
use App\Providers\AppServiceProvider;

return [
    AppServiceProvider::class,
    AccountSecurityServiceProvider::class,
    GameCatalogServiceProvider::class,
];
