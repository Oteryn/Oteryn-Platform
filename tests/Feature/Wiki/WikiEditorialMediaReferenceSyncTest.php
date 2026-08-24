<?php

namespace Tests\Feature\Wiki;

use App\EditorialMedia\Application\Actions\DeleteEditorialImage;
use App\Wiki\Infrastructure\Models\WikiArticle;
use App\Wiki\Infrastructure\Models\WikiArticleTranslation;
use App\Wiki\Infrastructure\Models\WikiRevision;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Storage;
use Illuminate\Validation\ValidationException;

final class WikiEditorialMediaReferenceSyncTest extends WikiEditorialMediaTestCase
{
    public function test_wiki_editor_can_discover_and_insert_without_media_management_authority(): void
    {
        $actor = $this->createIdentity('wiki-media-editor@example.test');
        $this->grantWikiPermissions($actor, ['wiki.access', 'wiki.articles.manage']);
        $media = $this->uploadThroughAction($actor, 'approved.png');
        $this->actingAsCurrent($actor);

        $this->get(route('admin.media.index'))->assertForbidden();
        $library = $this->getJson(route('admin.wiki.media.index'));
        $library->assertOk()
            ->assertJsonPath('items.0.id', $media->id)
            ->assertJsonPath('items.0.markdown', "![Editorial media action fixture.](wiki-media:{$media->id})");
        $this->get(route('admin.wiki.media.thumbnail', $media))->assertOk();

        $this->post(route('admin.wiki.articles.store'), $this->wikiArticlePayload(
            "![An Oteryn bridge](wiki-media:{$media->id})",
            "![Most Oteryn](wiki-media:{$media->id})",
        ))->assertRedirect()->assertSessionHasNoErrors();

        $article = WikiArticle::query()->firstOrFail();
        $translations = WikiArticleTranslation::query()
            ->where('article_id', $article->id)
            ->get()
            ->keyBy('locale');
        $english = $translations->get('en');
        $polish = $translations->get('pl');
        self::assertInstanceOf(WikiArticleTranslation::class, $english);
        self::assertInstanceOf(WikiArticleTranslation::class, $polish);
        $this->assertDatabaseHas('editorial_media_references', [
            'media_id' => $media->id,
            'consumer' => 'wiki',
            'consumer_id' => "translation:{$english->id}",
            'usage' => "body.{$media->id}",
        ]);
        $this->assertDatabaseHas('editorial_media_references', [
            'media_id' => $media->id,
            'consumer' => 'wiki',
            'consumer_id' => "translation:{$polish->id}",
            'usage' => "body.{$media->id}",
        ]);
        $this->assertDatabaseCount('editorial_media_references', 2);

        try {
            app(DeleteEditorialImage::class)->execute($actor, $media);
            self::fail('A current Wiki reference must protect media from deletion.');
        } catch (ValidationException $exception) {
            self::assertArrayHasKey('media', $exception->errors());
        }

        $payload = $this->wikiArticlePayload(
            'No current English image.',
            "![Most Oteryn](wiki-media:{$media->id})",
        );
        $payload['lock_version'] = $article->lock_version;
        $this->put(route('admin.wiki.articles.update', $article), $payload)
            ->assertRedirect()
            ->assertSessionHasNoErrors();

        $this->assertDatabaseMissing('editorial_media_references', [
            'consumer_id' => "translation:{$english->id}",
            'usage' => "body.{$media->id}",
        ]);
        $this->assertDatabaseHas('editorial_media_references', [
            'consumer_id' => "translation:{$polish->id}",
            'usage' => "body.{$media->id}",
        ]);
        $this->assertDatabaseCount('editorial_media_references', 1);

        $article->refresh();
        $payload = $this->wikiArticlePayload('No current image.', 'Brak aktualnego obrazu.');
        $payload['lock_version'] = $article->lock_version;
        $this->put(route('admin.wiki.articles.update', $article), $payload)
            ->assertRedirect()
            ->assertSessionHasNoErrors();

        $this->assertDatabaseCount('editorial_media_references', 0);
        app(DeleteEditorialImage::class)->execute($actor, $media->refresh());
        $this->assertDatabaseMissing('editorial_media', ['id' => $media->id]);
    }

    public function test_wiki_thumbnail_distinguishes_missing_storage_from_integrity_failure(): void
    {
        $actor = $this->createIdentity('wiki-media-failure-states@example.test');
        $this->grantWikiPermissions($actor, ['wiki.access', 'wiki.articles.manage']);
        $missing = $this->uploadThroughAction($actor, 'missing.png');
        $corrupt = $this->uploadThroughAction($actor, 'corrupt.png');
        $this->actingAsCurrent($actor);

        self::assertIsString($missing->storage_path);
        Storage::disk($missing->disk)->delete($missing->storage_path);
        $this->get(route('admin.wiki.media.thumbnail', $missing))->assertNotFound();

        self::assertIsString($corrupt->storage_path);
        Storage::disk($corrupt->disk)->put($corrupt->storage_path, 'corrupt-wiki-thumbnail');
        $this->get(route('admin.wiki.media.thumbnail', $corrupt))->assertStatus(500);
    }

