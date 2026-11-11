<?php

namespace Tests\Unit\Wiki;

use App\Wiki\Infrastructure\Rendering\CommonMarkWikiRenderer;
use PHPUnit\Framework\TestCase;

final class CommonMarkWikiRendererTest extends TestCase
{
    public function test_it_strips_raw_html_and_never_renders_active_embeds_or_images(): void
    {
        $rendered = (new CommonMarkWikiRenderer)->render(<<<'MD'
<script>alert("xss")</script>

<iframe src="https://example.test"></iframe>

<form action="/steal"><input name="secret"></form>

![Remote image](https://images.example.test/tracker.png)
MD);

        self::assertStringNotContainsString('<script', $rendered->html);
        self::assertStringNotContainsString('<iframe', $rendered->html);
        self::assertStringNotContainsString('<form', $rendered->html);
        self::assertStringNotContainsString('<input', $rendered->html);
        self::assertStringNotContainsString('<img', $rendered->html);
        self::assertStringNotContainsString('tracker.png', $rendered->html);
        self::assertStringContainsString('Remote image', $rendered->html);
    }

    public function test_it_allows_only_fragments_internal_paths_and_https_links(): void
    {
        $rendered = (new CommonMarkWikiRenderer)->render(<<<'MD'
[section](#safe-heading)
[internal](/en/wiki/start)
[external](https://example.test/guide)
[http](http://example.test)
[script](javascript:alert(1))
[protocol relative](//example.test/path)
[credentials](https://user:pass@example.test/path)
MD);

        self::assertStringContainsString('href="#safe-heading"', $rendered->html);
        self::assertStringContainsString('href="/en/wiki/start"', $rendered->html);
        self::assertStringContainsString('href="https://example.test/guide"', $rendered->html);
        self::assertStringContainsString('rel="noopener noreferrer"', $rendered->html);
        self::assertStringNotContainsString('href="http://', $rendered->html);
        self::assertStringNotContainsString('javascript:', $rendered->html);
        self::assertStringNotContainsString('href="//', $rendered->html);
        self::assertStringNotContainsString('user:pass', $rendered->html);
    }

    public function test_heading_ids_and_table_of_contents_are_deterministic_and_collision_safe(): void
    {
        $source = <<<'MD'
# First steps
## First steps
### **First** steps
# !!!
MD;
        $first = (new CommonMarkWikiRenderer)->render($source);
        $second = (new CommonMarkWikiRenderer)->render($source);

        self::assertSame($first->html, $second->html);
        self::assertSame(
            ['first-steps', 'first-steps-2', 'first-steps-3', 'section'],
            array_map(static fn ($item): string => $item->id, $first->tableOfContents),
        );
        self::assertSame(
            [2, 3, 4, 2],
            array_map(static fn ($item): int => $item->level, $first->tableOfContents),
        );
        self::assertStringContainsString('id="first-steps-3"', $first->html);
    }

    public function test_malformed_and_deep_markdown_is_bounded_and_renders_without_trusting_html(): void
    {
        $source = str_repeat('> ', 60).'bounded'
            ."\n\n[unfinished](https://example.test\n\n"
            .str_repeat('*', 500);

        $rendered = (new CommonMarkWikiRenderer)->render($source);

        self::assertNotSame('', trim($rendered->html));
        self::assertStringNotContainsString('<script', $rendered->html);
        self::assertLessThan(10_000, strlen($rendered->html));
    }
}
