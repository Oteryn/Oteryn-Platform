<?php

namespace App\Http\Middleware;

use App\Localization\PublicLocale;
use Closure;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\URL;
use Symfony\Component\HttpFoundation\Response;

final readonly class DetectPublicLocaleFromPath
{
    public function __construct(private PublicLocale $locales) {}

    public function handle(Request $request, Closure $next): Response
    {
        $segment = $request->segment(1);
        $locale = is_string($segment) ? $this->locales->normalize($segment) : null;

        if ($locale === null && $request->query->has('locale')) {
            $requestedLocale = $request->query('locale');
            abort_unless(is_string($requestedLocale), 404);

            $locale = $this->locales->normalize($requestedLocale);
            abort_unless(is_string($locale), 404);
        }

        $locale ??= $this->locales->default();

        app()->setLocale($locale);
        URL::defaults(['locale' => $locale]);

        $response = $next($request);
        abort_unless($response instanceof Response, 500);

        return $response;
    }
}
