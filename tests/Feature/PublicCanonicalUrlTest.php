<?php

namespace Tests\Feature;

use Illuminate\Http\Request;
use Illuminate\Support\Facades\URL;
use Tests\TestCase;

class PublicCanonicalUrlTest extends TestCase
{
    private const CANONICAL_ORIGIN = 'https://oteryn.molehill.cloud';

    public function test_requestless_identity_and_signed_urls_use_the_canonical_public_origin(): void
    {
        config()->set('app.url', self::CANONICAL_ORIGIN);
        app('url')->setRequest(Request::create(self::CANONICAL_ORIGIN, 'GET'));

        $urls = [
            route('identity.login.create', absolute: true),
            route('password.reset', [
                'token' => 'redacted-test-token',
                'email' => 'controlled@example.invalid',
            ], true),
            URL::temporarySignedRoute(
                'admin.wiki.articles.preview',
                now()->addMinutes(5),
                ['article' => 1, 'locale' => 'en'],
            ),
        ];

        foreach ($urls as $url) {
            $parts = parse_url($url);

            $this->assertIsArray($parts);
            $this->assertSame('https', $parts['scheme'] ?? null);
            $this->assertSame('oteryn.molehill.cloud', $parts['host'] ?? null);
            $this->assertStringNotContainsString('127.0.0.1', $url);
            $this->assertStringNotContainsString('localhost', $url);
        }
    }

    public function test_synology_public_staging_defaults_to_the_canonical_origin(): void
    {
        $environmentExample = file_get_contents(base_path('deploy/synology/.env.example'));
        $deploymentWorkflow = file_get_contents(base_path('.github/workflows/deploy-synology-staging.yml'));

        $this->assertIsString($environmentExample);
        $this->assertIsString($deploymentWorkflow);
        $this->assertStringContainsString(
            'APP_URL='.self::CANONICAL_ORIGIN,
            $environmentExample,
        );
        $this->assertStringContainsString('SESSION_SECURE_COOKIE=true', $environmentExample);
        $this->assertStringContainsString(
            'CANONICAL_PUBLIC_APP_URL: '.self::CANONICAL_ORIGIN,
            $deploymentWorkflow,
        );
        $this->assertStringContainsString(
            'app_url="${APP_URL_INPUT:-$CANONICAL_PUBLIC_APP_URL}"',
            $deploymentWorkflow,
        );
        $this->assertStringContainsString('SESSION_SECURE_COOKIE=true', $deploymentWorkflow);
        $this->assertStringNotContainsString(
            'APP_URL=http://127.0.0.1:8000',
            $environmentExample,
        );
        $this->assertStringNotContainsString(
            'app_url="${APP_URL_INPUT:-http://127.0.0.1:8000}"',
            $deploymentWorkflow,
        );
    }
}
