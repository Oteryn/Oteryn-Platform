<?php

namespace Tests\Feature\Security;

use Illuminate\Http\Middleware\TrustProxies;
use Illuminate\Testing\TestResponse;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

final class TrustedProxySchemeTest extends TestCase
{
    protected function setUp(): void
    {
        parent::setUp();

        config(['http.trusted_proxies' => ['10.201.3.0/24']]);
        TrustProxies::flushState();
    }

    protected function tearDown(): void
    {
        TrustProxies::flushState();
        parent::tearDown();
    }

    public function test_configured_reverse_proxy_generates_external_https_login_action(): void
    {
        $response = $this->requestLoginPage(
            remoteAddress: '10.201.3.10',
            forwardedHost: 'platform.oteryn.test',
            forwardedProto: 'https',
        );

        $response->assertOk();

        self::assertSame(
            'https://platform.oteryn.test/login?locale=en',
            $this->loginFormAction($response),
        );
    }

    public function test_untrusted_client_cannot_spoof_forwarded_scheme_or_host(): void
    {
        $configuredScheme = str_starts_with(config()->string('app.url'), 'https://') ? 'https' : 'http';
        $spoofedScheme = $configuredScheme === 'https' ? 'http' : 'https';

        $response = $this->requestLoginPage(
            remoteAddress: '203.0.113.10',
            forwardedHost: 'attacker.example.test',
            forwardedProto: $spoofedScheme,
        );

        $response->assertOk()
            ->assertDontSee('attacker.example.test', false);

        $action = $this->loginFormAction($response);

        self::assertSame($configuredScheme, parse_url($action, PHP_URL_SCHEME));
        self::assertNotSame('attacker.example.test', parse_url($action, PHP_URL_HOST));
        self::assertSame('locale=en', parse_url($action, PHP_URL_QUERY));
    }

    /**
     * @param  TestResponse<Response>  $response
     */
    private function loginFormAction(TestResponse $response): string
    {
        $content = $response->getContent();

        if (! is_string($content)) {
            self::fail('Login response did not contain an HTML string.');
        }

        if (preg_match('/<form[^>]+action="([^"]+)"/', $content, $matches) !== 1) {
            self::fail('Login form action was not present in the response.');
        }

        return html_entity_decode($matches[1], ENT_QUOTES | ENT_HTML5);
    }

    /**
     * @return TestResponse<Response>
     */
    private function requestLoginPage(string $remoteAddress, string $forwardedHost, string $forwardedProto): TestResponse
    {
        return $this
            ->withServerVariables([
                'REMOTE_ADDR' => $remoteAddress,
                'REMOTE_PORT' => '49152',
                'SERVER_ADDR' => '10.201.3.20',
                'SERVER_NAME' => 'platform-backend',
                'SERVER_PORT' => '8080',
                'REQUEST_SCHEME' => 'http',
                'HTTPS' => 'off',
            ])
            ->withHeaders([
                'Host' => 'platform-backend',
                'X-Forwarded-For' => '198.51.100.20',
                'X-Forwarded-Host' => $forwardedHost,
                'X-Forwarded-Port' => '443',
                'X-Forwarded-Proto' => $forwardedProto,
            ])
            ->get('/login');
    }
}
