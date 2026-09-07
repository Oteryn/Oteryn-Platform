<?php

namespace Tests\Feature\Operations;

use Tests\TestCase;

final class SecurityHeadersTest extends TestCase
{
    private const CSP = "default-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'; object-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'";

    public function test_public_page_has_enforced_security_headers_and_same_origin_stylesheet(): void
    {
        $response = $this->get('/');

        $response
            ->assertOk()
            ->assertHeader('Content-Security-Policy', self::CSP)
            ->assertHeader('X-Content-Type-Options', 'nosniff')
            ->assertHeader('X-Frame-Options', 'DENY')
            ->assertHeader('Referrer-Policy', 'strict-origin-when-cross-origin')
            ->assertHeader('Permissions-Policy', 'camera=(), geolocation=(), microphone=(), payment=(), usb=()')
            ->assertHeaderMissing('Strict-Transport-Security')
            ->assertSee('href="'.asset('css/portal-system.css').'"', false)
            ->assertSee('href="'.asset('css/portal-pages.css').'"', false)
            ->assertSee('href="'.asset('css/home-production.css').'"', false)
            ->assertDontSee('css/app.css', false)
            ->assertDontSee('<style>', false);

        $csp = $response->headers->get('Content-Security-Policy');
        self::assertIsString($csp);
        self::assertStringNotContainsString("'unsafe-inline'", $csp);
        self::assertStringNotContainsString("'unsafe-eval'", $csp);
    }

    public function test_authentication_page_has_same_security_header_boundary(): void
    {
        $this->get('/login')
            ->assertOk()
            ->assertHeader('Content-Security-Policy', self::CSP)
            ->assertHeader('X-Content-Type-Options', 'nosniff')
            ->assertHeader('X-Frame-Options', 'DENY')
            ->assertHeader('Referrer-Policy', 'strict-origin-when-cross-origin')
            ->assertHeader('Permissions-Policy', 'camera=(), geolocation=(), microphone=(), payment=(), usb=()');
    }
}
