<?php

namespace App\Identity\Sessions;

use App\Identity\Models\Identity;
use Illuminate\Contracts\Auth\Authenticatable;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Auth;

final class IdentityWebSessionManager
{
    public function __construct(
        private readonly ?IdentityWebSessionRegistry $registry = null,
    ) {}

    public function login(Identity $identity): void
    {
        Auth::login($identity, false);
    }

    public function user(): ?Authenticatable
    {
        return Auth::user();
    }

    public function establish(Request $request, Identity $identity): void
    {
        $request->session()->regenerate();
        $identity->refresh();
        $registered = $this->registry()->establish($request, $identity);

        $request->session()->put(WebSessionState::GENERATION_KEY, $identity->web_session_generation);
        $request->session()->put(WebSessionState::REGISTRY_ID_KEY, $registered->id);
    }

    public function invalidate(Request $request): void
    {
        $authenticated = Auth::user();
        $this->registry()->revokeCurrent($request, $authenticated instanceof Identity ? $authenticated : null);
        Auth::logout();
        $request->session()->invalidate();
        $request->session()->regenerateToken();
    }

    private function registry(): IdentityWebSessionRegistry
    {
        return $this->registry ?? app(IdentityWebSessionRegistry::class);
    }
}
