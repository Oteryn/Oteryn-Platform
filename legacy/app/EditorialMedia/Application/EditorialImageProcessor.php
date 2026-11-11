<?php

namespace App\EditorialMedia\Application;

use GdImage;
use Illuminate\Http\UploadedFile;
use Illuminate\Validation\ValidationException;
use RuntimeException;

final class EditorialImageProcessor
{
    public function process(UploadedFile $file): ProcessedEditorialImage
    {
        $this->assertRuntimeSupport();

        if (! $file->isValid()) {
            $this->reject('The uploaded image could not be read safely.');
        }

        $maxBytes = $this->positiveConfigInt('max_bytes');
        $sourceSize = $file->getSize();

        if (! is_int($sourceSize) || $sourceSize > $maxBytes) {
            $this->reject('The image exceeds the maximum allowed byte size.');
        }

        $sourceBytes = file_get_contents($file->getPathname());

        if (! is_string($sourceBytes) || $sourceBytes === '') {
            $this->reject('The uploaded image could not be read safely.');
        }

        if (strlen($sourceBytes) > $maxBytes) {
            $this->reject('The image exceeds the maximum allowed byte size.');
        }

        $format = $this->formatForOriginalName($file->getClientOriginalName());
        $detectedMime = (new \finfo(FILEINFO_MIME_TYPE))->buffer($sourceBytes);
        $imageInfo = @getimagesizefromstring($sourceBytes);

        if (! is_string($detectedMime) || $imageInfo === false) {
            $this->reject('The file is not a supported decodable image.');
        }

        $headerMime = $imageInfo['mime'];

        if ($detectedMime !== $format['mime'] || $headerMime !== $format['mime']) {
            $this->reject('The image extension does not match its verified content type.');
        }

        $this->assertContainerBoundary($sourceBytes, $format['mime']);
        $width = $imageInfo[0];
        $height = $imageInfo[1];
        $this->assertDimensions($width, $height);
        $this->assertFormatSupport($format['type']);

        $image = @imagecreatefromstring($sourceBytes);

        if (! $image instanceof GdImage) {
            $this->reject('The image content is malformed or unsupported.');
        }

        try {
            if (imagesx($image) !== $width || imagesy($image) !== $height) {
                $this->reject('The decoded image dimensions are inconsistent.');
            }

            $this->prepareAlpha($image, $format['mime']);
            $normalizedBytes = $this->encode($image, $format['mime']);
            [$thumbnailBytes, $thumbnailWidth, $thumbnailHeight] = $this->thumbnail(
                $image,
                $width,
                $height,
                $format['mime'],
            );
        } finally {
            imagedestroy($image);
        }

        if (strlen($normalizedBytes) > $maxBytes) {
            $this->reject('The normalized image exceeds the maximum allowed byte size.');
        }

        $this->assertEncodedImage($normalizedBytes, $format['mime'], $width, $height);

        if ($thumbnailBytes !== null && $thumbnailWidth !== null && $thumbnailHeight !== null) {
            $this->assertEncodedImage(
                $thumbnailBytes,
                $format['mime'],
                $thumbnailWidth,
                $thumbnailHeight,
            );
        }

        return new ProcessedEditorialImage(
            bytes: $normalizedBytes,
            mimeType: $format['mime'],
            extension: $format['extension'],
            width: $width,
            height: $height,
            sha256: hash('sha256', $normalizedBytes),
            thumbnailBytes: $thumbnailBytes,
            thumbnailSha256: $thumbnailBytes === null ? null : hash('sha256', $thumbnailBytes),
            thumbnailWidth: $thumbnailWidth,
            thumbnailHeight: $thumbnailHeight,
        );
    }

    /**
     * @return array{mime: string, extension: string, type: int}
     */
    private function formatForOriginalName(string $originalName): array
    {
        $extension = strtolower(pathinfo($originalName, PATHINFO_EXTENSION));

        return match ($extension) {
            'jpg', 'jpeg' => ['mime' => 'image/jpeg', 'extension' => 'jpg', 'type' => IMG_JPG],
            'png' => ['mime' => 'image/png', 'extension' => 'png', 'type' => IMG_PNG],
            'webp' => ['mime' => 'image/webp', 'extension' => 'webp', 'type' => IMG_WEBP],
            default => $this->reject('Only JPEG, PNG and WebP images are accepted.'),
        };
    }

    private function assertContainerBoundary(string $bytes, string $mimeType): void
    {
        $validBoundary = match ($mimeType) {
            'image/jpeg' => str_starts_with($bytes, "\xFF\xD8")
                && str_ends_with($bytes, "\xFF\xD9"),
            'image/png' => str_starts_with($bytes, "\x89PNG\r\n\x1A\n")
                && str_ends_with($bytes, "\x00\x00\x00\x00IEND\xAE\x42\x60\x82"),
            'image/webp' => $this->hasExactWebpContainerLength($bytes),
            default => false,
        };

        if (! $validBoundary) {
            $this->reject('The image container is malformed or contains an appended payload.');
        }
    }

