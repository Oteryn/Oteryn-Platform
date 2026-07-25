<?php

namespace App\Http\Middleware;

use App\Localization\PublicLocale;
use Closure;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\URL;
use Symfony\Component\HttpFoundation\Response;

final readonly class SetPublicLocale
{
    public function __construct(private PublicLocale $locales) {}

    public function handle(Request $request, Closure $next): Response
    {
        $routeLocale = $request->route('locale');
        abort_unless(is_string($routeLocale), 404);

        $locale = $this->locales->normalize($routeLocale);
        abort_if($locale === null, 404);

        app()->setLocale($locale);
        URL::defaults(['locale' => $locale]);

        $response = $next($request);
        $response->headers->set('Content-Language', $locale);
        $this->remember($request, $response, $locale);

        return $response;
    }

    private function remember(Request $request, Response $response, string $locale): void
    {
        $cookieName = config('localization.cookie', 'oteryn_locale');
        $minutes = config('localization.cookie_minutes', 525600);

        if (is_string($cookieName) && $cookieName !== '' && is_int($minutes) && $minutes > 0) {
            $response->headers->setCookie(cookie(
                $cookieName,
                $locale,
                $minutes,
                '/',
                null,
                $request->isSecure(),
                true,
                false,
                'lax',
            ));
        }
    }
}
