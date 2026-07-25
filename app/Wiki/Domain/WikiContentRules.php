<?php

namespace App\Wiki\Domain;

use InvalidArgumentException;

final class WikiContentRules
{
    private const MAX_TITLE_LENGTH = 200;

    private const MAX_SLUG_LENGTH = 160;

    private const MAX_SUMMARY_LENGTH = 1000;

    private const MAX_SOURCE_LENGTH = 100_000;

    private const MAX_DESCRIPTION_LENGTH = 10_000;

    private const MAX_CONTENT_TYPE_LENGTH = 64;

    private const MAX_CATEGORY_KEY_LENGTH = 96;

    private const MAX_CHANGE_NOTE_LENGTH = 500;

    public static function assertSupportedLocale(string $locale): void
    {
        if (WikiLocale::tryFrom($locale) === null) {
            throw new InvalidArgumentException('Unsupported Wiki locale.');
        }
    }

    public static function assertArticleTranslation(
        string $locale,
        string $title,
        string $slug,
        string $summary,
        string $sourceMarkdown,
    ): void {
        self::assertSupportedLocale($locale);
        self::assertRequiredText($title, self::MAX_TITLE_LENGTH, 'Wiki title');
        self::assertSlug($slug);
        self::assertLength($summary, self::MAX_SUMMARY_LENGTH, 'Wiki summary');
        self::assertLength($sourceMarkdown, self::MAX_SOURCE_LENGTH, 'Wiki source Markdown');
        self::assertRestrictedMarkdown($sourceMarkdown);
    }

    public static function assertPublishableArticleTranslation(
        string $title,
        string $slug,
        string $summary,
        string $sourceMarkdown,
    ): void {
        self::assertRequiredText($title, self::MAX_TITLE_LENGTH, 'Wiki title');
        self::assertSlug($slug);
        self::assertRequiredText($summary, self::MAX_SUMMARY_LENGTH, 'Wiki summary');
        self::assertRequiredText($sourceMarkdown, self::MAX_SOURCE_LENGTH, 'Wiki source Markdown');
        self::assertRestrictedMarkdown($sourceMarkdown);
    }

    public static function assertCategoryTranslation(
        string $locale,
        string $name,
        string $slug,
        ?string $description,
    ): void {
        self::assertSupportedLocale($locale);
        self::assertRequiredText($name, self::MAX_TITLE_LENGTH, 'Wiki category name');
        self::assertSlug($slug);

        if ($description !== null) {
            self::assertLength($description, self::MAX_DESCRIPTION_LENGTH, 'Wiki category description');
            self::assertRestrictedMarkdown($description);
        }
    }

    public static function assertContentType(string $contentType): void
    {
        self::assertStableKey($contentType, 'Wiki content type', self::MAX_CONTENT_TYPE_LENGTH);
    }

    public static function assertCategoryKey(string $key): void
    {
        self::assertStableKey($key, 'Wiki category key', self::MAX_CATEGORY_KEY_LENGTH);
    }

    public static function assertChangeNote(?string $changeNote): void
    {
        if ($changeNote !== null) {
            self::assertLength($changeNote, self::MAX_CHANGE_NOTE_LENGTH, 'Wiki change note');
        }
    }

    private static function assertStableKey(string $key, string $label, int $maximumLength): void
    {
        if (
            $key === ''
            || mb_strlen($key) > $maximumLength
            || preg_match('/\A[a-z0-9]+(?:[._-][a-z0-9]+)*\z/D', $key) !== 1
        ) {
            throw new InvalidArgumentException("{$label} must be a bounded lowercase stable key.");
        }
    }

    private static function assertSlug(string $slug): void
    {
        if (
            $slug === ''
            || mb_strlen($slug) > self::MAX_SLUG_LENGTH
            || preg_match('/\A[a-z0-9]+(?:-[a-z0-9]+)*\z/D', $slug) !== 1
        ) {
            throw new InvalidArgumentException('Wiki slug must be a bounded lowercase kebab-case value.');
        }
    }

    private static function assertRequiredText(string $value, int $maximumLength, string $label): void
    {
        if (trim($value) === '') {
            throw new InvalidArgumentException("{$label} is required.");
        }

        self::assertLength($value, $maximumLength, $label);
    }

    private static function assertLength(string $value, int $maximumLength, string $label): void
    {
        if (mb_strlen($value) > $maximumLength) {
            throw new InvalidArgumentException("{$label} exceeds its maximum length.");
        }
    }

    private static function assertRestrictedMarkdown(string $source): void
    {
        if (str_contains($source, '<!--') || preg_match('/<\/?[A-Za-z][^>]*>/', $source) === 1) {
            throw new InvalidArgumentException('Raw HTML is not allowed in Wiki Markdown.');
        }

        if (preg_match('/(?:javascript|data|vbscript)\s*:/i', $source) === 1) {
            throw new InvalidArgumentException('Dangerous URL protocols are not allowed in Wiki Markdown.');
        }
    }
}
