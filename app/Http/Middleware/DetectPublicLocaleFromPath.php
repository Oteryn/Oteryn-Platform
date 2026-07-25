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
        $locale ??= $this->locales->default();

        app()->setLocale($locale);
        URL::defaults(['locale' => $locale]);

        return $next($request);
    }
}
