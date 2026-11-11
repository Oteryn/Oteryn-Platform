<?php

namespace App\Http\Controllers\Identity;

use App\Http\Requests\Identity\ForgotPasswordRequest;
use App\Identity\Credentials\PasswordResetLinkSender;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;

final class PasswordRecoveryController
{
    public function create(): View
    {
        return view('identity.forgot-password');
    }

    public function store(
        ForgotPasswordRequest $request,
        PasswordResetLinkSender $resetLinks,
    ): RedirectResponse {
        $resetLinks->send($request->email());

        return back()->with(
            'status',
            'If an account exists for that email, a password reset link has been sent.',
        );
    }
}
