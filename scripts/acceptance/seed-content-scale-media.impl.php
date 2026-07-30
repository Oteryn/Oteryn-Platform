<?php

declare(strict_types=1);

use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Storage;

require dirname(__DIR__, 2).'/vendor/autoload.php';

$app = require dirname(__DIR__, 2).'/bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$disk = Storage::disk('editorial_media');
$existing = DB::table('editorial_media')
    ->where('storage_path', 'like', 'originals/content-scale-media-%')
    ->get(['id', 'storage_path', 'thumbnail_path']);
$existingIds = $existing->pluck('id')->map(static fn (mixed $id): int => (int) $id)->all();

if ($existingIds !== []) {
    DB::table('editorial_media_references')->whereIn('media_id', $existingIds)->delete();
    DB::table('editorial_media')->whereIn('id', $existingIds)->delete();
}
foreach ($existing as $media) {
    $disk->delete((string) $media->storage_path);
    if (is_string($media->thumbnail_path)) {
        $disk->delete($media->thumbnail_path);
    }
}

$bytes = base64_decode('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl2jXQAAAAASUVORK5CYII=', true);
if (! is_string($bytes)) {
    throw new RuntimeException('Cannot decode deterministic content-scale image fixture.');
}
$sha256 = hash('sha256', $bytes);
$byteSize = strlen($bytes);
$now = now();
$ids = [];
$longName = 'content-scale-'.implode('-', array_map(
    static fn (int $index): string => sprintf('media-segment-%02d', $index),
    range(1, 13),
)).'.png';
$longAlt = 'Content scale editorial media boundary '.implode(' ', array_map(
    static fn (int $index): string => sprintf('alternative-text-segment-%02d', $index),
    range(1, 24),
));

for ($index = 1; $index <= 25; $index++) {
    $storagePath = sprintf('originals/content-scale-media-%03d.png', $index);
    $thumbnailPath = sprintf('thumbnails/content-scale-media-%03d.png', $index);
    $disk->put($storagePath, $bytes);
    $disk->put($thumbnailPath, $bytes);

    $id = DB::table('editorial_media')->insertGetId([
        'disk' => 'editorial_media',
        'storage_path' => $storagePath,
        'thumbnail_path' => $thumbnailPath,
        'original_name' => $index === 25 ? $longName : sprintf('content-scale-media-%03d.png', $index),
        'mime_type' => 'image/png',
        'extension' => 'png',
        'byte_size' => $byteSize,
        'width' => 1,
        'height' => 1,
        'thumbnail_byte_size' => $byteSize,
        'thumbnail_sha256' => $sha256,
        'thumbnail_width' => 1,
        'thumbnail_height' => 1,
        'sha256' => $sha256,
        'alt_text' => $index === 25 ? $longAlt : sprintf('Content scale media fixture %03d', $index),
        'uploaded_by_identity_id' => null,
        'created_at' => $now->copy()->addSeconds($index),
        'updated_at' => $now->copy()->addSeconds($index),
    ]);
    $ids[] = (int) $id;
}

fwrite(STDOUT, json_encode([
    'media_long_id' => $ids[24],
    'media_long_name' => $longName,
    'media_long_alt' => $longAlt,
    'media_page_two_id' => $ids[0],
], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES).PHP_EOL);
