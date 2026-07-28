<?php

namespace App\Http\Requests\Identity;

use App\Identity\Support\CanonicalEmail;
use App\Identity\Support\IdentityPasswordPolicy;
use Illuminate\Foundation\Http\FormRequest;

final class RecoverWithIdentityKeyRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    /** @return array<string, list<mixed>> */
    public function rules(): array
    {
        return [
            'email' => ['required', 'string', 'email:rfc', 'max:254'],
            'recovery_key' => ['required', 'string', 'max:160'],
            'password' => ['required', 'confirmed', IdentityPasswordPolicy::rule()],
        ];
    }

    public function canonicalEmail(): string
    {
        return CanonicalEmail::normalize((string) $this->validated('email'));
    }

    public function recoveryKey(): string
    {
        return (string) $this->validated('recovery_key');
    }

    public function newPassword(): string
    {
        return (string) $this->validated('password');
    }
}
