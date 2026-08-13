<?php

namespace Tests\Feature\PlayerCompanion;

use App\Identity\Models\Identity;
use App\PlayerCompanion\Models\SessionAnalysis;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;

final class SessionAnalysisFeatureTest extends TestCase
{
    use RefreshDatabase;

    public function test_analyzer_requires_authentication_and_renders_private_no_store_surface(): void
    {
        $this->get(route('player-companion.session-analyses.index'))
            ->assertRedirect(route('identity.login.create'));

        $identity = $this->identity('owner@example.com');
        $this->actingAs($identity);

        $this->get(route('player-companion.session-analyses.index'))
            ->assertOk()
            ->assertHeader('Cache-Control', 'private, no-store')
            ->assertSeeText('Hunt Session Analyzer')
            ->assertSeeText('No saved session analyses yet.');
    }

    public function test_valid_log_is_normalized_persisted_and_raw_text_is_not_stored(): void
    {
        $identity = $this->identity('save@example.com');
        $this->actingAs($identity);
        $raw = $this->sampleLog();

        $response = $this->post(route('player-companion.session-analyses.store'), [
            'label' => 'Duo test',
            'session_log' => $raw,
        ]);

        $analysis = SessionAnalysis::query()->sole();
        $response->assertRedirect(route('player-companion.session-analyses.show', $analysis->id));
        self::assertSame($identity->id, $analysis->identity_id);
        self::assertSame(400_000, $analysis->balance_value);
        self::assertSame(2, $analysis->participant_count);
        self::assertSame('Alice', $analysis->settlements[0]['from']);
        self::assertNotContains($raw, array_values($analysis->getAttributes()), true);
        self::assertArrayNotHasKey('session_log', $analysis->getAttributes());
        self::assertArrayNotHasKey('raw_log', $analysis->getAttributes());
    }

    public function test_invalid_log_does_not_persist_or_flash_raw_log_back_to_session(): void
    {
        $identity = $this->identity('invalid@example.com');
        $this->actingAs($identity);
        $raw = 'THIS PRIVATE RAW LOG SHOULD NOT BE FLASHED';

        $this->from(route('player-companion.session-analyses.index'))
            ->post(route('player-companion.session-analyses.store'), [
                'label' => 'Invalid test',
                'session_log' => $raw,
            ])
            ->assertRedirect(route('player-companion.session-analyses.index'))
            ->assertSessionHasErrors('session_log')
            ->assertSessionMissing('_old_input.session_log');

        self::assertSame(0, SessionAnalysis::query()->count());
    }

    public function test_analysis_is_owner_private_and_owner_can_delete_it(): void
    {
        $owner = $this->identity('owner-private@example.com');
        $other = $this->identity('other@example.com');
        $analysis = SessionAnalysis::query()->create([
            'identity_id' => $owner->id,
            'label' => 'Private hunt',
            'source_format' => 'tibia-session-text-v1',
            'parser_version' => '1.0.0',
            'formula_version' => 'equal-split-v1',
            'session_seconds' => 3600,
            'experience_gain' => 100,
            'loot_value' => 100,
            'supplies_value' => 50,
            'balance_value' => 50,
            'damage' => null,
            'healing' => null,
            'experience_per_hour' => 100,
            'profit_per_hour' => 50,
            'participant_count' => 0,
            'participants' => [],
            'settlements' => [],
        ]);

        $this->actingAs($other);
        $this->get(route('player-companion.session-analyses.show', $analysis->id))->assertNotFound();
        $this->delete(route('player-companion.session-analyses.destroy', $analysis->id))->assertNotFound();

        $this->actingAs($owner);
        $this->get(route('player-companion.session-analyses.show', $analysis->id))
            ->assertOk()
            ->assertHeader('Cache-Control', 'private, no-store')
            ->assertSeeText('Private hunt');
        $this->delete(route('player-companion.session-analyses.destroy', $analysis->id))
            ->assertRedirect(route('player-companion.session-analyses.index'));

        $this->assertDatabaseMissing('player_companion_session_analyses', ['id' => $analysis->id]);
    }

    public function test_polish_locale_renders_localized_analyzer_copy(): void
    {
        $identity = $this->identity('pl@example.com');
        $this->actingAs($identity);

        $this->get(route('player-companion.session-analyses.index', ['locale' => 'pl']))
            ->assertOk()
            ->assertSeeText('Analizator sesji polowania');
    }

    private function identity(string $email): Identity
    {
        return Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('testing-password'),
        ]);
    }

    private function sampleLog(): string
    {
        return <<<'LOG'
Session: 01:00h
XP Gain: 3,600,000
Loot: 600,000
Supplies: 200,000
Balance: 400,000
Alice
Loot: 400,000
Supplies: 100,000
Balance: 300,000
Bob
Loot: 200,000
Supplies: 100,000
Balance: 100,000
LOG;
    }
}
