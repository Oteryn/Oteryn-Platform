<?php

namespace App\Providers;

use Illuminate\Support\Facades\Schedule;
use Illuminate\Support\ServiceProvider;

final class AccountSecurityServiceProvider extends ServiceProvider
{
    public function boot(): void
    {
        Schedule::command('identity:finalize-terminations --limit=100')
            ->hourly()
            ->withoutOverlapping(10);
    }
}
