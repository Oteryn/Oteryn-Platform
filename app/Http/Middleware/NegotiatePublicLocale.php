<?php

namespace App\Http\Middleware;

use App\Localization\LocaleNegotiator;
use Closure;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\URL;
use Symfony\Component\HttpFoundation\Response;

final readonly class NegotiatePublicLocale
{
    public function __construct(private LocaleNegotiator $negotiator) {}

    public function handle(Request $request, Closure $next): Response
    {
        $locale = $this->negotiator->negotiate($request);
        app()->setLocale($locale);
        URL::defaults(['locale' => $locale]);
        $request->route()?->setParameter('locale', $locale);

        $response = $next($request);
        $response->headers->set('Content-Language', $locale);
        $response->headers->set('Vary', 'Accept-Language, Cookie', false);

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

        return $response;
    }
}
