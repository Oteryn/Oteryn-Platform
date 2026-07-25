<?php

namespace App\EditorialMedia\Application;

final readonly class ProcessedEditorialImage
{
    public function __construct(
        public string $bytes,
        public string $mimeType,
        public string $extension,
        public int $width,
        public int $height,
        public string $sha256,
        public ?string $thumbnailBytes,
        public ?int $thumbnailWidth,
        public ?int $thumbnailHeight,
    ) {}
}
