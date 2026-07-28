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

    /** @return array<string, string> */
    public function messages(): array
    {
        return [
            'email.required' => __('identity.validation.email_required'),
            'email.string' => __('identity.validation.email_invalid'),
            'email.email' => __('identity.validation.email_invalid'),
            'email.max' => __('identity.validation.email_too_long'),
            'recovery_key.required' => __('identity.validation.recovery_key_required'),
            'recovery_key.string' => __('identity.validation.recovery_key_required'),
            'recovery_key.max' => __('identity.validation.recovery_key_too_long'),
            'password.required' => __('identity.validation.new_password_required'),
            'password.confirmed' => __('identity.validation.new_password_confirmation'),
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