    private function hasExactWebpContainerLength(string $bytes): bool
    {
        if (
            strlen($bytes) < 12
            || substr($bytes, 0, 4) !== 'RIFF'
            || substr($bytes, 8, 4) !== 'WEBP'
        ) {
            return false;
        }

        $unpacked = unpack('Vsize', substr($bytes, 4, 4));
        $riffSize = is_array($unpacked) ? ($unpacked['size'] ?? null) : null;

        return is_int($riffSize) && $riffSize + 8 === strlen($bytes);
    }

    private function assertDimensions(int $width, int $height): void
    {
        $maxWidth = $this->positiveConfigInt('max_width');
        $maxHeight = $this->positiveConfigInt('max_height');
        $maxPixels = $this->positiveConfigInt('max_pixels');

        if ($width < 1 || $height < 1 || $width > $maxWidth || $height > $maxHeight) {
            $this->reject('The image dimensions exceed the allowed limits.');
        }

        if ($width * $height > $maxPixels) {
            $this->reject('The decoded image pixel count exceeds the allowed limit.');
        }
    }

    private function assertRuntimeSupport(): void
    {
        if (! extension_loaded('gd') || ! function_exists('imagecreatefromstring')) {
            throw new RuntimeException('Editorial image processing is unavailable.');
        }
    }

    private function assertFormatSupport(int $requiredType): void
    {
        if ((imagetypes() & $requiredType) !== $requiredType) {
            throw new RuntimeException('The required editorial image codec is unavailable.');
        }
    }

    private function prepareAlpha(GdImage $image, string $mimeType): void
    {
        if ($mimeType === 'image/jpeg') {
            return;
        }

        imagealphablending($image, false);
        imagesavealpha($image, true);
    }

    private function encode(GdImage $image, string $mimeType): string
    {
        ob_start();

        try {
            $encoded = match ($mimeType) {
                'image/jpeg' => imagejpeg($image, null, $this->boundedConfigInt('jpeg_quality', 0, 100)),
                'image/png' => imagepng($image, null, $this->boundedConfigInt('png_compression', 0, 9)),
                'image/webp' => imagewebp($image, null, $this->boundedConfigInt('webp_quality', 0, 100)),
                default => false,
            };
            $bytes = ob_get_contents();
        } finally {
            ob_end_clean();
        }

        if (! $encoded || ! is_string($bytes) || $bytes === '') {
            throw new RuntimeException('Editorial image encoding failed.');
        }

        return $bytes;
    }

    /**
     * @return array{0: string|null, 1: int|null, 2: int|null}
     */
    private function thumbnail(GdImage $source, int $width, int $height, string $mimeType): array
    {
        $maxDimension = $this->positiveConfigInt('thumbnail_max_dimension');

        if ($width <= $maxDimension && $height <= $maxDimension) {
            return [null, null, null];
        }

        $scale = min($maxDimension / $width, $maxDimension / $height);
        $thumbnailWidth = max(1, (int) floor($width * $scale));
        $thumbnailHeight = max(1, (int) floor($height * $scale));
        $thumbnail = imagecreatetruecolor($thumbnailWidth, $thumbnailHeight);

        if (! $thumbnail instanceof GdImage) {
            throw new RuntimeException('Editorial image thumbnail allocation failed.');
        }

        try {
            if ($mimeType !== 'image/jpeg') {
                imagealphablending($thumbnail, false);
                imagesavealpha($thumbnail, true);
                $transparent = imagecolorallocatealpha($thumbnail, 0, 0, 0, 127);

                if (! is_int($transparent)) {
                    throw new RuntimeException('Editorial image thumbnail alpha allocation failed.');
                }

                imagefill($thumbnail, 0, 0, $transparent);
            }

            imagecopyresampled(
                $thumbnail,
                $source,
                0,
                0,
                0,
                0,
                $thumbnailWidth,
                $thumbnailHeight,
                $width,
                $height,
            );

            $bytes = $this->encode($thumbnail, $mimeType);
        } finally {
            imagedestroy($thumbnail);
        }

        return [$bytes, $thumbnailWidth, $thumbnailHeight];
    }

    private function assertEncodedImage(string $bytes, string $mimeType, int $width, int $height): void
    {
        $this->assertContainerBoundary($bytes, $mimeType);
        $detectedMime = (new \finfo(FILEINFO_MIME_TYPE))->buffer($bytes);
        $imageInfo = @getimagesizefromstring($bytes);

        if (
            $detectedMime !== $mimeType
            || $imageInfo === false
            || $imageInfo['mime'] !== $mimeType
            || $imageInfo[0] !== $width
            || $imageInfo[1] !== $height
        ) {
            throw new RuntimeException('Editorial image encoding verification failed.');
        }
    }

    private function positiveConfigInt(string $key): int
    {
        $value = config('editorial_media.'.$key);

        if (! is_int($value) || $value < 1) {
            throw new RuntimeException('Editorial image processing configuration is invalid.');
        }

        return $value;
    }

    private function boundedConfigInt(string $key, int $minimum, int $maximum): int
    {
        $value = config('editorial_media.'.$key);

        if (! is_int($value) || $value < $minimum || $value > $maximum) {
            throw new RuntimeException('Editorial image processing configuration is invalid.');
        }

        return $value;
    }

    private function reject(string $message): never
    {
        throw ValidationException::withMessages(['image' => $message]);
    }
}
