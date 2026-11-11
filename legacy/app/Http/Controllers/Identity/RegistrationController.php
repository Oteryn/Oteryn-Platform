<?php

namespace App\Http\Controllers\Identity;

use App\Http\Requests\Identity\RegisterIdentityRequest;
use App\Identity\Actions\RegisterIdentity;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;

final class RegistrationController
{
    public function create(): View
    {
        return view('identity.register');
    }

    public function store(RegisterIdentityRequest $request, RegisterIdentity $registerIdentity): RedirectResponse
    {
        $email = $request->validated('email');
        $password = $request->validated('password');

        if (! is_string($email) || ! is_string($password)) {
            abort(422);
        }

        $registerIdentity->execute($email, $password);

        return redirect()
            ->route('identity.register.create')
            ->with('status', 'Registration completed.');
    }
}
