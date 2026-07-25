<?php

namespace Tests\Feature\EditorialMedia;

use App\Admin\AdminRoleManager;
use App\EditorialMedia\Application\Actions\DeleteEditorialImage;
use App\EditorialMedia\Application\Actions\StoreEditorialImage;
use App\EditorialMedia\Application\EditorialMediaReferenceManager;
use App\EditorialMedia\Domain\EditorialMediaConsumer;
use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use GdImage;
use Illuminate\Foundation\Http\Middleware\ValidateCsrfToken;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Http\UploadedFile;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Storage;
use LogicException;
use RuntimeException;
use Tests\TestCase;

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

        $filesystem = \Mockery::mock(\Illuminate\Contracts\Filesystem\Filesystem::class);
        $filesystem->shouldReceive('exists')->twice()->andReturnTrue();
        $filesystem->shouldReceive('get')->with($media->storage_path)->once()->andReturn($originalBytes);
        $filesystem->shouldReceive('get')->with($media->thumbnail_path)->twice()->andReturn($thumbnailBytes);
        $filesystem->shouldReceive('delete')->with($media->thumbnail_path)->once()->ordered()->andReturnTrue();
        $filesystem->shouldReceive('delete')->with($media->storage_path)->once()->ordered()->andReturnFalse();
        $filesystem->shouldReceive('put')
            ->with($media->thumbnail_path, $thumbnailBytes, ['visibility' => 'private'])
            ->once()
            ->andReturnTrue();

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
