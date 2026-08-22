<?php

namespace Tests\Feature\GameCatalog;

use App\GameCatalog\Application\Import\Native\NativeCatalogContract;
use App\GameCatalog\Application\Import\Native\NativeCatalogEnvelopeValidator;
use App\GameCatalog\Application\Import\Native\ValidatedNativeCatalogSnapshot;
use App\GameCatalog\Domain\Exceptions\CatalogValidationException;
use JsonException;
use RuntimeException;
use Tests\TestCase;

/**
 * @phpstan-import-type NativeCatalogPayload from ValidatedNativeCatalogSnapshot
 * @phpstan-import-type NativeEntity from ValidatedNativeCatalogSnapshot
 */
final class NativeCatalogEnvelopeValidatorTest extends TestCase
{
    public function test_locked_game_fixture_validates_without_persistence_or_activation(): void
    {
        $validated = $this->validator()->validate(
            $this->fixturePath(),
            trim((string) file_get_contents($this->fixturePath().'.sha256')),
        );

        self::assertSame(NativeCatalogContract::CONTRACT_ID, $validated->payload['contract_id']);
        self::assertSame(NativeCatalogContract::SCHEMA_VERSION, $validated->payload['schema_version']);
        self::assertSame(NativeCatalogContract::CONTENT_AUTHORITY_ID, $validated->payload['content_authority_id']);
        self::assertSame('4fe8ce4488c21396b588dc6b04179cde93c7a9b22b87427b6e4af3cda6e60d7d', $validated->artifactSha256);
        self::assertSame([], $validated->payload['entities']);
        self::assertSame([], $validated->payload['relations']);
        self::assertSame([], $validated->payload['tombstones']);
    }

    public function test_fixture_preserves_unsupported_unknown_capability_truth(): void
    {
        $validated = $this->validator()->validate($this->fixturePath());

        foreach ($validated->payload['capability_manifest'] as $capability) {
            self::assertSame('unsupported', $capability['support']);
        }
        foreach ($validated->payload['completeness_manifest'] as $capability) {
            self::assertSame('unknown', $capability['state']);
        }
        self::assertSame([], $validated->payload['required_capabilities']);
    }

    public function test_contract_pin_matches_locked_game_contract(): void
    {
        self::assertSame('Oteryn/Oteryn-Game', NativeCatalogContract::CANONICAL_REPOSITORY);
        self::assertSame('96ea673839f1d93190a40c17ae8036ac82096ded', NativeCatalogContract::CANONICAL_COMMIT);
        self::assertSame('9bc87fba5b565e5c7d4d3f6ca7a9bd75d45d8110de64a2a50f8f74d9ba181cad', NativeCatalogContract::CONTRACT_SHA256);
    }

    public function test_wrong_artifact_hash_fails_closed(): void
    {
        $this->assertFailsWithCode(
            'native.input.hash_mismatch',
            fn () => $this->validator()->validate($this->fixturePath(), str_repeat('0', 64)),
        );
    }

    public function test_file_limit_is_checked_before_decode(): void
    {
        $this->assertFailsWithCode(
            'native.input.file_too_large',
            fn () => $this->validator(32)->validate($this->fixturePath()),
        );
    }

    public function test_unknown_v1_capability_fails_closed(): void
    {
        $path = $this->temporarySnapshot(function (array &$payload): void {
            $payload['capability_manifest'][] = ['capability_id' => 'future_power', 'support' => 'unsupported'];
            $payload['completeness_manifest'][] = ['capability_id' => 'future_power', 'state' => 'unknown'];
        });

        try {
            $this->assertFailsWithCode(
                'native.capability.unknown',
                fn () => $this->validator()->validate($path),
            );
        } finally {
            @unlink($path);
        }
    }

    public function test_unsupported_required_capability_fails_closed(): void
    {
        $path = $this->temporarySnapshot(static function (array &$payload): void {
            $payload['required_capabilities'] = ['npc'];
        });

        try {
            $this->assertFailsWithCode(
                'native.capability.required_unsupported',
                fn () => $this->validator()->validate($path),
            );
        } finally {
            @unlink($path);
        }
    }

    public function test_duplicate_entity_identity_fails_closed(): void
    {
        $path = $this->temporarySnapshot(function (array &$payload): void {
            $this->support($payload, 'item', 'partial');
            $entity = $this->entity('oteryn:item.training_sword');
            $payload['entities'] = [$entity, $entity];
        });

        try {
            $this->assertFailsWithCode(
                'native.entity.duplicate',
                fn () => $this->validator()->validate($path),
            );
        } finally {
            @unlink($path);
        }
    }

    public function test_noncanonical_entity_order_fails_closed(): void
    {
        $path = $this->temporarySnapshot(function (array &$payload): void {
            $this->support($payload, 'item', 'partial');
            $payload['entities'] = [
                $this->entity('oteryn:item.zeta'),
                $this->entity('oteryn:item.alpha'),
            ];
        });

        try {
            $this->assertFailsWithCode(
                'native.entity.ordering',
                fn () => $this->validator()->validate($path),
            );
        } finally {
            @unlink($path);
        }
    }

