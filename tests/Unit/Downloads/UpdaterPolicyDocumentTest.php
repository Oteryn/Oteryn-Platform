<?php

namespace Tests\Unit\Downloads;

use App\Downloads\Updater\UpdaterPolicyDocument;
use PHPUnit\Framework\TestCase;

final class UpdaterPolicyDocumentTest extends TestCase
{
    public function test_policy_document_is_canonical_and_uses_numeric_security_ordering(): void
    {
        $documents = new UpdaterPolicyDocument;
        $targets = [
            [
                'artifact_id' => 20,
                'platform' => 'windows',
                'architecture' => 'x86_64',
                'target_path' => 'channels/stable/releases/rel-a/windows/x86_64/client.zip',
                'size_bytes' => 200,
                'supplied_sha256' => str_repeat('b', 64),
            ],
            [
                'artifact_id' => 10,
                'platform' => 'linux',
                'architecture' => 'arm64',
                'target_path' => 'channels/stable/releases/rel-a/linux/arm64/client.tar.gz',
                'size_bytes' => 100,
                'supplied_sha256' => str_repeat('a', 64),
            ],
        ];

        $first = $documents->encode(
            7,
            'stable',
            'rel-a',
            12,
            '2.0.0',
            4,
            'required',
            $targets,
            ['rel-z', 'rel-b', 'rel-z'],
            ['target-z', 'target-a', 'target-z'],
            'none',
        );
        $second = $documents->encode(
            7,
            'stable',
            'rel-a',
            12,
            '2.0.0',
            4,
            'required',
            array_reverse($targets),
            ['rel-b', 'rel-z'],
            ['target-a', 'target-z'],
            'none',
        );

        self::assertSame($first, $second);
        self::assertSame(hash('sha256', $first), $documents->sha256($first));
        self::assertSame('channels/stable/policy-v1.json', $documents->targetPath('stable'));

        $decoded = json_decode($first, true, 64, JSON_THROW_ON_ERROR);
        self::assertSame(1, $decoded['schema_version']);
        self::assertSame(7, $decoded['policy_revision']);
        self::assertSame(12, $decoded['current_release_sequence']);
        self::assertSame('2.0.0', $decoded['current_version_display']);
        self::assertSame(['rel-b', 'rel-z'], $decoded['revoked_release_ids']);
        self::assertSame(['target-a', 'target-z'], $decoded['revoked_artifact_targets']);
        self::assertSame('linux', $decoded['artifacts'][0]['platform']);
        self::assertSame('windows', $decoded['artifacts'][1]['platform']);
    }
}
