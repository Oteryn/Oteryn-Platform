<?php

namespace Tests\Unit\Wiki;

use App\Wiki\Application\Media\InvalidWikiMediaSyntax;
use App\Wiki\Application\Media\WikiMediaReferenceExtractor;
use App\Wiki\Application\Media\WikiMediaRenderContext;
use App\Wiki\Application\Media\WikiMediaSyntax;
use App\Wiki\Infrastructure\Rendering\CommonMarkWikiRenderer;
use InvalidArgumentException;
use PHPUnit\Framework\TestCase;

final class WikiMediaRendererTest extends TestCase
{
    public function test_media_target_and_reference_identifiers_are_exact_and_bounded(): void
    {
        self::assertSame(17, WikiMediaSyntax::mediaId('wiki-media:17'));
        self::assertSame('translation:23', WikiMediaSyntax::consumerId(23));
        self::assertSame('body.17', WikiMediaSyntax::usage(17));

        foreach ([
            'wiki-media:0',
            'wiki-media:017',
            'wiki-media:+17',
            'wiki-media:17?size=large',
            'wiki-media:17#fragment',
            'wiki-media:%31%37',
            ' wiki-media:17',
            'https://example.test/image.png',
        ] as $target) {
            self::assertNull(WikiMediaSyntax::mediaId($target));
        }
    }

    public function test_only_canonical_allowed_media_renders_with_localized_markdown_alt_text(): void
    {
        $context = new WikiMediaRenderContext([17 => '/en/wiki/media/17']);
        $rendered = (new CommonMarkWikiRenderer)->render(
            '![Most nad rzeka](wiki-media:17)',
            $context,
        );

        self::assertStringContainsString('<img', $rendered->html);
        self::assertStringContainsString('src="/en/wiki/media/17"', $rendered->html);
        self::assertStringContainsString('alt="Most nad rzeka"', $rendered->html);
        self::assertStringContainsString('loading="lazy"', $rendered->html);
        self::assertStringContainsString('decoding="async"', $rendered->html);
    }

    public function test_remote_malformed_unknown_and_empty_alt_targets_remain_inert(): void
    {
        $context = new WikiMediaRenderContext([17 => '/en/wiki/media/17']);
        $rendered = (new CommonMarkWikiRenderer)->render(<<<'MARKDOWN'
![Remote](https://images.example.test/tracker.png)
![Malformed](wiki-media:017)
![Unsupported](/storage/editorial-media/private.png)
![Unknown](wiki-media:18)
![](wiki-media:17)
![<b>Raw HTML</b>](wiki-media:17)
MARKDOWN, $context);

        self::assertStringNotContainsString('<img', $rendered->html);
        self::assertStringNotContainsString('tracker.png', $rendered->html);
        self::assertStringNotContainsString('/storage/editorial-media', $rendered->html);
        self::assertStringContainsString('Remote', $rendered->html);
        self::assertStringContainsString('Malformed', $rendered->html);
        self::assertStringContainsString('Unknown', $rendered->html);
    }

    public function test_reference_extraction_uses_parsed_commonmark_images_and_deduplicates_ids(): void
    {
        $ids = (new WikiMediaReferenceExtractor)->extractValidated(<<<'MARKDOWN'
![One](wiki-media:12)
![Again](wiki-media:12)
[A normal link](wiki-media:13)
![Two](wiki-media:2)
`![Inline code](wiki-media:14)`

```markdown
![Fenced code](wiki-media:15)
```
MARKDOWN);

        self::assertSame([2, 12], $ids);
    }

    public function test_validated_extraction_rejects_every_noncanonical_or_inaccessible_image(): void
    {
        $rejected = 0;

        foreach ([
            '![Remote](https://example.test/image.png)',
            '![Leading zero](wiki-media:01)',
            '![](wiki-media:1)',
            '![<b>Raw HTML</b>](wiki-media:1)',
        ] as $sourceMarkdown) {
            try {
                (new WikiMediaReferenceExtractor)->extractValidated($sourceMarkdown);
                self::fail("Expected invalid Wiki image markup to be rejected: {$sourceMarkdown}");
            } catch (InvalidWikiMediaSyntax) {
                $rejected++;
            }
        }

        self::assertSame(4, $rejected);
    }

    public function test_markdown_token_escapes_library_alt_text_and_rejects_empty_text(): void
    {
        self::assertSame(
            '![A \\[safe\\] \\\\ bridge](wiki-media:7)',
            WikiMediaSyntax::markdownToken(7, 'A [safe] \\ bridge'),
        );

        $this->expectException(InvalidArgumentException::class);
        WikiMediaSyntax::markdownToken(7, '   ');
    }
}
