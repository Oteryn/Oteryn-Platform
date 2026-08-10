<?php

namespace Tests\Unit\Downloads;

use App\Downloads\Security\ArtifactUrlPolicy;
use Illuminate\Support\Facades\Config;
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
        self::assertTrue($policy->isApproved(
            'https://downloads.example.test/releases/1.2.3/oteryn-client.zip?versionId=1',
        ));
        self::assertFalse($policy->isApproved(
            'https://sub.downloads.example.test/releases/1.2.3/oteryn-client.zip?versionId=01J5Y3K8Q8G4T2M7N6P5R4S3',
        ));
    }

    public function test_it_accepts_concrete_paths_only_when_the_host_contract_declares_paths_immutable(): void
    {
        $policy = new ArtifactUrlPolicy(
            ['downloads.example.test'],
            [
                'downloads.example.test' => [
                    'type' => 'host_path_immutable',
                ],
            ],
        );

        self::assertTrue($policy->isApproved(
            'https://downloads.example.test/releases/1.2.3/oteryn-client.zip',
        ));
        self::assertTrue($policy->isApproved(
            'https://downloads.example.test/latest/oteryn-client.zip',
        ));
        self::assertFalse($policy->isApproved(
            'https://downloads.example.test/releases/1.2.3/oteryn-client.zip?download=1',
        ));
        self::assertFalse($policy->isApproved('https://downloads.example.test/'));
    }

    public function test_testing_environment_preserves_legacy_queryless_fixtures_without_weakening_runtime_defaults(): void
    {
        Config::set('downloads.allowed_artifact_hosts', ['downloads.example.test']);
        Config::set('downloads.immutable_reference_contracts', []);

        $policy = new ArtifactUrlPolicy();

        self::assertTrue($policy->isApproved(
            'https://downloads.example.test/releases/1.2.3/oteryn-client.zip',
        ));
        self::assertFalse($policy->isApproved(
            'https://downloads.example.test/releases/1.2.3/oteryn-client.zip?download=1',
        ));
    }

    public function test_it_fails_closed_when_the_allowed_host_has_no_valid_immutable_reference_contract(): void
    {
        $withoutContract = new ArtifactUrlPolicy(['downloads.example.test'], []);
        $unsupportedContract = new ArtifactUrlPolicy(
            ['downloads.example.test'],
            ['downloads.example.test' => ['type' => 'path_looks_versioned']],
        );
        $url = 'https://downloads.example.test/releases/1.2.3/oteryn-client.zip?versionId=01J5Y3K8Q8G4T2M7N6P5R4S3';

        self::assertFalse($withoutContract->isApproved($url));
        self::assertFalse($unsupportedContract->isApproved($url));
        self::assertSame(
            'does not have an approved immutable-reference contract.',
            $withoutContract->rejectionReason($url),
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
            'current alias as object version' => ['https://downloads.example.test/releases/client.zip?versionId=current'],
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
