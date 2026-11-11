<?php

namespace App\Http\Controllers\Identity;

use App\Http\Requests\Identity\ResetPasswordRequest;
use App\Identity\Credentials\PasswordResetCompleter;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Password;

final class PasswordResetController
{
    public function create(Request $request, string $token): View
    {
        $email = $request->query('email');

        return view('identity.reset-password', [
            'token' => $token,
            'email' => is_string($email) ? $email : '',
        ]);
    }

    public function store(
        ResetPasswordRequest $request,
        PasswordResetCompleter $passwordReset,
    ): RedirectResponse {
        $status = $passwordReset->complete($request->credentials());

        if ($status === Password::PASSWORD_RESET) {
            return redirect()
                ->route('identity.login.create')
                ->with('status', 'Your password has been reset. Sign in again.');
        }

        return back()
            ->withInput($request->only('email'))
            ->withErrors([
                'email' => 'This password reset link is invalid or expired.',
            ]);
    }
}
