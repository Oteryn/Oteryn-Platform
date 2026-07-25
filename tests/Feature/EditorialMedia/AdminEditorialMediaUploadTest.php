<?php

namespace Tests\Feature\EditorialMedia;

use App\Admin\AdminRoleManager;
use App\EditorialMedia\Application\Actions\StoreEditorialImage;
use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Storage;
use LogicException;

final class AdminEditorialMediaUploadTest extends EditorialMediaTestCase
{
    public function test_editorial_media_disk_is_private_non_public_and_throws_on_storage_failure(): void
    {
        self::assertSame('editorial_media', config('editorial_media.disk'));
        self::assertSame(storage_path('app/editorial-media'), config('filesystems.disks.editorial_media.root'));
        self::assertSame('private', config('filesystems.disks.editorial_media.visibility'));
        self::assertFalse(config('filesystems.disks.editorial_media.serve'));
        self::assertTrue(config('filesystems.disks.editorial_media.throw'));
        self::assertTrue(config('filesystems.disks.editorial_media.report'));
        /** @var array<string, string> $links */
        $links = config('filesystems.links');
        self::assertNotContains(storage_path('app/editorial-media'), array_values($links));
    }

    public function test_authorized_editor_uploads_normalized_jpeg_with_metadata_removed_and_thumbnail(): void
    {
        $actor = $this->createIdentity('media-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $marker = 'OTERYN-SECRET-EXIF-MARKER';
        $jpeg = $this->imageBytes('jpeg', 400, 200);
        $jpeg = $this->withJpegMetadata($jpeg, $marker);

        $this->post(route('admin.media.store'), [
            'image' => $this->rawUpload('unsafe-original-name.jpeg', $jpeg, 'image/jpeg'),
            'alt_text' => 'A stone bridge leading into Oteryn.',
        ])->assertRedirect(route('admin.media.index'));

        $media = EditorialMedia::query()->firstOrFail();
        self::assertSame('image/jpeg', $media->mime_type);
        self::assertSame('jpg', $media->extension);
        self::assertSame(400, $media->width);
        self::assertSame(200, $media->height);
        self::assertSame(100, $media->thumbnail_width);
        self::assertSame(50, $media->thumbnail_height);
        $thumbnailPath = $media->thumbnail_path;
        self::assertNotNull($thumbnailPath);
        self::assertMatchesRegularExpression(
            '#^originals/[0-9a-f]{2}/[0-9a-f]{48}\.jpg$#',
            $media->storage_path,
        );
        self::assertStringNotContainsString('unsafe-original-name', $media->storage_path);

        $stored = $this->storedBytes($media->storage_path);
        self::assertSame(hash('sha256', $stored), $media->sha256);
        self::assertStringNotContainsString($marker, $stored);

        $decoded = getimagesizefromstring($stored);
        self::assertIsArray($decoded);
        self::assertSame(400, $decoded[0]);
        self::assertSame(200, $decoded[1]);
        self::assertSame('image/jpeg', $decoded['mime']);

        Storage::disk('editorial_media')->assertExists($media->storage_path);
        Storage::disk('editorial_media')->assertExists($thumbnailPath);
        self::assertSame('private', Storage::disk('editorial_media')->getVisibility($media->storage_path));
        self::assertSame('private', Storage::disk('editorial_media')->getVisibility($thumbnailPath));
        $storedThumbnail = $this->storedBytes($thumbnailPath);
        self::assertSame(hash('sha256', $storedThumbnail), $media->thumbnail_sha256);
        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $actor->id,
            'action' => 'editorial_media.uploaded',
            'target_type' => 'editorial_media',
            'target_id' => (string) $media->id,
        ]);
        $auditMetadata = DB::table('admin_audit_events')
            ->where('action', 'editorial_media.uploaded')
            ->where('target_id', (string) $media->id)
            ->value('metadata');
        self::assertIsString($auditMetadata);
        self::assertStringNotContainsString('A stone bridge leading into Oteryn.', $auditMetadata);
        self::assertStringNotContainsString('unsafe-original-name.jpeg', $auditMetadata);
        self::assertStringNotContainsString($marker, $auditMetadata);

