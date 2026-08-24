<?php

namespace App\EditorialMedia\Application;

use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use Illuminate\Http\Request;
use Illuminate\Http\Response;
use Illuminate\Support\Facades\Storage;
use Symfony\Component\HttpKernel\Exception\HttpExceptionInterface;
use Throwable;

final class WikiEditorialMediaFileResponse
{
    public function public(Request $request, EditorialMedia $media): Response
    {
        $verified = $this->verified($media, false);
        $response = response($verified['bytes'], 200, $this->headers($media, $verified));
        $response->setEtag($verified['sha256']);
        $response->setLastModified($media->created_at);
        $response->setPublic();
        $response->setMaxAge(0);
        $response->headers->addCacheControlDirective('no-cache');
        $response->headers->addCacheControlDirective('must-revalidate');
        $response->isNotModified($request);

        return $response;
    }

    public function private(EditorialMedia $media, bool $thumbnail): Response
    {
        $verified = $this->verified($media, $thumbnail);

        return response($verified['bytes'], 200, [
            ...$this->headers($media, $verified),
            'Cache-Control' => 'private, no-store',
        ]);
    }

    /**
     * @return array{bytes: string, path: string, byte_size: int, sha256: string}
     */
    private function verified(EditorialMedia $media, bool $thumbnail): array
    {
        if ($media->disk !== 'editorial_media') {
            throw new WikiEditorialMediaUnavailable('Editorial image storage disk is unavailable.');
        }

        $expectedMimeType = match ($media->extension) {
            'jpg' => 'image/jpeg',
            'png' => 'image/png',
            'webp' => 'image/webp',
            default => null,
        };

        if ($expectedMimeType === null || $media->mime_type !== $expectedMimeType) {
            throw new WikiEditorialMediaUnavailable('Editorial image format metadata is unavailable.');
        }

        $path = $media->storage_path;
        $expectedByteSize = $media->byte_size;
        $expectedSha256 = $media->sha256;
        $directory = 'originals';

        if ($thumbnail && $media->thumbnail_path !== null) {
            $path = $media->thumbnail_path;
            $expectedByteSize = $media->thumbnail_byte_size;
            $expectedSha256 = $media->thumbnail_sha256;
            $directory = 'thumbnails';
        }

        if (
            ! is_int($expectedByteSize)
            || $expectedByteSize < 1
            || ! is_string($expectedSha256)
            || preg_match('/\A[0-9a-f]{64}\z/D', $expectedSha256) !== 1
            || preg_match(
                sprintf(
                    '#\A%s/[0-9a-f]{2}/[0-9a-f]{48}\.%s\z#D',
                    $directory,
                    preg_quote($media->extension, '#'),
                ),
                $path,
            ) !== 1
        ) {
            throw new WikiEditorialMediaUnavailable('Editorial image integrity metadata is unavailable.');
        }

        try {
            $filesystem = Storage::disk($media->disk);
            if (! $filesystem->exists($path)) {
                abort(404);
            }

            $bytes = $filesystem->get($path);
        } catch (WikiEditorialMediaUnavailable $exception) {
            throw $exception;
        } catch (HttpExceptionInterface $exception) {
            throw $exception;
        } catch (Throwable $exception) {
            throw new WikiEditorialMediaUnavailable(
                'Editorial image storage is unavailable.',
                0,
                $exception,
            );
        }

        if (
            ! is_string($bytes)
            || strlen($bytes) !== $expectedByteSize
            || ! hash_equals($expectedSha256, hash('sha256', $bytes))
        ) {
            throw new WikiEditorialMediaUnavailable('Editorial image integrity verification failed.');
        }

        return [
            'bytes' => $bytes,
            'path' => $path,
            'byte_size' => $expectedByteSize,
            'sha256' => $expectedSha256,
        ];
    }

    /**
     * @param  array{bytes: string, path: string, byte_size: int, sha256: string}  $verified
     * @return array<string, string>
     */
    private function headers(EditorialMedia $media, array $verified): array
    {
        return [
            'Content-Type' => $media->mime_type,
            'Content-Length' => (string) $verified['byte_size'],
            'Content-Disposition' => 'inline; filename="'.basename($verified['path']).'"',
            'X-Content-Type-Options' => 'nosniff',
            'Cross-Origin-Resource-Policy' => 'same-origin',
        ];
    }
}