    public function test_unknown_media_rejects_and_rolls_back_the_complete_create_transaction(): void
    {
        $actor = $this->createIdentity('wiki-unknown-media@example.test');
        $this->grantWikiPermissions($actor, ['wiki.access', 'wiki.articles.manage']);
        $this->actingAsCurrent($actor);

        $this->from(route('admin.wiki.articles.create'))
            ->post(route('admin.wiki.articles.store'), $this->wikiArticlePayload(
                '![Unknown](wiki-media:987654321)',
                'Bez obrazu.',
            ))
            ->assertRedirect(route('admin.wiki.articles.create'))
            ->assertSessionHasErrors('translations.en.source_markdown');

        $this->assertDatabaseCount('wiki_articles', 0);
        $this->assertDatabaseCount('wiki_article_translations', 0);
        $this->assertDatabaseCount('wiki_revisions', 0);
        $this->assertDatabaseCount('editorial_media_references', 0);
        $this->assertDatabaseCount('admin_audit_events', 0);
    }

    public function test_noncanonical_or_inaccessible_image_markup_rejects_the_complete_write(): void
    {
        $actor = $this->createIdentity('wiki-invalid-media-markup@example.test');
        $this->grantWikiPermissions($actor, ['wiki.access', 'wiki.articles.manage']);
        $this->actingAsCurrent($actor);

        foreach ([
            ['![Remote](https://images.example.test/tracker.png)', 'translations.en.source_markdown'],
            ['![Leading zero](wiki-media:01)', 'translations.en.source_markdown'],
            ['![](wiki-media:1)', 'translations.en.source_markdown'],
            ['![<b>Raw HTML</b>](wiki-media:1)', 'translations.en'],
        ] as [$sourceMarkdown, $errorKey]) {
            $this->from(route('admin.wiki.articles.create'))
                ->post(route('admin.wiki.articles.store'), $this->wikiArticlePayload(
                    $sourceMarkdown,
                    'Bez obrazu.',
                ))
                ->assertRedirect(route('admin.wiki.articles.create'))
                ->assertSessionHasErrors($errorKey);
        }

        $this->assertDatabaseCount('wiki_articles', 0);
        $this->assertDatabaseCount('wiki_article_translations', 0);
        $this->assertDatabaseCount('wiki_revisions', 0);
        $this->assertDatabaseCount('editorial_media_references', 0);
        $this->assertDatabaseCount('admin_audit_events', 0);
    }

    public function test_unknown_media_update_preserves_current_content_references_revisions_and_audit(): void
    {
        $actor = $this->createIdentity('wiki-unknown-media-update@example.test');
        $this->grantWikiPermissions($actor, ['wiki.access', 'wiki.articles.manage']);
        $media = $this->uploadThroughAction($actor, 'current.png');
        $this->actingAsCurrent($actor);

        $currentSource = "![Current](wiki-media:{$media->id})";
        $this->post(route('admin.wiki.articles.store'), $this->wikiArticlePayload(
            $currentSource,
            'Bez obrazu.',
        ))->assertRedirect();
        $article = WikiArticle::query()->firstOrFail();
        $english = WikiArticleTranslation::query()
            ->where('article_id', $article->id)
            ->where('locale', 'en')
            ->firstOrFail();
        $revisionCount = WikiRevision::query()->where('article_id', $article->id)->count();
        $auditCount = DB::table('admin_audit_events')->count();

        $payload = $this->wikiArticlePayload(
            '![Missing](wiki-media:987654321)',
            'Bez obrazu.',
        );
        $payload['lock_version'] = $article->lock_version;
        $this->from(route('admin.wiki.articles.edit', $article))
            ->put(route('admin.wiki.articles.update', $article), $payload)
            ->assertRedirect(route('admin.wiki.articles.edit', $article))
            ->assertSessionHasErrors('translations.en.source_markdown');

        self::assertSame($currentSource, $english->refresh()->source_markdown);
        self::assertSame(
            $revisionCount,
            WikiRevision::query()->where('article_id', $article->id)->count(),
        );
        self::assertSame($auditCount, DB::table('admin_audit_events')->count());
        $this->assertDatabaseHas('editorial_media_references', [
            'media_id' => $media->id,
            'consumer_id' => "translation:{$english->id}",
            'usage' => "body.{$media->id}",
        ]);
        $this->assertDatabaseCount('editorial_media_references', 1);
    }

