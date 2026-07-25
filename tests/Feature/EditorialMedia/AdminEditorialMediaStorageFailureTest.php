<?php

namespace Tests\Feature\EditorialMedia;

use App\EditorialMedia\Application\Actions\DeleteEditorialImage;
use App\EditorialMedia\Application\Actions\StoreEditorialImage;
use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use Illuminate\Contracts\Filesystem\Filesystem;
use Illuminate\Support\Facades\Storage;
use RuntimeException;

final class AdminEditorialMediaStorageFailureTest extends EditorialMediaTestCase
{
    public function test_upload_storage_failure_leaves_database_state_unchanged(): void
    {
        $actor = $this->createIdentity('media-upload-storage-editor@example.com');

        Storage::shouldReceive('disk')
            ->once()
            ->with('editorial_media')
            ->andThrow(new RuntimeException('storage unavailable'));

        try {
            app(StoreEditorialImage::class)->execute(
                $actor,
                $this->rawUpload('storage-down.png', $this->imageBytes('png', 20, 20), 'image/png'),
                'Storage failure fixture.',
            );
            self::fail('Storage failure must abort upload persistence.');
        } catch (RuntimeException $exception) {
            self::assertSame('storage unavailable', $exception->getMessage());
        }

        $this->assertDatabaseCount('editorial_media', 0);
    }

    public function test_partial_storage_delete_failure_restores_removed_objects_and_rolls_back_database_changes(): void
    {
        $actor = $this->createIdentity('media-partial-delete-editor@example.com');
        $originalBytes = 'normalized-original';
        $thumbnailBytes = 'normalized-thumbnail';
        $media = EditorialMedia::query()->create([
            'disk' => 'editorial_media',
            'storage_path' => 'originals/aa/'.str_repeat('a', 48).'.png',
            'thumbnail_path' => 'thumbnails/aa/'.str_repeat('a', 48).'.png',
            'original_name' => 'partial-delete.png',
            'mime_type' => 'image/png',
            'extension' => 'png',
            'byte_size' => strlen($originalBytes),
            'width' => 400,
            'height' => 200,
            'thumbnail_byte_size' => strlen($thumbnailBytes),
            'thumbnail_sha256' => hash('sha256', $thumbnailBytes),
            'thumbnail_width' => 100,
            'thumbnail_height' => 50,
            'sha256' => hash('sha256', $originalBytes),
            'alt_text' => 'Partial delete failure fixture.',
            'uploaded_by_identity_id' => $actor->id,
        ]);

        $filesystem = $this->createMock(Filesystem::class);
        $filesystem->expects($this->exactly(2))
            ->method('exists')
            ->willReturn(true);
        $filesystem->expects($this->exactly(3))
            ->method('get')
            ->willReturnCallback(static function (string $path) use ($media, $originalBytes, $thumbnailBytes): string {
                return match ($path) {
                    $media->storage_path => $originalBytes,
                    $media->thumbnail_path => $thumbnailBytes,
                    default => throw new RuntimeException('Unexpected storage read in partial deletion fixture.'),
                };
            });

        $deleteCall = 0;
        $filesystem->expects($this->exactly(2))
            ->method('delete')
            ->willReturnCallback(function (string $path) use ($media, &$deleteCall): bool {
                $deleteCall++;
                self::assertSame(
                    $deleteCall === 1 ? $media->thumbnail_path : $media->storage_path,
                    $path,
                );

                return $deleteCall === 1;
            });
        $filesystem->expects($this->once())
            ->method('put')
            ->with($media->thumbnail_path, $thumbnailBytes, ['visibility' => 'private'])
            ->willReturn(true);

        Storage::shouldReceive('disk')
            ->once()
            ->with('editorial_media')
            ->andReturn($filesystem);

        try {
            app(DeleteEditorialImage::class)->execute($actor, $media);
            self::fail('A partial storage deletion failure must roll back and restore removed objects.');
        } catch (RuntimeException $exception) {
            self::assertStringContainsString('storage deletion failed', $exception->getMessage());
        }

        $this->assertDatabaseHas('editorial_media', ['id' => $media->id]);
        $this->assertDatabaseMissing('admin_audit_events', [
            'action' => 'editorial_media.deleted',
            'target_id' => (string) $media->id,
        ]);
    }

    public function test_delete_storage_failure_leaves_database_and_audit_state_unchanged(): void
    {
        $actor = $this->createIdentity('media-delete-storage-editor@example.com');
        $media = $this->uploadThroughAction($actor, 'missing-on-delete.png');
        Storage::disk('editorial_media')->delete($media->storage_path);

        try {
            app(DeleteEditorialImage::class)->execute($actor, $media);
            self::fail('Missing storage must abort record deletion.');
        } catch (RuntimeException $exception) {
            self::assertStringContainsString('deletion was refused', $exception->getMessage());
        }

        $this->assertDatabaseHas('editorial_media', ['id' => $media->id]);
        $this->assertDatabaseMissing('admin_audit_events', [
            'action' => 'editorial_media.deleted',
            'target_id' => (string) $media->id,
        ]);
    }
}
