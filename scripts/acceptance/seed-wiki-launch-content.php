<?php

declare(strict_types=1);

use App\Wiki\Content\WikiLaunchContentCatalog;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\DB;

require __DIR__.'/../../vendor/autoload.php';

$app = require __DIR__.'/../../bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

if (! $app->environment('acceptance')) {
    fwrite(STDERR, "Wiki launch-content fixture reset is restricted to the acceptance environment.\n");
    exit(2);
}

$publisher = $argv[1] ?? '';

if ($publisher === '') {
    fwrite(STDERR, "Usage: php scripts/acceptance/seed-wiki-launch-content.php <publisher-email>\n");
    exit(2);
}

DB::transaction(function (): void {
    DB::table('wiki_revisions')->delete();
    DB::table('wiki_article_category')->delete();
    DB::table('wiki_article_translations')->delete();
    DB::table('wiki_category_translations')->delete();
    DB::table('wiki_articles')->delete();
    DB::table('wiki_categories')->delete();
});

$status = Artisan::call('wiki:launch-content:install', [
    'publisher' => $publisher,
    '--content-version' => WikiLaunchContentCatalog::VERSION,
]);

if ($status !== 0) {
    fwrite(STDERR, Artisan::output());
    exit($status);
}

fwrite(STDOUT, Artisan::output());
