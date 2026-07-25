<?php

return [
    'disk' => env('EDITORIAL_MEDIA_DISK', 'editorial_media'),
    'max_bytes' => 8 * 1024 * 1024,
    'max_width' => 6000,
    'max_height' => 6000,
    'max_pixels' => 20_000_000,
    'thumbnail_max_dimension' => 320,
    'jpeg_quality' => 88,
    'webp_quality' => 85,
    'png_compression' => 6,
];