    public function test_dangling_relation_target_fails_closed(): void
    {
        $path = $this->temporarySnapshot(function (array &$payload): void {
            $this->support($payload, 'item', 'partial');
            $payload['entities'] = [$this->entity('oteryn:item.training_sword')];
            $payload['relations'] = [[
                'type' => 'item_upgrade',
                'relation_key' => 'oteryn:relation.training_upgrade',
                'capability_id' => 'item',
                'source' => 'oteryn:item.training_sword',
                'target' => 'oteryn:item.missing',
                'data' => [],
            ]];
        });

        try {
            $this->assertFailsWithCode(
                'native.relation.dangling_target',
                fn () => $this->validator()->validate($path),
            );
        } finally {
            @unlink($path);
        }
    }

    public function test_tombstone_requires_complete_capability(): void
    {
        $path = $this->temporarySnapshot(function (array &$payload): void {
            $this->support($payload, 'item', 'partial');
            $payload['tombstones'] = [[
                'content_key' => 'oteryn:item.retired_blade',
                'capability_id' => 'item',
                'reason' => 'removed',
            ]];
        });

        try {
            $this->assertFailsWithCode(
                'native.tombstone.incomplete_capability',
                fn () => $this->validator()->validate($path),
            );
        } finally {
            @unlink($path);
        }
    }

    public function test_float_in_native_data_is_rejected(): void
    {
        $path = $this->temporarySnapshot(function (array &$payload): void {
            $this->support($payload, 'item', 'partial');
            $entity = $this->entity('oteryn:item.training_sword');
            $entity['data'] = ['weight' => 1.5];
            $payload['entities'] = [$entity];
        });

        try {
            $this->assertFailsWithCode(
                'native.data.float_forbidden',
                fn () => $this->validator()->validate($path),
            );
        } finally {
            @unlink($path);
        }
    }

    public function test_modified_provenance_breaks_payload_digest(): void
    {
        $path = $this->temporarySnapshot(static function (array &$payload): void {
            $payload['generated_at'] = '2026-08-22T18:05:01Z';
        });

        try {
            $this->assertFailsWithCode(
                'native.digest.mismatch',
                fn () => $this->validator()->validate($path),
            );
        } finally {
            @unlink($path);
        }
    }

    /** @param callable(NativeCatalogPayload&): void $mutate */
    private function temporarySnapshot(callable $mutate): string
    {
        try {
            $contents = file_get_contents($this->fixturePath());
            if (! is_string($contents)) {
                throw new RuntimeException('Native Game Catalog fixture is unreadable.');
            }
            $payload = json_decode($contents, true, 32, JSON_THROW_ON_ERROR);
            if (! is_array($payload) || array_is_list($payload)) {
                throw new RuntimeException('Native Game Catalog fixture root is invalid.');
            }
            /** @var NativeCatalogPayload $payload */
            $mutate($payload);
            $encoded = json_encode(
                $payload,
                JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE,
            )."\n";
        } catch (JsonException $exception) {
            self::fail($exception->getMessage());
        }

        $path = tempnam(sys_get_temp_dir(), 'native-game-catalog-');
        self::assertIsString($path);
        file_put_contents($path, $encoded);

        return $path;
    }

    /**
     * @param  NativeCatalogPayload  $payload
     * @param  'complete'|'partial'|'unknown'  $state
     */
    private function support(array &$payload, string $capabilityId, string $state): void
    {
        foreach ($payload['capability_manifest'] as &$capability) {
            if ($capability['capability_id'] === $capabilityId) {
                $capability['support'] = 'supported';
            }
        }
        unset($capability);

        foreach ($payload['completeness_manifest'] as &$capability) {
            if ($capability['capability_id'] === $capabilityId) {
                $capability['state'] = $state;
            }
        }
        unset($capability);
    }

    /** @return NativeEntity */
    private function entity(string $key): array
    {
        return [
            'type' => 'item',
            'content_key' => $key,
            'capability_id' => 'item',
            'data' => [],
        ];
    }

    private function validator(?int $maximumFileBytes = null): NativeCatalogEnvelopeValidator
    {
        return new NativeCatalogEnvelopeValidator($maximumFileBytes);
    }

    /** @param callable(): mixed $operation */
    private function assertFailsWithCode(string $expectedCode, callable $operation): void
    {
        try {
            $operation();
            self::fail("Expected native catalog failure {$expectedCode}.");
        } catch (CatalogValidationException $exception) {
            self::assertNotSame([], $exception->findings);
            self::assertSame($expectedCode, $exception->findings[0]->code);
        }
    }

    private function fixturePath(): string
    {
        return base_path('tests/Fixtures/GameCatalog/native-v1/unsupported-snapshot.json');
    }
}