        $this->get(route('admin.media.index'))
            ->assertOk()
            ->assertSeeText('A stone bridge leading into Oteryn.');
        $this->get(route('admin.media.content', $media))
            ->assertOk()
            ->assertHeader('Content-Type', 'image/jpeg')
            ->assertHeader('X-Content-Type-Options', 'nosniff');
    }

    public function test_png_text_metadata_is_removed_during_reencode(): void
    {
        $actor = $this->createIdentity('media-png-metadata-editor@example.com');
        $marker = 'OTERYN-PNG-METADATA-MARKER';
        $png = $this->withPngTextMetadata($this->imageBytes('png', 40, 30), $marker);

        $media = app(StoreEditorialImage::class)->execute(
            $actor,
            $this->rawUpload('metadata.png', $png, 'image/png'),
            'PNG metadata removal fixture.',
        );

        $stored = $this->storedBytes($media->storage_path);
        self::assertStringNotContainsString($marker, $stored);
        self::assertSame(hash('sha256', $stored), $media->sha256);
    }

    public function test_png_and_webp_are_accepted_without_unnecessary_small_thumbnails(): void
    {
        $actor = $this->createIdentity('media-raster-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        foreach (['png' => 'image/png', 'webp' => 'image/webp'] as $extension => $mimeType) {
            $this->post(route('admin.media.store'), [
                'image' => $this->rawUpload(
                    'small-'.$extension.'.'.$extension,
                    $this->imageBytes($extension, 40, 30),
                    $mimeType,
                ),
                'alt_text' => 'Small '.$extension.' editorial image.',
            ])->assertRedirect(route('admin.media.index'));
        }

        $items = EditorialMedia::query()->orderBy('id')->get();
        self::assertCount(2, $items);

        foreach ($items as $item) {
            self::assertNull($item->thumbnail_path);
            self::assertNull($item->thumbnail_sha256);
            self::assertNull($item->thumbnail_width);
            self::assertNull($item->thumbnail_height);
            Storage::disk('editorial_media')->assertExists($item->storage_path);
        }
    }

    public function test_storage_and_integrity_fields_are_immutable_after_creation(): void
    {
        $actor = $this->createIdentity('media-immutable-editor@example.com');
        $media = $this->uploadThroughAction($actor, 'immutable.png');
        $originalPath = $media->storage_path;
        $media->storage_path = 'originals/changed.png';

        try {
            $media->save();
            self::fail('Editorial media storage paths must remain immutable.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('immutable', $exception->getMessage());
        }

        self::assertSame($originalPath, $media->fresh()?->storage_path);
        Storage::disk('editorial_media')->assertExists($originalPath);

        try {
            $media->refresh()->delete();
            self::fail('Direct model deletion must be rejected.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('safe deletion action', $exception->getMessage());
        }

        $this->assertDatabaseHas('editorial_media', ['id' => $media->id]);
        Storage::disk('editorial_media')->assertExists($originalPath);
    }

    public function test_disallowed_malformed_script_document_archive_executable_and_mismatched_fixtures_are_rejected(): void
    {
        $actor = $this->createIdentity('media-fixture-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $fixtures = [
            ['vector.svg', '<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script></svg>'],
            ['page.html', '<!doctype html><script>alert(1)</script>'],
            ['script.jpg', '<?php echo "executed";'],
            ['archive.png', "PK\x03\x04not-an-image"],
            ['program.webp', "MZ\x90\x00not-an-image"],
            ['document.png', "%PDF-1.7\nnot-an-image"],
            ['malformed.png', random_bytes(128)],
            ['polyglot.jpg', $this->imageBytes('jpeg', 20, 20).'<?php echo "executed";'],
            ['polyglot.png', $this->imageBytes('png', 20, 20).'<script>alert(1)</script>'],
            ['mismatch.jpg', $this->imageBytes('png', 20, 20)],
        ];

        foreach ($fixtures as [$name, $bytes]) {
            $this->post(route('admin.media.store'), [
                'image' => $this->rawUpload($name, $bytes),
                'alt_text' => 'Rejected fixture.',
            ])->assertSessionHasErrors('image');
        }

        $this->assertDatabaseCount('editorial_media', 0);
        self::assertSame([], Storage::disk('editorial_media')->allFiles());
    }

    public function test_byte_dimension_and_pixel_limits_are_enforced_before_persistence(): void
    {
        $actor = $this->createIdentity('media-limit-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        config(['editorial_media.max_bytes' => 16]);
        $this->post(route('admin.media.store'), [
            'image' => $this->rawUpload('too-large.png', $this->imageBytes('png', 20, 20), 'image/png'),
            'alt_text' => 'Too large.',
        ])->assertSessionHasErrors('image');

        config([
            'editorial_media.max_bytes' => 8 * 1024 * 1024,
            'editorial_media.max_width' => 10,
        ]);
        $this->post(route('admin.media.store'), [
            'image' => $this->rawUpload('too-wide.png', $this->imageBytes('png', 20, 5), 'image/png'),
            'alt_text' => 'Too wide.',
        ])->assertSessionHasErrors('image');

        config([
            'editorial_media.max_width' => 5000,
            'editorial_media.max_pixels' => 100,
        ]);
        $this->post(route('admin.media.store'), [
            'image' => $this->rawUpload('too-many-pixels.png', $this->imageBytes('png', 20, 20), 'image/png'),
            'alt_text' => 'Too many pixels.',
        ])->assertSessionHasErrors('image');

        $this->assertDatabaseCount('editorial_media', 0);
    }

    public function test_alt_text_is_required_and_bounded(): void
    {
        $actor = $this->createIdentity('media-alt-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $this->post(route('admin.media.store'), [
            'image' => $this->rawUpload('missing-alt.png', $this->imageBytes('png', 20, 20), 'image/png'),
            'alt_text' => '',
        ])->assertSessionHasErrors('alt_text');

        $this->post(route('admin.media.store'), [
            'image' => $this->rawUpload('long-alt.png', $this->imageBytes('png', 20, 20), 'image/png'),
            'alt_text' => str_repeat('a', 501),
        ])->assertSessionHasErrors('alt_text');

        $this->assertDatabaseCount('editorial_media', 0);
    }
}
