<?php

namespace Tests\Feature\Wiki;

use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use App\Identity\Models\Identity;
use App\Wiki\Infrastructure\Models\WikiArticle;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Storage;
use Illuminate\Support\Facades\URL;

final class WikiEditorialMediaServingTest extends WikiEditorialMediaTestCase
{
    public function test_published_effective_translation_serves_verified_bytes_with_revalidating_cache(): void
    {
        [$actor, $article, $media] = $this->publishedArticle();
        auth()->logout();

        $articlePage = $this->get(route('wiki.article', [
            'locale' => 'en',
            'slug' => 'media-guide',
        ]));
        $articlePage->assertOk()
            ->assertSee('class="wiki-editorial-image"', false)
            ->assertSee('alt="An Oteryn bridge"', false)
            ->assertSee(route('wiki.media', ['locale' => 'en', 'editorialMedia' => $media]), false);

        $url = route('wiki.media', ['locale' => 'en', 'editorialMedia' => $media]);
        $response = $this->get($url);
        $response->assertOk()
            ->assertHeader('Content-Type', 'image/png')
            ->assertHeader('X-Content-Type-Options', 'nosniff')
            ->assertHeader('Cross-Origin-Resource-Policy', 'same-origin');
        self::assertStringContainsString('public', (string) $response->headers->get('Cache-Control'));
        self::assertStringContainsString('no-cache', (string) $response->headers->get('Cache-Control'));
        self::assertStringContainsString('max-age=0', (string) $response->headers->get('Cache-Control'));
        self::assertStringContainsString('must-revalidate', (string) $response->headers->get('Cache-Control'));
        self::assertSame($this->storedBytes($media->storage_path), $response->getContent());

        $etag = $response->headers->get('ETag');
        self::assertIsString($etag);
        $this->withHeader('If-None-Match', $etag)->get($url)->assertStatus(304);

        DB::table('wiki_article_translations')
            ->where('article_id', $article->id)
            ->where('locale', 'en')
            ->update([
                'source_markdown' => 'The current source no longer contains the image.',
                'updated_at' => now(),
            ]);
        $this->withHeader('If-None-Match', $etag)->get($url)->assertNotFound();

        $this->actingAsCurrent($actor);
        $this->post(route('admin.wiki.articles.unpublish', $article), [
            'lock_version' => $article->lock_version,
        ])->assertRedirect();
        $this->get($url)->assertNotFound();
    }

    public function test_draft_review_archive_future_missing_and_stale_locales_cannot_authorize_public_bytes(): void
    {
        [$actor, $article, $media] = $this->publishedArticle();
        $this->actingAsCurrent($actor);
        $englishUrl = route('wiki.media', ['locale' => 'en', 'editorialMedia' => $media]);
        $polishUrl = route('wiki.media', ['locale' => 'pl', 'editorialMedia' => $media]);

        $this->get($englishUrl)->assertOk();
        $this->get($polishUrl)->assertOk();

        foreach (['draft', 'in_review', 'archived'] as $status) {
            DB::table('wiki_articles')->where('id', $article->id)->update([
                'status' => $status,
                'published_at' => $status === 'draft' ? null : now()->subMinute(),
            ]);
            $this->get($englishUrl)->assertNotFound();
        }

        DB::table('wiki_articles')->where('id', $article->id)->update([
            'status' => 'published',
            'published_at' => now()->addHour(),
        ]);
        $this->get($englishUrl)->assertNotFound();

        DB::table('wiki_articles')->where('id', $article->id)->update([
            'published_at' => now()->subMinute(),
        ]);
        DB::table('wiki_article_translations')
            ->where('article_id', $article->id)
            ->where('locale', 'pl')
            ->update(['updated_at' => now()->subHour()]);
        DB::table('wiki_article_translations')
            ->where('article_id', $article->id)
            ->where('locale', 'en')
            ->update(['updated_at' => now()]);
        $this->get($englishUrl)->assertOk();
        $this->get($polishUrl)->assertNotFound();

        DB::table('wiki_article_translations')
            ->where('article_id', $article->id)
            ->where('locale', 'pl')
            ->delete();
        $this->get($polishUrl)->assertNotFound();
    }

