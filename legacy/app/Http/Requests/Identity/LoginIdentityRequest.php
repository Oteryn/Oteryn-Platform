<?php

namespace App\Http\Requests\Identity;

use App\Identity\Models\Identity;
use App\Identity\Support\CanonicalEmail;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Support\Facades\Hash;
use Illuminate\Validation\ValidationException;

final class LoginIdentityRequest extends FormRequest
{
    /**
     * Non-secret Argon2id hash used only to equalize password-verification work
     * when no Identity row exists. It is not a credential for any account.
     */
    private const DUMMY_PASSWORD_HASH = '$argon2id$v=19$m=19456,t=2,p=1$M0Iyek83amFsS1J0cWNnLg$5wDTmQ1PJJorXDWlZfJnbjZkEATuPz4DMxaPpZ9azV0';

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
            'password' => [
                'required',
                'string',
                'max:1024',
            ],
        ];
    }

    public function authenticate(): Identity
    {
        $credentials = $this->credentials();
        $identity = Identity::query()
            ->where('email', $credentials['email'])
            ->first();

        if ($identity === null) {
            Hash::check($credentials['password'], self::DUMMY_PASSWORD_HASH);
            $this->failAuthentication();
        }

        $passwordIsValid = Hash::check($credentials['password'], $identity->password);

        if (! $passwordIsValid || $identity->disabled_at !== null) {
            $this->failAuthentication();
        }

        return $identity;
    }

    /**
     * @return array{email: string, password: string}
     */
    private function credentials(): array
    {
        $validated = $this->validated();
        $email = $validated['email'] ?? null;
        $password = $validated['password'] ?? null;

        if (! is_string($email) || ! is_string($password)) {
            $this->failAuthentication();
        }

        return [
            'email' => $email,
            'password' => $password,
        ];
    }

    /**
     * @throws ValidationException
     */
    private function failAuthentication(): never
    {
        throw ValidationException::withMessages([
            'email' => 'The provided credentials are invalid.',
        ]);
    }
}
