<?php

namespace App\Wiki\Domain;

enum WikiLocale: string
{
    case ENGLISH = 'en';
    case POLISH = 'pl';

    /**
     * @return list<string>
     */
    public static function values(): array
    {
        return array_map(
            static fn (self $locale): string => $locale->value,
            self::cases(),
        );
    }
}
