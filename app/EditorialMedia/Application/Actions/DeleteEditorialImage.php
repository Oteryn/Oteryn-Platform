<?php

namespace App\EditorialMedia\Application\Actions;

use App\Audit\AdminAuditRecorder;
use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Filesystem\Filesystem;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Storage;
use Illuminate\Validation\ValidationException;
use RuntimeException;
use Throwable;

final class DeleteEditorialImage
{
    public function __construct(private readonly AdminAuditRecorder $audit) {}

    public function execute(Identity $actor, EditorialMedia $media): void
    {
        /** @var Filesystem|null $filesystem */
        $filesystem = null;
        /** @var array<string, string> $backups */
        $backups = [];
        /** @var list<string> $deletedPaths */
        $deletedPaths = [];

        try {
            DB::transaction(function () use (
                $actor,
                $media,
                &$filesystem,
                &$backups,
                &$deletedPaths,
            ): void {
                $lockedMedia = EditorialMedia::query()
                    ->lockForUpdate()
                    ->findOrFail($media->id);

                if ($lockedMedia->references()->exists()) {
                    throw ValidationException::withMessages([
                        'media' => 'Referenced media cannot be deleted. Remove every consumer reference first.',
                    ]);
                }

                if ($lockedMedia->disk !== 'editorial_media') {
                    throw new RuntimeException('Editorial image storage disk is invalid.');
                }

                $filesystem = Storage::disk($lockedMedia->disk);

                foreach ($this->objects($lockedMedia) as $object) {
                    if (! $filesystem->exists($object['path'])) {
                        throw new RuntimeException('Editorial image storage is incomplete; deletion was refused.');
                    }

                    $bytes = $filesystem->get($object['path']);

                    if (
                        strlen($bytes) !== $object['byte_size']
                        || ! hash_equals($object['sha256'], hash('sha256', $bytes))
                    ) {
                        throw new RuntimeException('Editorial image integrity verification failed; deletion was refused.');
                    }

                    $backups[$object['path']] = $bytes;
                }

                $this->audit->record(
                    $actor->id,
                    'editorial_media.deleted',
                    'editorial_media',
                    (string) $lockedMedia->id,
                    [
                        'mime_type' => $lockedMedia->mime_type,
                        'byte_size' => $lockedMedia->byte_size,
                        'width' => $lockedMedia->width,
                        'height' => $lockedMedia->height,
                        'sha256' => $lockedMedia->sha256,
                        'reference_count' => 0,
                    ],
                );

                foreach (array_reverse(array_keys($backups)) as $path) {
                    if (! $filesystem->delete($path)) {
                        throw new RuntimeException('Editorial image storage deletion failed.');
                    }

                    $deletedPaths[] = $path;
                }

                $deletedRecords = EditorialMedia::query()
                    ->whereKey($lockedMedia->id)
                    ->delete();

                if ($deletedRecords !== 1) {
                    throw new RuntimeException('Editorial media record deletion failed.');
                }
            });
        } catch (Throwable $exception) {
            if ($filesystem instanceof Filesystem && $deletedPaths !== []) {
                $this->restore($filesystem, $backups, $deletedPaths, $exception);
            }

            throw $exception;
        }
    }

    /**
     * @return list<array{path: string, byte_size: int, sha256: string}>
     */
    private function objects(EditorialMedia $media): array
    {
        if (
            ! str_starts_with($media->storage_path, 'originals/')
            || str_contains($media->storage_path, '..')
        ) {
            throw new RuntimeException('Editorial image storage path is invalid.');
        }

        $objects = [[
            'path' => $media->storage_path,
            'byte_size' => $media->byte_size,
            'sha256' => $media->sha256,
        ]];

        if ($media->thumbnail_path !== null) {
            if (
                ! str_starts_with($media->thumbnail_path, 'thumbnails/')
                || str_contains($media->thumbnail_path, '..')
            ) {
                throw new RuntimeException('Editorial image thumbnail path is invalid.');
            }

            if ($media->thumbnail_byte_size === null || $media->thumbnail_sha256 === null) {
                throw new RuntimeException('Editorial image thumbnail integrity metadata is incomplete.');
            }

            $objects[] = [
                'path' => $media->thumbnail_path,
                'byte_size' => $media->thumbnail_byte_size,
                'sha256' => $media->thumbnail_sha256,
            ];
        }

        return $objects;
    }

    /**
     * @param  array<string, string>  $backups
     * @param  list<string>  $deletedPaths
     */
    private function restore(
        Filesystem $filesystem,
        array $backups,
        array $deletedPaths,
        Throwable $originalException,
    ): void {
        foreach ($deletedPaths as $path) {
            $bytes = $backups[$path] ?? null;

            if (! is_string($bytes) || ! $filesystem->put($path, $bytes, ['visibility' => 'private'])) {
                throw new RuntimeException(
                    'Editorial image deletion failed and storage restoration was incomplete.',
                    0,
                    $originalException,
                );
            }

            $restoredBytes = $filesystem->get($path);

            if (
                strlen($restoredBytes) !== strlen($bytes)
                || ! hash_equals(hash('sha256', $bytes), hash('sha256', $restoredBytes))
            ) {
                throw new RuntimeException(
                    'Editorial image deletion failed and restored storage failed integrity verification.',
                    0,
                    $originalException,
                );
            }
        }
    }
}
