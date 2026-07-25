<?php

namespace App\Localization;

use InvalidArgumentException;

final class PublicLocale
{
    /** @var list<string> */
    private const SUPPORTED = ['en', 'pl'];

    /** @return list<string> */
    public function supported(): array
    {
        $configured = config('localization.supported', self::SUPPORTED);

        if ($configured !== self::SUPPORTED) {
            throw new InvalidArgumentException('Supported public locales must be exactly en and pl.');
        }

        return self::SUPPORTED;
    }

    public function default(): string
    {
        $configured = config('localization.default', 'en');

        if (! is_string($configured)) {
            throw new InvalidArgumentException('The default public locale must be a string.');
        }

        $locale = $this->normalize($configured);

        if ($locale === null) {
            throw new InvalidArgumentException('The default public locale must be supported.');
        }

        return $locale;
    }

    public function supports(string $locale): bool
    {
        return $this->normalize($locale) !== null;
    }

    public function normalize(string $locale): ?string
    {
        $normalized = strtolower(str_replace('_', '-', trim($locale)));
        $primary = explode('-', $normalized, 2)[0];

        return in_array($primary, self::SUPPORTED, true) ? $primary : null;
    }
}
