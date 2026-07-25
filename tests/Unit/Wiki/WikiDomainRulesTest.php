<?php

namespace Tests\Unit\Wiki;

use App\Wiki\Domain\Exceptions\InvalidWikiLifecycleTransition;
use App\Wiki\Domain\WikiArticleStatus;
use App\Wiki\Domain\WikiContentRules;
use App\Wiki\Domain\WikiLocale;
use App\Wiki\Domain\WikiTranslationInput;
use InvalidArgumentException;
use PHPUnit\Framework\TestCase;

final class WikiDomainRulesTest extends TestCase
{
    public function test_lifecycle_transitions_are_explicit_and_deterministic(): void
    {
        self::assertTrue(WikiArticleStatus::DRAFT->canTransitionTo(WikiArticleStatus::IN_REVIEW));
        self::assertTrue(WikiArticleStatus::IN_REVIEW->canTransitionTo(WikiArticleStatus::PUBLISHED));
        self::assertTrue(WikiArticleStatus::PUBLISHED->canTransitionTo(WikiArticleStatus::DRAFT));
        self::assertTrue(WikiArticleStatus::DRAFT->canTransitionTo(WikiArticleStatus::ARCHIVED));
        self::assertFalse(WikiArticleStatus::ARCHIVED->canTransitionTo(WikiArticleStatus::DRAFT));

        $this->expectException(InvalidWikiLifecycleTransition::class);
        WikiArticleStatus::DRAFT->assertCanTransitionTo(WikiArticleStatus::PUBLISHED);
    }

    public function test_supported_locales_are_explicit(): void
    {
        self::assertSame(['en', 'pl'], WikiLocale::values());
        WikiContentRules::assertSupportedLocale('en');
        WikiContentRules::assertSupportedLocale('pl');

        $this->expectException(InvalidArgumentException::class);
        WikiContentRules::assertSupportedLocale('de');
    }

    public function test_schema_bound_keys_and_change_notes_use_exact_limits(): void
    {
        WikiContentRules::assertContentType(str_repeat('a', 64));
        WikiContentRules::assertCategoryKey(str_repeat('a', 96));
        WikiContentRules::assertChangeNote(str_repeat('a', 500));

        $rejectedValues = 0;

        foreach ([
            static fn () => WikiContentRules::assertContentType(str_repeat('a', 65)),
            static fn () => WikiContentRules::assertCategoryKey(str_repeat('a', 97)),
            static fn () => WikiContentRules::assertChangeNote(str_repeat('a', 501)),
        ] as $assertion) {
            try {
                $assertion();
                self::fail('Expected the Wiki schema boundary validation to fail.');
            } catch (InvalidArgumentException) {
                $rejectedValues++;
            }
        }

        self::assertSame(3, $rejectedValues);
    }

    public function test_restricted_markdown_rejects_raw_html_and_dangerous_protocols(): void
    {
        $rejectedSources = 0;

        foreach ([
            '<script>alert(1)</script>',
            '[unsafe](javascript:alert(1))',
            '[unsafe](data:text/html,unsafe)',
        ] as $source) {
            try {
                new WikiTranslationInput('en', 'Unsafe', 'unsafe', 'Summary', $source);
                self::fail('Expected restricted Wiki Markdown validation to fail.');
            } catch (InvalidArgumentException) {
                $rejectedSources++;
            }
        }

        self::assertSame(3, $rejectedSources);

        $safe = new WikiTranslationInput(
            'en',
            'Safe guide',
            'safe-guide',
            'Safe summary',
            "# Heading\n\n- safe list\n- [safe link](https://example.com)",
        );

        self::assertSame('safe-guide', $safe->slug);
    }
}
