<?php

namespace App\Http\Requests\Identity;

use App\Identity\Support\CanonicalEmail;
use App\Identity\Support\IdentityPasswordPolicy;
use Illuminate\Foundation\Http\FormRequest;

final class ResetPasswordRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    protected function prepareForValidation(): void
    {
        $email = $this->input('email');

        if (is_string($email)) {
            $this->merge([
                'email' => CanonicalEmail::normalize($email),
            ]);
        }
    }

    /**
     * @return array<string, array<int, mixed>>
     */
    public function rules(): array
    {
        return [
            'email' => [
                'required',
                'string',
                'max:254',
                'email:rfc',
            ],
            'token' => [
                'required',
                'string',
                'max:512',
            ],
            'password' => [
                'required',
                'string',
                'max:1024',
                'confirmed',
                IdentityPasswordPolicy::rule(),
            ],
        ];
    }

    /**
     * @return array{email: string, token: string, password: string}
     */
    public function credentials(): array
    {
        $validated = $this->validated();
        $email = $validated['email'] ?? null;
        $token = $validated['token'] ?? null;
        $password = $validated['password'] ?? null;

        abort_unless(
            is_string($email)
                && is_string($token)
                && is_string($password),
            422,
        );

        return [
            'email' => $email,
            'token' => $token,
            'password' => $password,
        ];
    }
}
