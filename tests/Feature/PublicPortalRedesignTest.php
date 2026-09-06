<?php

namespace Tests\Feature;

use App\Announcements\ViewModels\AnnouncementTicker;
use App\Announcements\ViewModels\AnnouncementTickerState;
use App\Events\ViewModels\UpcomingEventState;
use App\Events\ViewModels\UpcomingEventSummary;
use App\PublicPortal\PublicContentState;
use App\PublicPortal\ViewModels\HomeNewsSummary;
use App\PublicPortal\ViewModels\HomePageViewModel;
use App\PublicPortal\ViewModels\HomeWorldChannel;
use App\PublicPortal\ViewModels\HomeWorldSummary;
use DOMDocument;
use DOMElement;
use DOMXPath;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\URL;
use PHPUnit\Framework\Attributes\DataProvider;
use Tests\TestCase;

final class PublicPortalRedesignTest extends TestCase
{
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();

        app()->setLocale('en');
        URL::defaults(['locale' => 'en']);
        $this->withViewErrors([]);
    }

    /** @return iterable<string, array{string, PublicContentState}> */
    public static function states(): iterable
    {
        foreach (['en', 'pl'] as $locale) {
            foreach (PublicContentState::cases() as $state) {
                yield $locale.'-'.$state->value => [$locale, $state];
            }
        }
    }

    #[DataProvider('states')]
    public function test_hero_preserves_localized_truthful_world_states(string $locale, PublicContentState $state): void
    {
        app()->setLocale($locale);
        URL::defaults(['locale' => $locale]);
        $world = new HomeWorldSummary($state, [], $state === PublicContentState::AVAILABLE ? 42 : null);
        $view = $this->view('home', ['homePage' => $this->home($world)]);
        $view->assertSee('data-hero-world-state="'.$state->value.'"', false)
            ->assertSee('aria-label="Oteryn Platform">OTERYN</h1>', false)
            ->assertSee('css/portal-system.css', false)
            ->assertDontSee('css/home-preview.css', false);

        if ($state === PublicContentState::AVAILABLE) {
            $view->assertSee(trans_choice('public.home.players_online', 42, ['count' => '42']));
        } else {
            $view->assertSee(__('portal.home.summary_'.strtolower($state->value)))
                ->assertDontSee('42 players online')
                ->assertDontSee('0 players online')
                ->assertDontSee('42 graczy online')
                ->assertDontSee('0 graczy online');
        }
    }

    public function test_configured_maintenance_is_visible_in_hero_and_details_without_unescaped_content(): void
    {
        $world = new HomeWorldSummary(PublicContentState::AVAILABLE, [
            new HomeWorldChannel(1, 'Alpha', 'open-pvp', 500, true, '<script>not markup</script>', 'ONLINE', 12),
        ], 12);

        $this->view('home', ['homePage' => $this->home($world)])
            ->assertSee('class="production-hero-maintenance"', false)
            ->assertSee('Configured maintenance.')
            ->assertSee('&lt;script&gt;not markup&lt;/script&gt;', false)
            ->assertDontSee('<script>not markup</script>', false);
    }

    public function test_localized_home_navigation_is_current_and_all_registered_destinations_remain(): void
    {
        foreach (['en', 'pl'] as $locale) {
            $response = $this->get('/'.$locale)->assertOk();
            $html = $response->getContent();
            self::assertIsString($html);
            $document = new DOMDocument;
            $previous = libxml_use_internal_errors(true);
            try {
                $document->loadHTML($html);
            } finally {
                libxml_clear_errors();
                libxml_use_internal_errors($previous);
            }
            $xpath = new DOMXPath($document);
            $current = $xpath->query('//nav[contains(@class,"primary-nav")]/a[@aria-current="page"]');
            self::assertNotFalse($current);
            self::assertSame(1, $current->length);
            $activeLink = $current->item(0);
            self::assertInstanceOf(DOMElement::class, $activeLink);
            self::assertSame(url('/'.$locale), $activeLink->getAttribute('href'));
            $response->assertSee('/'.$locale.'/guilds', false)
                ->assertSee('/'.$locale.'/download', false)
                ->assertSee('/'.$locale.'/wiki', false)
                ->assertSee('/'.$locale.'/support', false);
        }
    }

    public function test_identity_and_error_pages_share_visual_system_without_changing_security_metadata(): void
    {
        $this->get('/login')->assertOk()
            ->assertSee('css/portal-system.css', false)
            ->assertSee('images/oteryn-wordmark.svg', false)
            ->assertSee('noindex,nofollow,noarchive', false)
            ->assertSee('autocomplete="current-password"', false);
        $this->get('/portal-redesign-missing-page')->assertNotFound()
            ->assertSee('portal-error-body', false)
            ->assertSee('css/portal-system.css', false);
    }

    private function home(HomeWorldSummary $world): HomePageViewModel
    {
        return new HomePageViewModel(
            $world,
            new HomeNewsSummary(PublicContentState::EMPTY, []),
            new AnnouncementTicker(AnnouncementTickerState::EMPTY, []),
            new UpcomingEventSummary(UpcomingEventState::EMPTY, null),
        );
    }
}
