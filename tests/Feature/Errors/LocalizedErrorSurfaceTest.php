<?php

namespace Tests\Feature\Errors;

use PHPUnit\Framework\Attributes\DataProvider;
use Tests\TestCase;

final class LocalizedErrorSurfaceTest extends TestCase
{
    /**
     * @return array<string, array{string, string, string}>
     */
    public static function errorSurfaceProvider(): array
    {
        return [
            'english 419' => ['en', '419', 'The security token expired'],
            'polish 419' => ['pl', '419', 'Token bezpieczeństwa wygasł'],
            'english 429' => ['en', '429', 'Slow down and try again'],
            'polish 429' => ['pl', '429', 'Zwolnij i spróbuj ponownie'],
            'english 500' => ['en', '500', 'Oteryn could not complete this request'],
            'polish 500' => ['pl', '500', 'Oteryn nie mógł wykonać tego żądania'],
        ];
    }

    #[DataProvider('errorSurfaceProvider')]
    public function test_dedicated_error_surface_is_localized_dependency_light_and_non_debug(
        string $locale,
        string $status,
        string $heading,
    ): void {
        app()->setLocale($locale);

        $viewName = match ($status) {
            '419' => 'errors.419',
            '429' => 'errors.429',
            '500' => 'errors.500',
            default => self::fail('Unsupported global error status fixture.'),
        };
        $html = view($viewName)->render();

        self::assertStringContainsString('<html lang="'.$locale.'">', $html);
        self::assertStringContainsString('<meta name="robots" content="noindex, nofollow">', $html);
        self::assertStringContainsString('<p class="error-code">'.$status.'</p>', $html);
        self::assertStringContainsString(e($heading), $html);
        self::assertStringContainsString('class="action-row"', $html);
        self::assertStringNotContainsString('SQLSTATE', $html);
        self::assertStringNotContainsString('Stack trace', $html);
        self::assertStringNotContainsString('InvalidArgumentException', $html);
        self::assertStringNotContainsString(base_path(), $html);
    }

    public function test_global_locale_detector_accepts_only_supported_query_locales_before_route_middleware(): void
    {
        $this->get('/login?locale=pl')
            ->assertOk()
            ->assertHeader('Content-Language', 'pl')
            ->assertSee('<html lang="pl">', false)
            ->assertSee('action="'.route('identity.login.store', ['locale' => 'pl']).'"', false);

        $this->get('/login?locale=en')
            ->assertOk()
            ->assertHeader('Content-Language', 'en')
            ->assertSee('<html lang="en">', false)
            ->assertSee('action="'.route('identity.login.store', ['locale' => 'en']).'"', false);

        $this->get('/login?locale=de')->assertNotFound();
    }

    public function test_localized_missing_route_uses_the_branded_error_layout(): void
    {
        $this->get('/pl/acceptance-missing-error-surface')
            ->assertNotFound()
            ->assertHeader('Content-Language', 'pl')
            ->assertSee('Nie udało się znaleźć tej strony')
            ->assertSee('<meta name="robots" content="noindex, nofollow">', false);
    }
}
