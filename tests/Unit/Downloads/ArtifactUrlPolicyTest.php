<?php

namespace Tests\Unit\Downloads;

use App\Downloads\Security\ArtifactUrlPolicy;
use PHPUnit\Framework\Attributes\DataProvider;
use Tests\TestCase;

final class ArtifactUrlPolicyTest extends TestCase
{
    #[DataProvider('rejectedUrls')]
    public function test_it_rejects_unsafe_unapproved_or_mutable_artifact_urls(string $url): void
    {
        self::assertFalse($this->policy()->isApproved($url));
    }

    public function test_it_accepts_only_an_exact_configured_https_host_with_an_object_version_reference(): void
    {
        $policy = $this->policy();

        self::assertTrue($policy->isApproved(
            'https://downloads.example.test/releases/1.2.3/oteryn-client.zip?versionId=01J5Y3K8Q8G4T2M7N6P5R4S3',
        ));
        self::assertFalse($policy->isApproved(
            'https://sub.downloads.example.test/releases/1.2.3/oteryn-client.zip?versionId=01J5Y3K8Q8G4T2M7N6P5R4S3',
        ));
    }

    public function test_it_fails_closed_when_the_allowed_host_has_no_immutable_reference_contract(): void
    {
        $policy = new ArtifactUrlPolicy(['downloads.example.test'], []);

        self::assertFalse($policy->isApproved(
            'https://downloads.example.test/releases/1.2.3/oteryn-client.zip?versionId=01J5Y3K8Q8G4T2M7N6P5R4S3',
        ));
        self::assertSame(
            'does not have an approved immutable-reference contract.',
            $policy->rejectionReason(
                'https://downloads.example.test/releases/1.2.3/oteryn-client.zip?versionId=01J5Y3K8Q8G4T2M7N6P5R4S3',
            ),
        );
    }

    /**
     * @return array<string, array{string}>
     */
    public static function rejectedUrls(): array
    {
        return [
            'javascript scheme' => ['javascript:alert(1)'],
            'data scheme' => ['data:application/octet-stream;base64,AA=='],
            'plain http' => ['http://downloads.example.test/releases/client.zip?versionId=01J5Y3K8Q8G4T2M7N6P5R4S3'],
            'unapproved host' => ['https://evil.example.test/releases/client.zip?versionId=01J5Y3K8Q8G4T2M7N6P5R4S3'],
            'approved-looking suffix' => ['https://downloads.example.test.evil.test/releases/client.zip?versionId=01J5Y3K8Q8G4T2M7N6P5R4S3'],
            'userinfo' => ['https://user@downloads.example.test/releases/client.zip?versionId=01J5Y3K8Q8G4T2M7N6P5R4S3'],
            'fragment' => ['https://downloads.example.test/releases/client.zip?versionId=01J5Y3K8Q8G4T2M7N6P5R4S3#download'],
            'nonstandard port' => ['https://downloads.example.test:8443/releases/client.zip?versionId=01J5Y3K8Q8G4T2M7N6P5R4S3'],
            'host root only' => ['https://downloads.example.test/?versionId=01J5Y3K8Q8G4T2M7N6P5R4S3'],
            'control character' => ["https://downloads.example.test/releases/client.zip?versionId=01J5Y3K8Q8G4T2M7N6P5R4S3\n"],
            'version-looking path without immutable reference' => ['https://downloads.example.test/releases/1.2.3/client.zip'],
            'latest alias without immutable reference' => ['https://downloads.example.test/latest/client.zip'],
            'current alias without immutable reference' => ['https://downloads.example.test/current/client.zip'],
            'mutable alias as object version' => ['https://downloads.example.test/releases/client.zip?versionId=latest'],
            'wrong object version parameter' => ['https://downloads.example.test/releases/client.zip?objectVersion=01J5Y3K8Q8G4T2M7N6P5R4S3'],
            'duplicate object version parameter' => ['https://downloads.example.test/releases/client.zip?versionId=01J5Y3K8Q8G4T2M7N6P5R4S3&versionId=01J5Y3K8Q8G4T2M7N6P5R4S4'],
            'extra query parameter' => ['https://downloads.example.test/releases/client.zip?versionId=01J5Y3K8Q8G4T2M7N6P5R4S3&download=1'],
            'empty object version' => ['https://downloads.example.test/releases/client.zip?versionId='],
        ];
    }

    private function policy(): ArtifactUrlPolicy
    {
        return new ArtifactUrlPolicy(
            ['downloads.example.test'],
            [
                'downloads.example.test' => [
                    'type' => 'object_version_query',
                    'parameter' => 'versionId',
                ],
            ],
        );
    }
}
