<?php

namespace App\Wiki\Application\Media;

use InvalidArgumentException;

final class WikiMediaSyntax
{
    private const TARGET_PATTERN = '/\Awiki-media:([1-9][0-9]{0,18})\z/D';

    public static function mediaId(string $target): ?int
    {
        if (preg_match(self::TARGET_PATTERN, $target, $matches) !== 1) {
            return null;
        }

        $identifier = $matches[1];
        if (strlen($identifier) > strlen((string) PHP_INT_MAX)) {
            return null;
        }

        $mediaId = (int) $identifier;

        return $mediaId > 0 && (string) $mediaId === $identifier ? $mediaId : null;
    }

    public static function consumerId(int $translationId): string
    {
        if ($translationId < 1) {
            throw new InvalidArgumentException('Wiki translation identifiers must be positive integers.');
        }

        return "translation:{$translationId}";
    }

    public static function usage(int $mediaId): string
    {
        if ($mediaId < 1) {
            throw new InvalidArgumentException('Wiki media identifiers must be positive integers.');
        }

        return "body.{$mediaId}";
    }

    public static function markdownToken(int $mediaId, string $altText): string
    {
        if ($mediaId < 1) {
            throw new InvalidArgumentException('Wiki media identifiers must be positive integers.');
        }

        $altText = preg_replace('/\s+/u', ' ', trim($altText));
        if (! is_string($altText) || $altText === '' || mb_strlen($altText) > 500) {
            throw new InvalidArgumentException('Wiki image alternative text must contain 1 to 500 characters.');
        }

        $escapedAltText = str_replace(
            ['\\', '[', ']'],
            ['\\\\', '\\[', '\\]'],
            $altText,
        );

        return sprintf('![%s](wiki-media:%d)', $escapedAltText, $mediaId);
    }
}