    public function test_storage_loss_or_integrity_mismatch_is_unavailable_without_private_path_disclosure(): void
    {
        [, , $media] = $this->publishedArticle();
        $url = route('wiki.media', ['locale' => 'en', 'editorialMedia' => $media]);
        $original = $this->storedBytes($media->storage_path);

        DB::table('editorial_media')->where('id', $media->id)->update([
            'mime_type' => 'image/jpeg',
        ]);
        $unsupported = $this->get($url);
        $unsupported->assertStatus(503)->assertDontSee($media->storage_path);
        DB::table('editorial_media')->where('id', $media->id)->update([
            'mime_type' => 'image/png',
        ]);

        Storage::disk('editorial_media')->put($media->storage_path, 'corrupt');
        $corrupt = $this->get($url);
        $corrupt->assertStatus(503)->assertDontSee($media->storage_path);

        Storage::disk('editorial_media')->put($media->storage_path, $original);
        Storage::disk('editorial_media')->delete($media->storage_path);
        $missing = $this->get($url);
        $missing->assertNotFound()->assertDontSee($media->storage_path);
    }

    public function test_signed_authenticated_preview_media_has_no_anonymous_or_unsigned_draft_route(): void
    {
        $actor = $this->createIdentity('wiki-preview-media@example.test');
        $this->grantWikiPermissions($actor, ['wiki.access', 'wiki.articles.manage']);
        $media = $this->uploadThroughAction($actor, 'preview.png');
        $this->actingAsCurrent($actor);
        $this->post(route('admin.wiki.articles.store'), $this->wikiArticlePayload(
            "![Draft bridge](wiki-media:{$media->id})",
            'Bez obrazu.',
        ))->assertRedirect();
        $article = WikiArticle::query()->firstOrFail();
        $translation = DB::table('wiki_article_translations')
            ->where('article_id', $article->id)
            ->where('locale', 'en')
            ->first();
        self::assertNotNull($translation);
        $polishTranslation = DB::table('wiki_article_translations')
            ->where('article_id', $article->id)
            ->where('locale', 'pl')
            ->first();
        self::assertNotNull($polishTranslation);

        $previewUrl = URL::temporarySignedRoute(
            'admin.wiki.articles.preview',
            now()->addMinutes(5),
            ['article' => $article, 'locale' => 'en'],
        );
        $preview = $this->get($previewUrl);
        $preview->assertOk()->assertSee('class="wiki-editorial-image"', false);
        $content = $preview->getContent();
        self::assertIsString($content);
        if (preg_match('/<img[^>]+src="([^"]+preview-media[^"]+)"/', $content, $matches) !== 1) {
            self::fail('Expected the signed preview page to contain a signed media URL.');
        }

        $signedMediaUrl = html_entity_decode($matches[1], ENT_QUOTES | ENT_HTML5, 'UTF-8');
        $this->get($signedMediaUrl)->assertOk();

        $mismatchedTranslationUrl = URL::temporarySignedRoute(
            'admin.wiki.media.preview',
            now()->addMinutes(5),
            [
                'article' => $article,
                'locale' => 'en',
                'translation' => $polishTranslation->id,
                'editorialMedia' => $media,
            ],
        );
        $this->get($mismatchedTranslationUrl)->assertNotFound();

        $this->get(route('admin.wiki.media.preview', [
            'article' => $article,
            'locale' => 'en',
            'translation' => $translation->id,
            'editorialMedia' => $media,
        ]))->assertForbidden();

        auth()->logout();
        $this->get($signedMediaUrl)->assertRedirect(route('identity.login.create'));
    }

    /**
     * @return array{0: Identity, 1: WikiArticle, 2: EditorialMedia}
     */
    private function publishedArticle(): array
    {
        $actor = $this->createIdentity('wiki-published-media-'.bin2hex(random_bytes(4)).'@example.test');
        $this->grantWikiPermissions($actor, [
            'wiki.access',
            'wiki.articles.manage',
            'wiki.publish',
        ]);
        $media = $this->uploadThroughAction($actor, 'published.png');
        $this->actingAsCurrent($actor);
        $this->post(route('admin.wiki.articles.store'), $this->wikiArticlePayload(
            "![An Oteryn bridge](wiki-media:{$media->id})",
            "![Most Oteryn](wiki-media:{$media->id})",
        ))->assertRedirect();
        $article = WikiArticle::query()->firstOrFail();
        $this->post(route('admin.wiki.articles.submit-review', $article), [
            'lock_version' => $article->lock_version,
        ])->assertRedirect();
        $article->refresh();
        $this->post(route('admin.wiki.articles.publish', $article), [
            'lock_version' => $article->lock_version,
        ])->assertRedirect();

        return [$actor, $article->refresh(), $media];
    }
}
