<?php

namespace App\Http\Requests\Identity;

use App\Identity\Support\CanonicalEmail;
use Illuminate\Foundation\Http\FormRequest;

final class ForgotPasswordRequest extends FormRequest
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
        ];
    }

    public function email(): string
    {
        $email = $this->validated('email');

        abort_unless(is_string($email), 422);

        return $email;
    }
}
