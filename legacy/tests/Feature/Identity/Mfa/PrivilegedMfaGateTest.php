<?php

namespace Tests\Feature\Identity\Mfa;

use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Route;
use Tests\TestCase;

final class PrivilegedMfaGateTest extends TestCase
{
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();

        Route::middleware(['web', 'auth', 'mfa.confirmed'])
            ->get('/_test/privileged-mfa', static fn (): string => 'allowed');
    }

    public function test_authenticated_identity_without_confirmed_mfa_is_forbidden(): void
    {
        $identity = $this->createIdentity();

        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => 0]);

        $this->get('/_test/privileged-mfa')->assertForbidden();
    }

    public function test_authenticated_identity_with_confirmed_mfa_passes_the_mfa_gate(): void
    {
        $identity = $this->createIdentity();
        $identity->forceFill([
            'two_factor_secret' => 'TEST-MFA-SECRET-NOT-REAL',
            'two_factor_confirmed_at' => now(),
        ])->save();

        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => 0]);

        $this->get('/_test/privileged-mfa')
            ->assertOk()
            ->assertSeeText('allowed');
    }

    private function createIdentity(): Identity
    {
        return Identity::query()->create([
            'email' => 'person@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }
}
