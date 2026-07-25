<?php

namespace App\Localization;

use Illuminate\Http\Request;

final readonly class LocaleNegotiator
{
    public function __construct(private PublicLocale $locales) {}

    public function negotiate(Request $request): string
    {
        $queryLocale = $request->query('lang');
        if (is_string($queryLocale)) {
            $normalized = $this->locales->normalize($queryLocale);
            if ($normalized !== null) {
                return $normalized;
            }
        }

        $cookieName = config('localization.cookie', 'oteryn_locale');
        $cookieLocale = is_string($cookieName) ? $request->cookie($cookieName) : null;
        if (is_string($cookieLocale)) {
            $normalized = $this->locales->normalize($cookieLocale);
            if ($normalized !== null) {
                return $normalized;
            }
        }

        foreach ($request->getLanguages() as $language) {
            $normalized = $this->locales->normalize($language);
            if ($normalized !== null) {
                return $normalized;
            }
        }

        return $this->locales->default();
    }
}
