<?php

namespace Tests\Feature\PublicPortal;

use App\Http\Controllers\PublicPortal\PublicHomeController;
use App\PublicPortal\Navigation\PublicNavigationRegistry;
use Illuminate\Support\Facades\Route;
use Tests\TestCase;

final class PublicPortalExtensionTest extends TestCase
{
    public function test_public_portal_routes_are_loaded_from_the_module_route_directory(): void
    {
        $home = Route::getRoutes()->getByName('home');

        self::assertNotNull($home);
        self::assertSame(PublicHomeController::class, $home->getActionName());
        self::assertFileExists(base_path('routes/modules/public-portal.php'));
    }

    public function test_public_navigation_exposes_only_registered_named_routes(): void
    {
        $navigation = app(PublicNavigationRegistry::class);
        $items = $navigation->header();

        foreach ($navigation->footer() as $group) {
            $items = [...$items, ...$group['items']];
        }

        self::assertNotEmpty($items);

        foreach ($items as $item) {
            self::assertNotSame('', $item['label']);
            self::assertNotFalse(filter_var($item['url'], FILTER_VALIDATE_URL));
        }

        self::assertSame(
            ['Home', 'News', 'Online', 'Highscores', 'Servers', 'Beginner\'s Guide', 'Support'],
            array_column($navigation->header(), 'label'),
        );
    }
}
