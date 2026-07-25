<?php

namespace Tests\Unit\Downloads;

use App\Downloads\Security\ArtifactUrlPolicy;
use PHPUnit\Framework\Attributes\DataProvider;
use Tests\TestCase;

final class ArtifactUrlPolicyTest extends TestCase
{
    #[DataProvider('rejectedUrls')]
    public function test_it_rejects_unsafe_or_unapproved_artifact_urls(string $url): void
    {
        self::assertFalse((new ArtifactUrlPolicy(['downloads.example.test']))->isApproved($url));
    }

    public function test_it_accepts_only_an_exact_configured_https_host_with_a_concrete_path(): void
    {
        $policy = new ArtifactUrlPolicy(['downloads.example.test']);

        self::assertTrue($policy->isApproved(
            'https://downloads.example.test/releases/1.2.3/oteryn-client.zip',
        ));
        self::assertFalse($policy->isApproved(
            'https://sub.downloads.example.test/releases/1.2.3/oteryn-client.zip',
        ));
    }

    /**
     * @return array<string, array{string}>
     */
    public static function rejectedUrls(): array
    {
        return [
            'javascript scheme' => ['javascript:alert(1)'],
            'data scheme' => ['data:application/octet-stream;base64,AA=='],
            'plain http' => ['http://downloads.example.test/releases/client.zip'],
            'unapproved host' => ['https://evil.example.test/releases/client.zip'],
            'approved-looking suffix' => ['https://downloads.example.test.evil.test/releases/client.zip'],
            'userinfo' => ['https://user@downloads.example.test/releases/client.zip'],
            'fragment' => ['https://downloads.example.test/releases/client.zip#download'],
            'nonstandard port' => ['https://downloads.example.test:8443/releases/client.zip'],
            'host root only' => ['https://downloads.example.test/'],
            'control character' => ["https://downloads.example.test/releases/client.zip\n"],
        ];
    }
}