    public function test_stale_update_never_mutates_media_references(): void
    {
        $actor = $this->createIdentity('wiki-stale-media@example.test');
        $this->grantWikiPermissions($actor, ['wiki.access', 'wiki.articles.manage']);
        $first = $this->uploadThroughAction($actor, 'first.png');
        $second = $this->uploadThroughAction($actor, 'second.png');
        $this->actingAsCurrent($actor);

        $this->post(route('admin.wiki.articles.store'), $this->wikiArticlePayload(
            "![First](wiki-media:{$first->id})",
            'Bez obrazu.',
        ))->assertRedirect();
        $article = WikiArticle::query()->firstOrFail();
        $english = WikiArticleTranslation::query()
            ->where('article_id', $article->id)
            ->where('locale', 'en')
            ->firstOrFail();
        $staleVersion = $article->lock_version;

        DB::table('wiki_articles')->where('id', $article->id)->update([
            'lock_version' => $staleVersion + 1,
            'updated_at' => now(),
        ]);

        $payload = $this->wikiArticlePayload(
            "![Second](wiki-media:{$second->id})",
            'Bez obrazu.',
        );
        $payload['lock_version'] = $staleVersion;
        $this->put(route('admin.wiki.articles.update', $article), $payload)->assertStatus(409);

        $this->assertDatabaseHas('editorial_media_references', [
            'media_id' => $first->id,
            'consumer_id' => "translation:{$english->id}",
            'usage' => "body.{$first->id}",
        ]);
        $this->assertDatabaseMissing('editorial_media_references', [
            'media_id' => $second->id,
            'consumer_id' => "translation:{$english->id}",
        ]);
        $this->assertDatabaseCount('editorial_media_references', 1);
    }

    public function test_revision_restore_revalidates_and_transactionally_restores_current_references(): void
    {
        $actor = $this->createIdentity('wiki-restore-media@example.test');
        $this->grantWikiPermissions($actor, [
            'wiki.access',
            'wiki.articles.manage',
            'wiki.publish',
        ]);
        $media = $this->uploadThroughAction($actor, 'restore.png');
        $this->actingAsCurrent($actor);

        $this->post(route('admin.wiki.articles.store'), $this->wikiArticlePayload(
            "![Restored image](wiki-media:{$media->id})",
            'Bez obrazu.',
        ))->assertRedirect();
        $article = WikiArticle::query()->firstOrFail();
        $source = WikiRevision::query()
            ->where('article_id', $article->id)
            ->where('locale', 'en')
            ->firstOrFail();

        $withoutMedia = $this->wikiArticlePayload('Temporarily removed.', 'Bez obrazu.');
        $withoutMedia['lock_version'] = $article->lock_version;
        $this->put(route('admin.wiki.articles.update', $article), $withoutMedia)->assertRedirect();
        $article->refresh();
        $this->assertDatabaseCount('editorial_media_references', 0);

        $this->post(route('admin.wiki.articles.revisions.restore', [$article, $source]), [
            'lock_version' => $article->lock_version,
            'change_note' => 'Restore media reference.',
        ])->assertRedirect()->assertSessionHasNoErrors();

        $this->assertDatabaseHas('editorial_media_references', [
            'media_id' => $media->id,
            'consumer_id' => 'translation:'.WikiArticleTranslation::query()
                ->where('article_id', $article->id)
                ->where('locale', 'en')
                ->firstOrFail()
                ->id,
            'usage' => "body.{$media->id}",
        ]);
        $restored = WikiRevision::query()
            ->where('article_id', $article->id)
            ->where('locale', 'en')
            ->orderByDesc('revision_number')
            ->firstOrFail();
        self::assertSame($source->id, $restored->source_revision_id);
    }

    public function test_revision_restore_of_deleted_historical_media_rolls_back_atomically(): void
    {
        $actor = $this->createIdentity('wiki-restore-deleted-media@example.test');
        $this->grantWikiPermissions($actor, [
            'wiki.access',
            'wiki.articles.manage',
            'wiki.publish',
        ]);
        $media = $this->uploadThroughAction($actor, 'restore-deleted.png');
        $this->actingAsCurrent($actor);

        $this->post(route('admin.wiki.articles.store'), $this->wikiArticlePayload(
            "![Historical image](wiki-media:{$media->id})",
            'Bez obrazu.',
        ))->assertRedirect();
        $article = WikiArticle::query()->firstOrFail();
        $source = WikiRevision::query()
            ->where('article_id', $article->id)
            ->where('locale', 'en')
            ->firstOrFail();

        $withoutMedia = $this->wikiArticlePayload('Current content.', 'Bez obrazu.');
        $withoutMedia['lock_version'] = $article->lock_version;
        $this->put(route('admin.wiki.articles.update', $article), $withoutMedia)->assertRedirect();
        $article->refresh();
        app(DeleteEditorialImage::class)->execute($actor, $media->refresh());
        $revisionCount = WikiRevision::query()->where('article_id', $article->id)->count();

        $revisionsUrl = route('admin.wiki.articles.revisions', $article);
        $this->from($revisionsUrl)
            ->post(route('admin.wiki.articles.revisions.restore', [$article, $source]), [
                'lock_version' => $article->lock_version,
                'change_note' => 'Rejected historical media restore.',
            ])
            ->assertRedirect($revisionsUrl)
            ->assertSessionHasErrors('translations.en.source_markdown');

        self::assertSame(
            'Current content.',
            WikiArticleTranslation::query()
                ->where('article_id', $article->id)
                ->where('locale', 'en')
                ->firstOrFail()
                ->source_markdown,
        );
        self::assertSame(
            $revisionCount,
            WikiRevision::query()->where('article_id', $article->id)->count(),
        );
        $this->assertDatabaseCount('editorial_media_references', 0);
        $this->assertDatabaseMissing('editorial_media', ['id' => $media->id]);
    }
}
