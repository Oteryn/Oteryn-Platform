<?php

namespace Tests\Unit\Identity\Sessions;

use App\Identity\Models\Identity;
use App\Identity\Models\IdentityWebSession;
use App\Identity\Sessions\IdentityWebSessionManager;
use App\Identity\Sessions\WebSessionState;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Http\Request;
use Illuminate\Session\ArraySessionHandler;
use Illuminate\Session\Store;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;

final class IdentityWebSessionManagerTest extends TestCase
{
    use RefreshDatabase;

    public function test_establish_regenerates_session_identifier_and_stores_registered_generation(): void
    {
        $session = new Store('oteryn-test', new ArraySessionHandler(120));
        $session->start();
        $request = Request::create('/login', 'POST');
        $request->setLaravelSession($session);
        $identity = Identity::query()->create([
            'email' => 'session-manager@example.test',
            'password' => Hash::make('Correct-password-123!'),
        ]);
        $identity->forceFill(['web_session_generation' => 7])->save();
        $previousSessionId = $session->getId();

        app(IdentityWebSessionManager::class)->establish($request, $identity);

        self::assertNotSame($previousSessionId, $session->getId());
        self::assertSame(7, $session->get(WebSessionState::GENERATION_KEY));
        $registeredSessionId = $session->get(WebSessionState::REGISTRY_ID_KEY);
        self::assertIsString($registeredSessionId);
        $this->assertDatabaseHas('identity_web_sessions', [
            'id' => $registeredSessionId,
            'identity_id' => $identity->id,
            'generation' => 7,
            'revoked_at' => null,
        ]);
        self::assertTrue(IdentityWebSession::query()->findOrFail($registeredSessionId)->isActive());
    }
}
