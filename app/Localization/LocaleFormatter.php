<?php

namespace App\Localization;

use Carbon\CarbonImmutable;
use Carbon\CarbonInterface;
use DateTimeInterface;

final class LocaleFormatter
{
    public function date(DateTimeInterface $date, ?string $locale = null): string
    {
        $locale ??= app()->getLocale();
        $carbon = $date instanceof CarbonInterface ? $date->copy() : CarbonImmutable::instance($date);

        return $carbon->locale($locale)->isoFormat($locale === 'pl' ? 'D MMMM YYYY' : 'MMMM D, YYYY');
    }

    public function dateTime(DateTimeInterface $date, ?string $locale = null): string
    {
        $locale ??= app()->getLocale();
        $carbon = $date instanceof CarbonInterface ? $date->copy() : CarbonImmutable::instance($date);

        return $carbon->locale($locale)->isoFormat($locale === 'pl' ? 'D MMMM YYYY, HH:mm' : 'MMMM D, YYYY, h:mm A');
    }

    public function bytes(int $bytes, ?string $locale = null): string
    {
        $units = ['B', 'KB', 'MB', 'GB', 'TB'];
        $unitIndex = 0;
        $value = (float) $bytes;
        $lastUnitIndex = count($units) - 1;

        while ($value >= 1024 && $unitIndex < $lastUnitIndex) {
            $value /= 1024;
            $unitIndex++;
        }

        $decimals = $unitIndex === 0 || $value >= 100 ? 0 : 1;

        return $this->number($value, $decimals, $locale).' '.$units[$unitIndex];
    }

    public function number(int|float $value, int $decimals = 0, ?string $locale = null): string
    {
        $locale ??= app()->getLocale();

        return $locale === 'pl'
            ? number_format($value, $decimals, ',', "\u{00A0}")
            : number_format($value, $decimals, '.', ',');
    }
}
