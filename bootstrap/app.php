<?php

use App\Http\Middleware\DetectPublicLocaleFromPath;
use App\Http\Middleware\EnsureConfirmedMfa;
use App\Http\Middleware\EnsureIdentitySessionIsCurrent;
use App\Http\Middleware\GameAuth\PreventSensitiveGameAuthResponseCaching;
use App\Http\Middleware\NegotiatePublicLocale;
use App\Http\Middleware\RequestCorrelation;
use App\Http\Middleware\RequireAdminPermission;
use App\Http\Middleware\SecurityHeaders;
use App\Http\Middleware\SetPublicLocale;
use App\Http\Middleware\TrustConfiguredProxies;
use Illuminate\Foundation\Application;
use Illuminate\Foundation\Configuration\Exceptions;
use Illuminate\Foundation\Configuration\Middleware;
use Illuminate\Http\Middleware\TrustProxies;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Route;
use Symfony\Component\HttpFoundation\Response;

return Application::configure(basePath: dirname(__DIR__))
    ->withRouting(
        web: __DIR__.'/../routes/web.php',
        api: __DIR__.'/../routes/api.php',
        commands: __DIR__.'/../routes/console.php',
        health: '/health',
        then: function (): void {
            Route::middleware('api')->group(base_path('routes/internal.php'));
        },
    )
    ->withMiddleware(function (Middleware $middleware): void {
        $middleware->replace(TrustProxies::class, TrustConfiguredProxies::class);
        $middleware->append(DetectPublicLocaleFromPath::class);
        $middleware->append(RequestCorrelation::class);
        $middleware->append(PreventSensitiveGameAuthResponseCaching::class);
        $middleware->redirectGuestsTo('/login');
        $middleware->redirectUsersTo('/');
        $middleware->appendToGroup('web', EnsureIdentitySessionIsCurrent::class);
        $middleware->appendToGroup('web', SecurityHeaders::class);
        $middleware->alias([
            'mfa.confirmed' => EnsureConfirmedMfa::class,
            'admin.permission' => RequireAdminPermission::class,
            'public.locale' => SetPublicLocale::class,
            'public.locale.negotiate' => NegotiatePublicLocale::class,
        ]);
    })
    ->withExceptions(function (Exceptions $exceptions): void {
        $exceptions->shouldRenderJsonWhen(
            fn (Request $request) => $request->is('api/*')
                || $request->is('internal/*')
                || $request->expectsJson(),
        );
        $exceptions->respond(function (Response $response): Response {
            $request = request();
            $response->headers->set('Content-Language', app()->getLocale());

            return PreventSensitiveGameAuthResponseCaching::appliesTo($request)
                ? PreventSensitiveGameAuthResponseCaching::apply($response)
                : $response;
        });
    })->create();
