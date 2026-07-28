<?php

namespace App\Http\Requests\Identity;

use App\Identity\Support\CanonicalEmail;
use App\Identity\Support\IdentityPasswordPolicy;
use Illuminate\Foundation\Http\FormRequest;
use LogicException;

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
        return CanonicalEmail::normalize($this->validatedString('email'));
    }

    public function recoveryKey(): string
    {
        return $this->validatedString('recovery_key');
    }

    public function newPassword(): string
    {
        return $this->validatedString('password');
    }

    private function validatedString(string $key): string
    {
        $value = $this->validated($key);
        if (! is_string($value)) {
            throw new LogicException("Validated identity input {$key} is not a string.");
        }

        return $value;
    }
}
