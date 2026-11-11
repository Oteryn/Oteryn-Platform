<?php

namespace App\Providers;

use App\GameAuth\OAuth\NativeOAuthGenerationBinding;
use Illuminate\Support\Facades\Event;
use Illuminate\Support\ServiceProvider;
use Laravel\Passport\AuthCode;
use Laravel\Passport\Events\AccessTokenCreated;

final class GameAuthOAuthServiceProvider extends ServiceProvider
{
    public function boot(): void
    {
        $app = $this->app;

        AuthCode::creating(
            static function (AuthCode $authCode) use ($app): void {
                $app->make(NativeOAuthGenerationBinding::class)->bindAuthorizationCode($authCode);
            },
        );

        Event::listen(
            AccessTokenCreated::class,
            static function (AccessTokenCreated $event) use ($app): void {
                $app->make(NativeOAuthGenerationBinding::class)->bindAccessToken($event);
            },
        );
    }
}
