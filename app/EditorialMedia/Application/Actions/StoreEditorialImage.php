<?php

namespace App\EditorialMedia\Application\Actions;

use App\Audit\AdminAuditRecorder;
use App\EditorialMedia\Application\EditorialImageProcessor;
use App\EditorialMedia\Application\ProcessedEditorialImage;
use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Filesystem\Filesystem;
use Illuminate\Http\UploadedFile;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Storage;
use Illuminate\Validation\ValidationException;
use RuntimeException;
use Throwable;

final class StoreEditorialImage
{
    public function __construct(
        private readonly EditorialImageProcessor $processor,
        private readonly AdminAuditRecorder $audit,
    ) {}

    public function execute(Identity $actor, UploadedFile $file, string $altText): EditorialMedia
    {
        $altText = trim($altText);

        if ($altText === '' || mb_strlen($altText) > 500) {
            throw ValidationException::withMessages([
                'alt_text' => 'Alternative text is required and may not exceed 500 characters.',
            ]);
        }

        $processed = $this->processor->process($file);
        $disk = $this->diskName();
        $filesystem = Storage::disk($disk);
        [$storagePath, $thumbnailPath] = $this->allocatePaths($filesystem, $processed);
        $writtenPaths = [];

        try {
            if (! $filesystem->put($storagePath, $processed->bytes, ['visibility' => 'private'])) {
                throw new RuntimeException('Editorial image storage failed.');
            }

            $writtenPaths[] = $storagePath;

            if ($processed->thumbnailBytes !== null && $thumbnailPath !== null) {
                if (! $filesystem->put($thumbnailPath, $processed->thumbnailBytes, ['visibility' => 'private'])) {
                    throw new RuntimeException('Editorial image thumbnail storage failed.');
                }

                $writtenPaths[] = $thumbnailPath;
            }

            return DB::transaction(function () use (
                $actor,
                $file,
                $altText,
                $disk,
                $storagePath,
                $thumbnailPath,
                $processed,
            ): EditorialMedia {
                $media = EditorialMedia::query()->create([
                    'disk' => $disk,
                    'storage_path' => $storagePath,
                    'thumbnail_path' => $thumbnailPath,
                    'original_name' => $this->safeOriginalName($file->getClientOriginalName()),
                    'mime_type' => $processed->mimeType,
                    'extension' => $processed->extension,
                    'byte_size' => strlen($processed->bytes),
                    'width' => $processed->width,
                    'height' => $processed->height,
                    'thumbnail_byte_size' => $processed->thumbnailBytes === null
                        ? null
                        : strlen($processed->thumbnailBytes),
                    'thumbnail_sha256' => $processed->thumbnailSha256,
                    'thumbnail_width' => $processed->thumbnailWidth,
                    'thumbnail_height' => $processed->thumbnailHeight,
                    'sha256' => $processed->sha256,
                    'alt_text' => $altText,
                    'uploaded_by_identity_id' => $actor->id,
                ]);

                $this->audit->record(
                    $actor->id,
                    'editorial_media.uploaded',
                    'editorial_media',
                    (string) $media->id,
                    [
                        'mime_type' => $media->mime_type,
                        'byte_size' => $media->byte_size,
                        'width' => $media->width,
                        'height' => $media->height,
                        'sha256' => $media->sha256,
                        'thumbnail_generated' => $media->thumbnail_path !== null,
                    ],
                );

                return $media;
            }, 3);
        } catch (Throwable $exception) {
            $this->cleanup($filesystem, $writtenPaths);

            throw $exception;
        }
    }

    /**
     * @return array{0: string, 1: string|null}
     */
    private function allocatePaths(Filesystem $filesystem, ProcessedEditorialImage $processed): array
    {
        for ($attempt = 0; $attempt < 5; $attempt++) {
            $token = bin2hex(random_bytes(24));
            $prefix = substr($token, 0, 2);
            $storagePath = sprintf('originals/%s/%s.%s', $prefix, $token, $processed->extension);
            $thumbnailPath = $processed->thumbnailBytes === null
                ? null
                : sprintf('thumbnails/%s/%s.%s', $prefix, $token, $processed->extension);

            if (
                ! $filesystem->exists($storagePath)
                && ($thumbnailPath === null || ! $filesystem->exists($thumbnailPath))
            ) {
                return [$storagePath, $thumbnailPath];
            }
        }

        throw new RuntimeException('Could not allocate an immutable editorial image storage name.');
    }

    /**
     * @param  list<string>  $paths
     */
    private function cleanup(Filesystem $filesystem, array $paths): void
    {
        foreach (array_reverse($paths) as $path) {
            try {
                if ($filesystem->exists($path)) {
                    $filesystem->delete($path);
                }
            } catch (Throwable) {
                // The original exception remains authoritative; unreachable private objects are not exposed.
            }
        }
    }

    private function safeOriginalName(string $originalName): ?string
    {
        $basename = basename(str_replace('\\', '/', $originalName));
        $safeName = preg_replace('/[^\x20-\x7E]/', '_', $basename);

        if (! is_string($safeName)) {
            return null;
        }

        $safeName = trim($safeName);

        return $safeName === '' ? null : substr($safeName, 0, 255);
    }

    private function diskName(): string
    {
        $disk = config('editorial_media.disk');

        if ($disk !== 'editorial_media') {
            throw new RuntimeException('Editorial image storage configuration is invalid.');
        }

        return $disk;
    }
}
