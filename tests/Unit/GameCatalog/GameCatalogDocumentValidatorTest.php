<?php

namespace Tests\Unit\GameCatalog;

use App\GameCatalog\Contract\GameCatalogContract;
use App\GameCatalog\Validation\CatalogValidationException;
use App\GameCatalog\Validation\GameCatalogDocumentValidator;
use Tests\TestCase;

final class GameCatalogDocumentValidatorTest extends TestCase
{
    /** @var list<string> */
    private array $temporaryFiles = [];

    protected function tearDown(): void
    {
        foreach ($this->temporaryFiles as $file) {
            @unlink($file);
            @unlink($file.'.sha256');
        }

        parent::tearDown();
    }

    public function test_shared_fixture_and_schema_hash_are_pinned(): void
    {
        $fixture = base_path('resources/fixtures/game-catalog/minimal-snapshot.json');
        $schema = base_path(GameCatalogContract::SCHEMA_PATH);

        self::assertSame(GameCatalogContract::SCHEMA_SHA256, hash_file('sha256', $schema));

        $validated = (new GameCatalogDocumentValidator)->validatePath($fixture);

        self::assertSame('ec0658bb11877240f2e22575180513dbff426b3df1fc2af8f20343ed0d424055', $validated->contentSha256);
        self::assertSame(4, $validated->document['snapshot']['entity_count']);
        self::assertSame(2, $validated->document['snapshot']['relation_count']);
    }

    public function test_expected_or_sidecar_hash_mismatch_is_rejected(): void
    {
        $path = $this->copyFixture();
        file_put_contents($path.'.sha256', str_repeat('f', 64)."\n");

        $this->expectException(CatalogValidationException::class);
        (new GameCatalogDocumentValidator)->validatePath($path);
    }

    public function test_unknown_schema_property_is_rejected(): void
    {
        $document = $this->fixtureDocument();
        $document['unexpected'] = true;

        $this->assertRejected($document, 'schema.additional_property');
    }

    public function test_invalid_exclusive_version_range_is_rejected(): void
    {
        $document = $this->fixtureDocument();
        $document['entities'][0]['removed_in'] = '15.20';

        $this->assertRejected($document, 'semantic.invalid_version_range');
    }

    public function test_duplicate_canonical_key_is_rejected(): void
    {
        $document = $this->fixtureDocument();
        $document['entities'][1]['canonical_key'] = 'creature:dragon';

        $this->assertRejected($document, 'semantic.duplicate_entity');
    }

    public function test_dangling_relation_endpoint_is_rejected(): void
    {
        $document = $this->fixtureDocument();
        $document['relations'][0]['target'] = 'item:missing';

        $this->assertRejected($document, 'semantic.dangling_relation');
    }

    public function test_invalid_loot_probability_and_count_range_are_rejected(): void
    {
        $document = $this->fixtureDocument();
        $document['relations'][0]['data']['chance_numerator'] = 100001;
        $document['relations'][0]['data']['minimum_count'] = 2;
        $document['relations'][0]['data']['maximum_count'] = 1;

        $path = $this->writeDocument($document);

        try {
            (new GameCatalogDocumentValidator)->validatePath($path);
            self::fail('Invalid loot relation was accepted.');
        } catch (CatalogValidationException $exception) {
            $codes = array_column($exception->findings(), 'code');
            self::assertContains('semantic.invalid_probability', $codes);
            self::assertContains('semantic.invalid_count_range', $codes);
        }
    }

    /** @return array<string, mixed> */
    private function fixtureDocument(): array
    {
        $raw = file_get_contents(base_path('resources/fixtures/game-catalog/minimal-snapshot.json'));
        self::assertIsString($raw);
        $document = json_decode($raw, true, 512, JSON_THROW_ON_ERROR);
        self::assertIsArray($document);

        return $document;
    }

    /** @param array<string, mixed> $document */
    private function assertRejected(array $document, string $expectedCode): void
    {
        $path = $this->writeDocument($document);

        try {
            (new GameCatalogDocumentValidator)->validatePath($path);
            self::fail('Invalid snapshot was accepted.');
        } catch (CatalogValidationException $exception) {
            self::assertContains($expectedCode, array_column($exception->findings(), 'code'));
        }
    }

    /** @param array<string, mixed> $document */
    private function writeDocument(array $document): string
    {
        foreach ($document['entities'] as &$entity) {
            if (isset($entity['data']['attributes']) && $entity['data']['attributes'] === []) {
                $entity['data']['attributes'] = (object) [];
            }
        }
        unset($entity);

        $path = tempnam(sys_get_temp_dir(), 'oteryn-catalog-');
        self::assertIsString($path);
        $this->temporaryFiles[] = $path;
        file_put_contents(
            $path,
            json_encode($document, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE)."\n",
        );

        return $path;
    }

    private function copyFixture(): string
    {
        $path = tempnam(sys_get_temp_dir(), 'oteryn-catalog-');
        self::assertIsString($path);
        $this->temporaryFiles[] = $path;
        copy(base_path('resources/fixtures/game-catalog/minimal-snapshot.json'), $path);

        return $path;
    }
}
