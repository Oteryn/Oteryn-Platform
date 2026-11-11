<?php

namespace App\Identity\Localization;

use App\Localization\PublicLocale;
use Closure;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\URL;
use Symfony\Component\HttpFoundation\Response;

final readonly class SetIdentityLocale
{
    public const SESSION_KEY = 'identity.locale';

    public function __construct(
        private PublicLocale $locales,
    ) {}

    public function handle(Request $request, Closure $next): Response
    {
        $requestedLocale = $request->query('locale');
        if ($requestedLocale !== null) {
            abort_unless(is_string($requestedLocale), 404);

            $normalizedLocale = $this->locales->normalize($requestedLocale);
            abort_unless(is_string($normalizedLocale), 404);

            $request->session()->put(self::SESSION_KEY, $normalizedLocale);
        }

        $storedLocale = $request->session()->get(self::SESSION_KEY);
        $locale = is_string($storedLocale) ? $this->locales->normalize($storedLocale) : null;
        $locale ??= $this->locales->default();

        app()->setLocale($locale);
        URL::defaults(['locale' => $locale]);

        $response = $next($request);
        abort_unless($response instanceof Response, 500);
        $response->headers->set('Content-Language', $locale);

        return $response;
    }
}
