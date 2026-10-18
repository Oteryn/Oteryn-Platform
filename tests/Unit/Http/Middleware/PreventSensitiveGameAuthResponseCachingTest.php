<?php

namespace Tests\Unit\Http\Middleware;

use App\Http\Middleware\GameAuth\PreventSensitiveGameAuthResponseCaching;
use Illuminate\Http\Request;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;
use Symfony\Component\HttpFoundation\Response;

final class PreventSensitiveGameAuthResponseCachingTest extends TestCase
{
    /**
     * @return iterable<string, array{string}>
     */
    public static function sensitivePaths(): iterable
    {
        yield 'OAuth token' => ['/oauth/token'];
        yield 'Game Login Ticket issue' => ['/api/v1/game-auth/tickets'];
        yield 'Game Login Ticket redeem' => ['/internal/v1/game-auth/tickets/redeem'];
        yield 'Device enrollment' => ['/api/v1/game-auth/device-sessions'];
        yield 'Device owner characters' => ['/api/v1/game-auth/device-sessions/native-characters'];
        yield 'Device ticket' => ['/api/v1/game-auth/device-sessions/tickets'];
        yield 'Device revoke' => ['/api/v1/game-auth/device-sessions/revoke'];
        yield 'Premium time snapshot read' => ['/internal/v1/products-entitlements/premium-snapshots/read'];
    }

    #[DataProvider('sensitivePaths')]
    public function test_sensitive_responses_receive_complete_cache_headers(string $path): void
    {
        $request = Request::create($path, 'POST');
        $middleware = new PreventSensitiveGameAuthResponseCaching;

        $response = $middleware->handle(
            $request,
            static fn (Request $request): Response => new Response('{"ok":true}', 200),
        );

        self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));
        self::assertStringContainsString('no-cache', (string) $response->headers->get('Cache-Control'));
        self::assertStringContainsString('must-revalidate', (string) $response->headers->get('Cache-Control'));
        self::assertStringContainsString('private', (string) $response->headers->get('Cache-Control'));
        self::assertSame('no-cache', $response->headers->get('Pragma'));
        self::assertSame('0', $response->headers->get('Expires'));
    }

    public function test_unrelated_response_is_not_modified(): void
    {
        $request = Request::create('/health', 'GET');
        $middleware = new PreventSensitiveGameAuthResponseCaching;
        $expected = new Response('ok', 200);

        $response = $middleware->handle(
            $request,
            static fn (Request $request): Response => $expected,
        );

        self::assertSame($expected, $response);
        self::assertFalse($response->headers->has('Pragma'));
        self::assertFalse($response->headers->has('Expires'));
    }
}
