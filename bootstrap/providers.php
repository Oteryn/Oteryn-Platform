<?php

use App\GameCatalog\GameCatalogServiceProvider;
use App\Providers\AccountSecurityServiceProvider;
use App\Providers\AppServiceProvider;
use App\Providers\GameAuthOAuthServiceProvider;

return [
    AppServiceProvider::class,
    AccountSecurityServiceProvider::class,
    GameAuthOAuthServiceProvider::class,
    GameCatalogServiceProvider::class,
];
