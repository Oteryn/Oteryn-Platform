<?php

namespace Tests\Feature\Characters;

use App\CanaryIntegration\CanaryCharacterCreator;
use App\Characters\Contracts\CanaryCharacterCreationGateway;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;

final class CharacterCreationTest extends TestCase
{
    use RefreshDatabase;

    public function test_guest_cannot_open_or_submit_character_creation(): void
    {
        $this->get('/account/characters/create')->assertRedirect('/login');
        $this->post('/account/characters', [
            'name' => 'Alice Moon',
            'vocation' => 9,
            'sex' => 1,
        ])->assertRedirect('/login');
    }

    public function test_authenticated_form_is_available_and_contains_only_product_inputs(): void
    {
        $this->loginAsCurrentIdentity($this->identity());

        $response = $this->get('/account/characters/create');

        $response->assertOk();
        $response->assertSee('Create a character');
        $response->assertSee('name="_token"', false);
        $response->assertSee('name="name"', false);
        $response->assertSee('name="vocation"', false);
        $response->assertSee('name="sex"', false);
        $response->assertDontSee('name="account_id"', false);
    }

    public function test_valid_submission_returns_unavailable_without_any_write_or_game_connection(): void
    {
        $this->loginAsCurrentIdentity($this->identity());

        $response = $this->post('/account/characters', [
            'name' => '  aLiCe   moon  ',
            'vocation' => 9,
            'sex' => 1,
            'account_id' => 999999,
            'level' => 999,
        ]);

        $response->assertRedirect();
        $response->assertSessionHasErrors(['character' => 'Character creation is temporarily unavailable.']);
        $response->assertSessionMissing('status');
        self::assertFalse($this->app->bound(CanaryCharacterCreationGateway::class));
        self::assertArrayNotHasKey(CanaryCharacterCreator::CONNECTION, DB::getConnections());
    }

    public function test_invalid_name_is_rejected_before_unavailable_state(): void
    {
        $this->loginAsCurrentIdentity($this->identity());

        $this->post('/account/characters', [
            'name' => 'OterynSupport',
            'vocation' => 1,
            'sex' => 0,
        ])->assertSessionHasErrors('name');
    }

    public function test_invalid_vocation_is_rejected_by_request_validation(): void
    {
        $this->loginAsCurrentIdentity($this->identity());

        $this->post('/account/characters', [
            'name' => 'Alice Moon',
            'vocation' => 10,
            'sex' => 0,
        ])->assertSessionHasErrors('vocation');
    }

    private function identity(): Identity
    {
        return Identity::query()->create([
            'email' => uniqid('character-', true).'@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }

    private function loginAsCurrentIdentity(Identity $identity): void
    {
        $this->post('/login', [
            'email' => $identity->email,
            'password' => 'Correct-Horse-9!Battery',
        ])->assertRedirect(route('home'));

        $this->assertAuthenticatedAs($identity, 'web');
    }
}
