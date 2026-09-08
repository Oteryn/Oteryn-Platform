<?php

namespace Tests\Feature\PublicPortal;

use App\Wiki\Queries\Public\PublicWikiQuery;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Mockery\MockInterface;
use RuntimeException;
use Tests\TestCase;

final class PublicSeoConditionalRequestTest extends TestCase
{
    use RefreshDatabase;

    public function test_robots_returns_content_etag_and_honors_if_none_match(): void
    {
        $response = $this->get('/robots.txt')->assertOk();
        $etag = (string) $response->headers->get('ETag');

        self::assertMatchesRegularExpression('/^"[0-9a-f]{64}"$/', $etag);
        self::assertStringContainsString('public', (string) $response->headers->get('Cache-Control'));
        self::assertStringContainsString('no-cache', (string) $response->headers->get('Cache-Control'));
        self::assertStringContainsString('must-revalidate', (string) $response->headers->get('Cache-Control'));

        $conditional = $this->withHeader('If-None-Match', $etag)->get('/robots.txt');

        $conditional->assertStatus(304)->assertHeader('ETag', $etag);
        self::assertSame('', $conditional->getContent());
    }

    public function test_sitemap_returns_content_etag_and_honors_if_none_match(): void
    {
        $response = $this->get('/sitemap.xml')->assertOk();
        $etag = (string) $response->headers->get('ETag');

        self::assertMatchesRegularExpression('/^"[0-9a-f]{64}"$/', $etag);
        self::assertStringContainsString('public', (string) $response->headers->get('Cache-Control'));
        self::assertStringContainsString('no-cache', (string) $response->headers->get('Cache-Control'));
        self::assertStringContainsString('must-revalidate', (string) $response->headers->get('Cache-Control'));

        $conditional = $this->withHeader('If-None-Match', $etag)->get('/sitemap.xml');

        $conditional->assertStatus(304)->assertHeader('ETag', $etag);
        self::assertSame('', $conditional->getContent());
    }

    public function test_sitemap_dependency_failure_remains_no_store_without_etag(): void
    {
        $this->mock(PublicWikiQuery::class, function (MockInterface $mock): void {
            $mock->shouldReceive('sitemapSlugs')->andThrow(new RuntimeException('dependency unavailable'));
        });

        $response = $this->get('/sitemap.xml')->assertStatus(503);

        self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));
        self::assertNull($response->headers->get('ETag'));
    }
}
