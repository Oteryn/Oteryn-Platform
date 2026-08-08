<?php

namespace Tests\Feature\HomepageTemplates;

use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Tests\TestCase;

final class HomepageTemplateMigrationTest extends TestCase
{
    use RefreshDatabase;

    public function test_migration_seeds_the_existing_production_home_without_activating_an_alternative(): void
    {
        $this->assertDatabaseHas('homepage_template_settings', [
            'id' => 1,
            'active_key' => 'production',
            'previous_key' => null,
            'version' => 0,
            'updated_by_identity_id' => null,
        ]);

        self::assertSame(1, DB::table('homepage_template_settings')->count());
    }
}
