<?php

declare(strict_types=1);

use App\EditorialMedia\Application\Actions\StoreEditorialImage;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Http\UploadedFile;

require __DIR__.'/../../vendor/autoload.php';

$app = require __DIR__.'/../../bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$email = $argv[1] ?? '';
$label = trim($argv[2] ?? 'Oteryn acceptance bridge');

if ($email === '' || $label === '') {
    fwrite(STDERR, "Usage: php scripts/acceptance/seed-wiki-editorial-media.php <email> [label]\n");
    exit(2);
}

$identity = Identity::query()->where('email', $email)->first();
if (! $identity instanceof Identity) {
    throw new RuntimeException('The acceptance Wiki editor must be seeded before EditorialMedia.');
}

$image = imagecreatetruecolor(240, 135);
if (! $image instanceof GdImage) {
    throw new RuntimeException('Could not create the Wiki EditorialMedia acceptance fixture.');
}

$background = imagecolorallocate($image, 28, 66, 92);
$accent = imagecolorallocate($image, 184, 142, 76);
if (! is_int($background) || ! is_int($accent)) {
    throw new RuntimeException('Could not allocate acceptance image colours.');
}

imagefill($image, 0, 0, $background);
imagefilledrectangle($image, 28, 78, 212, 92, $accent);
$temporaryPath = tempnam(sys_get_temp_dir(), 'oteryn-wiki-media-');

if (! is_string($temporaryPath)) {
    throw new RuntimeException('Could not allocate a Wiki EditorialMedia acceptance file.');
}

try {
    if (! imagepng($image, $temporaryPath, 6)) {
        throw new RuntimeException('Could not encode the Wiki EditorialMedia acceptance fixture.');
    }

    $media = app(StoreEditorialImage::class)->execute(
        $identity,
        new UploadedFile(
            $temporaryPath,
            'wiki-acceptance-bridge.png',
            'image/png',
            null,
            true,
        ),
        $label,
    );
} finally {
    imagedestroy($image);

    if (is_file($temporaryPath)) {
        unlink($temporaryPath);
    }
}

fwrite(STDOUT, json_encode([
    'media_id' => $media->id,
    'alt_text' => $media->alt_text,
], JSON_THROW_ON_ERROR)."\n");
