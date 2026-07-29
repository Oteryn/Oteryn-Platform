<?php

namespace Tests\Feature\GameCatalog;

use App\GameCatalog\Application\Activation\CatalogActivationService;
use App\GameCatalog\Application\Import\CatalogImportService;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Tests\TestCase;

final class PublicGameCatalogTest extends TestCase
{
    use RefreshDatabase;

    public function test_public_catalog_reads_only_active_projected_items_creatures_and_loot(): void
    {
        $this->activatePublicProfile('15.20');

        $this->get('/en/wiki/catalog')
            ->assertOk()
            ->assertSee('1 visible items')
            ->assertSee('1 visible creatures');

        $this->get('/en/wiki/items')
            ->assertOk()
            ->assertSee('Fixture Sword')
            ->assertDontSee('Future Fixture Shield')
            ->assertDontSee('catalog/fixtures/minimal-snapshot.json')
            ->assertDontSee('validation_summary');

        $this->get('/en/wiki/items/fixture-sword')
            ->assertOk()
            ->assertSee('Fixture Sword')
            ->assertSee('Fixture Rat')
            ->assertSee('1 / 10')
            ->assertDontSee('1 / 20')
            ->assertDontSee('catalog/fixtures/minimal-snapshot.json');

        $this->get('/en/wiki/creatures')
            ->assertOk()
            ->assertSee('Fixture Rat')
            ->assertDontSee('Partial Fixture Beast');

        $this->get('/en/wiki/creatures/fixture-rat')
            ->assertOk()
            ->assertSee('Fixture Rat')
            ->assertSee('Fixture Sword')
            ->assertSee('1 / 10')
            ->assertDontSee('1 / 20');

        $this->get('/en/wiki/items/future-fixture-shield')->assertNotFound();
        $this->get('/en/wiki/creatures/partial-fixture-beast')->assertNotFound();
    }

    public function test_public_filters_are_bounded_and_use_visible_records_only(): void
    {
        $this->activatePublicProfile('15.20');

        $this->get('/en/wiki/items?category=weapons&weapon_type=sword')
            ->assertOk()
            ->assertSee('Fixture Sword');
        $this->get('/en/wiki/items?weapon_type=shield')
            ->assertOk()
            ->assertDontSee('Future Fixture Shield')
            ->assertSee('No visible items match these filters.');
        $this->get('/en/wiki/creatures?bestiary_class=fixture')
            ->assertOk()
            ->assertSee('Fixture Rat');
        $this->get('/en/wiki/items?q='.str_repeat('a', 81))->assertUnprocessable();
        $this->get('/en/wiki/creatures?bestiary_class[]=fixture')->assertUnprocessable();
    }

    public function test_approved_translation_controls_public_name_and_slug_but_stale_translation_does_not(): void
    {
        $this->activatePublicProfile('15.20');
        $entityId = $this->databaseInt(DB::table('game_catalog_entities')->where('canonical_key', 'item:fixture-sword')->value('id'));
        $now = CarbonImmutable::now('UTC');

        DB::table('game_catalog_entity_translations')->insert([
            'entity_id' => $entityId,
            'locale' => 'pl',
            'display_name' => 'Miecz testowy',
            'slug' => 'miecz-testowy',
            'summary' => 'Zatwierdzone tłumaczenie testowe.',
            'description_markdown' => null,
            'source_name_sha256' => hash('sha256', 'Fixture Sword'),
            'translation_status' => 'approved',
            'created_at' => $now,
            'updated_at' => $now,
        ]);

        $this->get('/pl/wiki/items')
            ->assertOk()
            ->assertSee('Miecz testowy')
            ->assertDontSee('Fixture Sword');
        $this->get('/pl/wiki/items/miecz-testowy')
            ->assertOk()
            ->assertSee('Miecz testowy');

        DB::table('game_catalog_entity_translations')
            ->where('entity_id', $entityId)
            ->where('locale', 'pl')
            ->update(['translation_status' => 'stale']);

        $this->get('/pl/wiki/items/miecz-testowy')->assertNotFound();
        $this->get('/pl/wiki/items/fixture-sword')
            ->assertOk()
            ->assertSee('Fixture Sword')
            ->assertDontSee('Miecz testowy');
    }

    public function test_catalog_has_a_safe_empty_state_without_an_active_public_profile(): void
    {
        $this->get('/en/wiki/catalog')
            ->assertOk()
            ->assertSee('The public catalog is not active yet.');
        $this->get('/en/wiki/items')
            ->assertOk()
            ->assertSee('The public catalog is not active yet.');
        $this->get('/en/wiki/items/fixture-sword')->assertNotFound();
    }

    private function activatePublicProfile(string $releaseKey): void
    {
        $snapshot = app(CatalogImportService::class)->import($this->fixturePath());
        $now = CarbonImmutable::now('UTC');

        DB::table('game_catalog_profiles')->insert([
            'key' => 'public',
            'name' => 'Public Game Catalog',
            'target_release_id' => DB::table('game_catalog_releases')->where('key', $releaseKey)->value('id'),
            'active_snapshot_id' => null,
            'protocol_profile' => 'fixture-protocol',
            'complete_only' => true,
            'completeness_policy_key' => 'complete-only',
            'availability_policy_key' => 'public-proven',
            'validation_policy_key' => 'validated-snapshot',
            'public_enabled' => true,
            'allow_backports' => false,
            'lock_version' => 0,
            'created_at' => $now,
            'updated_at' => $now,
        ]);

        app(CatalogActivationService::class)->activate($snapshot->snapshotId, 'public');
    }

    private function fixturePath(): string
    {
        return base_path('tests/Fixtures/GameCatalog/v1/minimal-snapshot.json');
    }

    private function databaseInt(mixed $value): int
    {
        if (is_int($value)) {
            return $value;
        }
        if (is_string($value) && preg_match('/^(?:0|[1-9][0-9]*)$/D', $value) === 1) {
            return (int) $value;
        }

        self::fail('Expected an integer database value.');
    }
}
