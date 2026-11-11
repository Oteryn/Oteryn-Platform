<?php

namespace App\Announcements\Links;

use InvalidArgumentException;

final class AnnouncementActionLink
{
    public static function normalize(?string $value): ?string
    {
        if ($value === null || trim($value) === '') {
            return null;
        }

        $link = trim($value);

        if (strlen($link) > 2048 || preg_match('/[\x00-\x1F\x7F]/', $link) === 1) {
            throw new InvalidArgumentException('The action link is invalid.');
        }

        if (str_starts_with($link, '/')) {
            if (str_starts_with($link, '//') || str_contains($link, '\\')) {
                throw new InvalidArgumentException('Internal action links must be absolute site paths.');
            }

            return $link;
        }

        if (filter_var($link, FILTER_VALIDATE_URL) === false) {
            throw new InvalidArgumentException('External action links must be valid HTTPS URLs.');
        }

        $parts = parse_url($link);

        if (
            ! is_array($parts)
            || ($parts['scheme'] ?? null) !== 'https'
            || ! isset($parts['host'])
            || $parts['host'] === ''
            || isset($parts['user'])
            || isset($parts['pass'])
            || (isset($parts['port']) && $parts['port'] !== 443)
        ) {
            throw new InvalidArgumentException('External action links must use approved HTTPS URL boundaries.');
        }

        return $link;
    }
}
